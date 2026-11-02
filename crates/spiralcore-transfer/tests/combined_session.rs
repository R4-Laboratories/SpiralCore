use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Context, Endpoint},
    hex, object_id,
    protocol::{Frame, Mode, Offer},
    schema,
    session::{CombinedSession, MAX_SESSION_CHARGE},
    shared_runtime::{OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
    wan::{self, BenchAuthority},
    Error, Id, Result,
};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Store(std::path::PathBuf);
impl Store {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-combined-{}-{}",
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
    fn path(&self, c: &Context, id: Id) -> std::path::PathBuf {
        self.0
            .join(hex(&c.namespace().unwrap()))
            .join(format!("{}.object", hex(&id)))
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn artifact() -> Vec<u8> {
    schema::fit(&[b"{\"value\":0}\n"]).unwrap()
}
fn prediction() -> PreparedPrediction {
    let checkpoint = State::seeded(4096).unwrap().encode().unwrap();
    PreparedPrediction::new(
        &checkpoint,
        &Recipe {
            steps: 81,
            payload: 1.25,
            output: OutputKind::Trace,
        },
    )
    .unwrap()
}
fn random(len: usize, mut seed: u64) -> Vec<u8> {
    (0..len)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u8
        })
        .collect()
}
fn edit(base: &[u8]) -> Vec<u8> {
    let mut out = base.to_vec();
    let i = out.len() / 2;
    for b in &mut out[i..i + 32] {
        *b ^= 0xa5;
    }
    out
}
fn exchange(
    endpoint: &mut Endpoint<BenchAuthority>,
    c: &Context,
    frames: &[Frame],
) -> Result<Vec<Frame>> {
    let mut replies = Vec::new();
    for f in frames {
        let r = endpoint
            .handle(c, f, 0)
            .unwrap_or_else(|e| f.reply(127, (e as u16).to_le_bytes().to_vec()));
        let stop = matches!(r.kind, 103 | 127);
        replies.push(r);
        if stop {
            break;
        }
    }
    Ok(replies)
}
#[test]
fn one_session_combines_runtime_and_rolling_sparse_without_raw_seed() {
    let c = wan::context([41; 32]);
    let store = Store::new();
    let mut endpoint = store.open();
    let prediction = prediction();
    let checkpoint = object_id(prediction.checkpoint());
    let generated = prediction.prediction().to_vec();
    let mut session = CombinedSession::new(&c, prediction, &artifact()).unwrap();
    session
        .seed_checkpoint(&c, [1; 32], |f| exchange(&mut endpoint, &c, f))
        .unwrap();
    assert_eq!(session.base_id(), None);
    let opaque = random(generated.len(), 77);
    let targets = [
        generated.clone(),
        edit(&generated),
        opaque.clone(),
        edit(&opaque),
        [edit(&opaque).as_slice(), b"\0append\0"].concat(),
    ];
    let mut previous = None;
    for (i, target) in targets.into_iter().enumerate() {
        let original = target.clone();
        let pointer = target.as_ptr();
        let mut begin = None;
        let sent = session
            .send(&c, target, [10 + i as u8; 32], true, |frames| {
                if begin.is_none() {
                    begin = Some(Offer::decode(&frames[0].body)?);
                }
                exchange(&mut endpoint, &c, frames)
            })
            .unwrap();
        let offer = begin.unwrap();
        assert_eq!(offer.object, object_id(&original));
        assert_eq!(sent.transfer.object, offer.object);
        assert_eq!(session.base_id(), Some(offer.object));
        assert_eq!(session.base_bytes().unwrap().as_ptr(), pointer);
        assert!(session.base_ready() && sent.selection.checkpoint_ready);
        assert!(!sent.selection.schema_ready && !sent.selection.exact_repeat_probe);
        assert!(sent
            .selection
            .candidates
            .iter()
            .all(|c| c.mode != Mode::SchemaZstdV1));
        let selected = sent
            .selection
            .candidates
            .iter()
            .find(|c| c.mode == sent.selection.selected_mode)
            .unwrap();
        assert!(sent
            .selection
            .candidates
            .iter()
            .all(|c| selected.total_http_body_bytes <= c.total_http_body_bytes));
        match offer.mode {
            Mode::RuntimeV1 => assert_eq!(offer.base, Some(checkpoint)),
            Mode::SparseV1 => assert_eq!(offer.base, previous),
            Mode::Raw | Mode::ZstdWholeV1 => assert_eq!(offer.base, None),
            other => panic!("unseeded unexpected mode {other:?}"),
        }
        if i == 0 {
            assert_eq!(offer.mode, Mode::RuntimeV1);
        }
        if i == 3 {
            assert_eq!(offer.mode, Mode::SparseV1);
        }
        assert_eq!(endpoint.read_object(&c, offer.object).unwrap(), original);
        previous = Some(offer.object);
        assert!(session.retained_charge() <= MAX_SESSION_CHARGE);
    }
    session
        .seed_schema(&c, [30; 32], |f| exchange(&mut endpoint, &c, f))
        .unwrap();
    assert_eq!(session.base_id(), previous);
    assert!(session.schema_ready());
    session
        .seed_checkpoint(&c, [31; 32], |f| exchange(&mut endpoint, &c, f))
        .unwrap();
    assert_eq!(session.base_id(), previous);
    let target = random(131_073, 91);
    let sent = session
        .send(&c, target.clone(), [32; 32], true, |f| {
            exchange(&mut endpoint, &c, f)
        })
        .unwrap();
    assert!(sent.selection.schema_ready);
    assert!(
        sent.selection
            .candidates
            .iter()
            .any(|c| c.mode == Mode::SchemaZstdV1)
            || sent
                .selection
                .pruned
                .iter()
                .any(|c| c.mode == Mode::SchemaZstdV1)
    );
    drop(endpoint);
    assert_eq!(
        store.open().read_object(&c, sent.transfer.object).unwrap(),
        target
    );
}
#[test]
fn exact_repeat_probes_before_codecs_and_recovers_eviction_under_same_offer() {
    for evict in [false, true] {
        let c = wan::context([42; 32]);
        let store = Store::new();
        let mut endpoint = store.open();
        let prediction = prediction();
        let target = prediction.prediction().to_vec();
        let id = object_id(&target);
        let mut session = CombinedSession::new(&c, prediction, &artifact()).unwrap();
        session
            .seed_checkpoint(&c, [1; 32], |f| exchange(&mut endpoint, &c, f))
            .unwrap();
        session
            .send(&c, target.clone(), [2; 32], true, |f| {
                exchange(&mut endpoint, &c, f)
            })
            .unwrap();
        if evict {
            std::fs::remove_file(store.path(&c, id)).unwrap();
        }
        let mut requests = Vec::new();
        let sent = session
            .send(&c, target.clone(), [3; 32], false, |frames| {
                if requests.is_empty() {
                    assert_eq!(frames.len(), 1);
                    let offer = Offer::decode(&frames[0].body)?;
                    assert_eq!(offer.mode, Mode::Raw);
                    assert_eq!(offer.base, None);
                    assert_eq!(offer.object, id);
                }
                requests.extend_from_slice(frames);
                exchange(&mut endpoint, &c, frames)
            })
            .unwrap();
        assert!(sent.selection.exact_repeat_probe);
        assert_eq!(sent.selection.selected_mode, Mode::Raw);
        assert_eq!(sent.selection.candidates.len(), 1);
        assert!(sent.selection.pruned.is_empty());
        assert_eq!(requests.iter().filter(|f| f.kind == 1).count(), 1);
        assert!(requests.iter().all(|f| f.transfer == [3; 32]));
        if evict {
            assert_eq!(sent.transfer.data_bytes, target.len() as u64);
        } else {
            assert_eq!(requests.len(), 1);
            assert_eq!(sent.transfer.data_bytes, 0);
        }
        assert_eq!(endpoint.read_object(&c, id).unwrap(), target);
    }
}
#[test]
fn readiness_failure_scope_and_capacity_never_promote_unverified_bytes() {
    let c = wan::context([43; 32]);
    let store = Store::new();
    let mut endpoint = store.open();
    let prediction = prediction();
    let target = prediction.prediction().to_vec();
    let mut session = CombinedSession::new(&c, prediction, &artifact()).unwrap();
    let mut calls = 0;
    assert!(matches!(
        session.send(&c, target.clone(), [1; 32], true, |_| {
            calls += 1;
            Err(Error::Storage)
        }),
        Err(Error::Missing)
    ));
    assert_eq!(calls, 0);
    session
        .seed_checkpoint(&c, [2; 32], |f| exchange(&mut endpoint, &c, f))
        .unwrap();
    session
        .seed_schema(&c, [3; 32], |f| exchange(&mut endpoint, &c, f))
        .unwrap();
    session
        .send(&c, target.clone(), [4; 32], true, |f| {
            exchange(&mut endpoint, &c, f)
        })
        .unwrap();
    let old = session.base_id();
    let mut excessive = Vec::with_capacity(client::MAX_BATCHED_OBJECT + 1);
    excessive.push(1);
    assert!(matches!(
        session.send(&c, excessive, [5; 32], true, |_| {
            calls += 1;
            Err(Error::Storage)
        }),
        Err(Error::Limit)
    ));
    assert_eq!(calls, 0);
    assert_eq!(session.base_id(), old);
    assert!(session.base_ready() && session.checkpoint_ready() && session.schema_ready());
    let edited = edit(&target);
    assert!(matches!(
        session.send(&c, edited.clone(), [6; 32], true, |f| {
            let mut reply = exchange(&mut endpoint, &c, f)?;
            if let Some(ack) = reply.iter_mut().find(|r| r.kind == 103) {
                ack.body[0] ^= 1;
            }
            Ok(reply)
        }),
        Err(Error::Integrity)
    ));
    assert_eq!(session.base_id(), old);
    assert!(!session.base_ready() && !session.checkpoint_ready() && !session.schema_ready());
    assert_eq!(
        endpoint.read_object(&c, object_id(&edited)).unwrap(),
        edited,
        "remote commit does not excuse invalid ACK"
    );
    let mut changed = c.clone();
    changed.policy_epoch += 1;
    assert!(matches!(
        session.seed_checkpoint(&changed, [7; 32], |_| {
            calls += 1;
            Err(Error::Storage)
        }),
        Err(Error::Unauthorized)
    ));
    assert_eq!(session.base_id(), None);
    assert_eq!(session.retained_bytes(), 0);
    assert!(matches!(
        session.seed_checkpoint(&c, [8; 32], |_| {
            calls += 1;
            Err(Error::Storage)
        }),
        Err(Error::Unauthorized)
    ));
    assert_eq!(calls, 0);
}

