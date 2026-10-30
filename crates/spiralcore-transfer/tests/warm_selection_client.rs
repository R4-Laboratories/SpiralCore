use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Endpoint},
    object_id,
    protocol::{Frame, Mode},
    schema,
    schema_selection::PreparedSelector,
    wan::{self, BenchAuthority},
    Error,
};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Store(std::path::PathBuf);
impl Store {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "warm-selection-client-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
    fn open(&self) -> Endpoint<BenchAuthority> {
        Endpoint::open_with_policy(
            &self.0,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap()
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn random(n: usize) -> Vec<u8> {
    let mut x = 0x1249_9faa_0071_ee63u64;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}
fn exchange(
    ep: &mut Endpoint<BenchAuthority>,
    scope: [u8; 32],
    frames: &[Frame],
    bytes: &mut u64,
) -> spiralcore_transfer::Result<Vec<Frame>> {
    *bytes += wan::encode_batch(frames)?.len() as u64;
    let mut replies = Vec::new();
    for f in frames {
        let r = ep
            .handle(&wan::context(scope), f, 0)
            .unwrap_or_else(|e| f.reply(127, (e as u16).to_le_bytes().to_vec()));
        let terminal = matches!(r.kind, 103 | 127);
        replies.push(r);
        if terminal {
            break;
        }
    }
    *bytes += wan::encode_batch(&replies)?.len() as u64 + 16;
    Ok(replies)
}

#[test]
fn selected_warm_plan_sends_exact_cost_without_reencoding_and_reopens() {
    let base = random(1 << 20);
    let mut changed = base.clone();
    for b in &mut changed[524288..524320] {
        *b ^= 0xa5;
    }
    let key: String = random(2048)
        .iter()
        .map(|x| char::from(b'a' + x % 26))
        .collect();
    let schema_target = format!("{{\"{key}\":7}}\n").into_bytes();
    let artifact = schema::fit(&[&schema_target]).unwrap();
    for (index, (target, supplied_base, schema_ready, wanted)) in [
        (b"tiny".to_vec(), None, false, Mode::Raw),
        (vec![0; 65537], None, false, Mode::ZstdWholeV1),
        (changed, Some(base.as_slice()), false, Mode::SparseV1),
        (schema_target, None, true, Mode::SchemaZstdV1),
    ]
    .into_iter()
    .enumerate()
    {
        let store = Store::new();
        let mut ep = store.open();
        let mut selector = PreparedSelector::new(&artifact).unwrap();
        let mut setup = 0;
        for bytes in supplied_base
            .into_iter()
            .chain(schema_ready.then_some(artifact.as_slice()))
        {
            client::transfer_batched(bytes, Mode::Raw, None, object_id(bytes), false, |f| {
                exchange(&mut ep, [1; 32], f, &mut setup)
            })
            .unwrap();
        }
        let plan = selector
            .prepare_warm(&target, supplied_base, schema_ready, false)
            .unwrap();
        let receipt = plan.selection().clone();
        assert_eq!(receipt.selected_mode, wanted);
        let expected = receipt
            .candidates
            .iter()
            .find(|c| c.mode == wanted)
            .unwrap()
            .target_http_body_bytes;
        // Prepared plan owns its data; dropping the encoder cannot change send behavior.
        drop(selector);
        let mut actual = 0;
        let stats = client::transfer_warm_selection_batched(plan, [40 + index as u8; 32], |f| {
            exchange(&mut ep, [1; 32], f, &mut actual)
        })
        .unwrap();
        assert_eq!(stats.mode, wanted);
        assert_eq!(actual, expected);
        if wanted == Mode::SparseV1 {
            assert_eq!(actual, 600);
        }
        drop(ep);
        assert_eq!(
            store
                .open()
                .read_object(&wan::context([1; 32]), object_id(&target))
                .unwrap(),
            target
        );
    }
}

#[test]
fn unavailable_selected_base_or_transport_failure_never_reoffers() {
    let base = random(65537);
    let mut target = base.clone();
    target[32768] ^= 1;
    let artifact = schema::fit(&[]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let store = Store::new();
    let mut ep = store.open();
    let mut bytes = 0;
    client::transfer_batched(&base, Mode::Raw, None, [1; 32], false, |f| {
        exchange(&mut ep, [1; 32], f, &mut bytes)
    })
    .unwrap();
    let plan = selector
        .prepare_warm(&target, Some(&base), false, false)
        .unwrap();
    assert_eq!(plan.selection().selected_mode, Mode::SparseV1);
    let mut calls = 0;
    let result = client::transfer_warm_selection_batched(plan, [2; 32], |f| {
        calls += 1;
        exchange(&mut ep, [2; 32], f, &mut bytes)
    });
    assert!(matches!(result, Err(Error::Missing)));
    assert_eq!(calls, 1);
    assert!(ep
        .read_object(&wan::context([2; 32]), object_id(&target))
        .is_err());
    let plan = selector
        .prepare_warm(&target, Some(&base), false, false)
        .unwrap();
    let mut calls = 0;
    let result = client::transfer_warm_selection_batched(plan, [3; 32], |_| {
        calls += 1;
        Err(Error::Storage)
    });
    assert!(matches!(result, Err(Error::Storage)));
    assert_eq!(calls, 1);
    drop(ep);
    assert!(store
        .open()
        .read_object(&wan::context([1; 32]), object_id(&target))
        .is_err());
}

#[test]
fn warm_hints_do_not_authorize_missing_schema_or_revoked_context() {
    let key: String = random(2048)
        .iter()
        .map(|x| char::from(b'a' + x % 26))
        .collect();
    let target = format!("{{\"{key}\":7}}\n").into_bytes();
    let artifact = schema::fit(&[&target]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let store = Store::new();
    let mut ep = store.open();
    let mut bytes = 0;
    let plan = selector.prepare_warm(&target, None, true, false).unwrap();
    assert_eq!(plan.selection().selected_mode, Mode::SchemaZstdV1);
    let mut calls = 0;
    let result = client::transfer_warm_selection_batched(plan, [80; 32], |f| {
        calls += 1;
        exchange(&mut ep, [1; 32], f, &mut bytes)
    });
    assert!(matches!(result, Err(Error::Missing)));
    assert_eq!(calls, 1);
    assert!(ep
        .read_object(&wan::context([1; 32]), object_id(&target))
        .is_err());

    let base = random(65537);
    let mut changed = base.clone();
    changed[32768] ^= 1;
    client::transfer_batched(&base, Mode::Raw, None, [81; 32], false, |f| {
        exchange(&mut ep, [1; 32], f, &mut bytes)
    })
    .unwrap();
    let plan = selector
        .prepare_warm(&changed, Some(&base), false, false)
        .unwrap();
    let mut revoked = wan::context([1; 32]);
    revoked.policy_epoch += 1;
    let mut calls = 0;
    let result = client::transfer_warm_selection_batched(plan, [82; 32], |f| {
        calls += 1;
        f.iter()
            .map(|frame| ep.handle(&revoked, frame, 0))
            .collect()
    });
    assert!(matches!(result, Err(Error::Unauthorized)));
    assert_eq!(calls, 1);
    drop(ep);
    assert!(store
        .open()
        .read_object(&wan::context([1; 32]), object_id(&changed))
        .is_err());
}