#[test]
fn evicted_checkpoint_or_rolling_base_fails_without_reoffer() {
    for missing_checkpoint in [true, false] {
        let c = wan::context([44; 32]);
        let store = Store::new();
        let mut endpoint = store.open();
        let prediction = prediction();
        let checkpoint = object_id(prediction.checkpoint());
        let generated = prediction.prediction().to_vec();
        let mut session = CombinedSession::new(&c, prediction, &artifact()).unwrap();
        session
            .seed_checkpoint(&c, [1; 32], |f| exchange(&mut endpoint, &c, f))
            .unwrap();
        let (missing, target, mode) = if missing_checkpoint {
            (checkpoint, generated, Mode::RuntimeV1)
        } else {
            let base = random(131_073, 51);
            session
                .send(&c, base.clone(), [2; 32], true, |f| {
                    exchange(&mut endpoint, &c, f)
                })
                .unwrap();
            (object_id(&base), edit(&base), Mode::SparseV1)
        };
        let old = session.base_id();
        std::fs::remove_file(store.path(&c, missing)).unwrap();
        let id = object_id(&target);
        let mut calls = 0;
        assert!(matches!(
            session.send(&c, target, [3; 32], true, |f| {
                calls += 1;
                assert_eq!(Offer::decode(&f[0].body)?.mode, mode);
                exchange(&mut endpoint, &c, f)
            }),
            Err(Error::Missing)
        ));
        assert_eq!(calls, 1);
        assert_eq!(session.base_id(), old);
        assert!(!session.base_ready() && !session.checkpoint_ready() && !session.schema_ready());
        assert!(endpoint.read_object(&c, id).is_err());
    }
}

#[test]
fn full_size_target_is_admitted_and_transport_failure_does_not_publish() {
    let c = wan::context([45; 32]);
    let store = Store::new();
    let mut endpoint = store.open();
    let mut session = CombinedSession::new(&c, prediction(), &artifact()).unwrap();
    session
        .seed_checkpoint(&c, [1; 32], |f| exchange(&mut endpoint, &c, f))
        .unwrap();
    let mut calls = 0;
    assert!(matches!(
        session.send(
            &c,
            vec![0; client::MAX_BATCHED_OBJECT],
            [2; 32],
            false,
            |frames| {
                calls += 1;
                assert_eq!(
                    Offer::decode(&frames[0].body)?.len,
                    client::MAX_BATCHED_OBJECT as u64
                );
                Err(Error::Storage)
            }
        ),
        Err(Error::Storage)
    ));
    assert_eq!(calls, 1);
    assert_eq!(session.base_id(), None);
    assert!(!session.checkpoint_ready());
}
