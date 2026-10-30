use spiralcore_transfer::{
    client,
    endpoint::{Authority, ColdGraphPolicy, Context, Endpoint, Limits},
    object_id,
    protocol::{Frame, Mode, Offer},
    Result,
};
use spiralcore_transfer::{zstd_prefix as prefix, Error};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-prefix-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[derive(Clone)]
struct Allow(Arc<AtomicBool>);
impl Authority for Allow {
    fn authorize(&self, _: &Context) -> Result<()> {
        if self.0.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
fn allow() -> Allow {
    Allow(Arc::new(AtomicBool::new(true)))
}
fn context() -> Context {
    Context {
        tenant: "t".into(),
        domain: "d".into(),
        sender: "s".into(),
        recipient: "r".into(),
        mailbox: "m".into(),
        capability: "c".into(),
        authorization_scope: "objects".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}
fn frame(kind: u8, seq: u64, body: Vec<u8>) -> Frame {
    Frame {
        kind,
        seq,
        body,
        transfer: [7; 32],
    }
}
fn offer(base: &[u8], target: &[u8]) -> Vec<u8> {
    let o = Offer {
        mode: Mode::SparseV1,
        len: target.len() as u64,
        object: object_id(target),
        manifest: None,
        base: Some(object_id(base)),
    };
    let mut b = o.encode().unwrap();
    b[0] = 4;
    b
}
fn body(b: &[u8]) -> Vec<u8> {
    let mut out = vec![4];
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.push(1);
    out.extend_from_slice(b);
    out
}
fn fixture(n: usize) -> Vec<u8> {
    let mut x = 0x91d4731a7bcdef11u64;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

#[test]
fn whole_prefix_roundtrips_empty_edits_insertions_and_unrelated_data() {
    for size in [0, 1, 31, 4096, 65537] {
        let base = fixture(size);
        let mut edit = base.clone();
        if let Some(x) = edit.get_mut(size / 2) {
            *x ^= 0xff;
        }
        for target in [
            base.clone(),
            edit,
            [b"insert".as_slice(), &base].concat(),
            vec![7; size],
        ] {
            let encoded = prefix::encode(&base, &target).unwrap();
            assert_eq!(
                prefix::decode(&base, &encoded, target.len()).unwrap(),
                target
            );
        }
    }
    let base = fixture(1 << 20);
    let mut target = base.clone();
    target[55555] ^= 7;
    let encoded = prefix::encode(&base, &target).unwrap();
    assert!(encoded.len() < 4096);
    assert_eq!(
        prefix::decode(&base, &encoded, target.len()).unwrap(),
        target
    );
}

#[test]
fn rejects_wrong_base_damage_trailing_frames_and_unbounded_header() {
    let base = fixture(8192);
    let mut target = base.clone();
    target[777] ^= 7;
    let encoded = prefix::encode(&base, &target).unwrap();
    assert!(prefix::decode(&vec![0; 8192], &encoded, target.len()).is_err());
    for cut in [0, 4, 5, encoded.len() - 1] {
        assert!(prefix::decode(&base, &encoded[..cut], target.len()).is_err());
    }
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(prefix::decode(&base, &trailing, target.len()).is_err());
    let mut concat = encoded.clone();
    concat.extend_from_slice(&encoded);
    assert!(prefix::decode(&base, &concat, target.len()).is_err());
    let mut damaged = encoded.clone();
    *damaged.last_mut().unwrap() ^= 1;
    assert!(prefix::decode(&base, &damaged, target.len()).is_err());
    assert!(prefix::decode(&base, &encoded, target.len() - 1).is_err());
    assert_eq!(
        prefix::decode(&base, &encoded, prefix::MAX_PREFIX_OBJECT + 1),
        Err(Error::Limit)
    );
    // Ordinary magic followed by an explicit 1-GiB window must be rejected before allocation.
    let oversized = [0x28, 0xb5, 0x2f, 0xfd, 0x44, 160, 0, 0];
    assert_eq!(prefix::decode(&base, &oversized, 1), Err(Error::Limit));
    let mut flags = encoded.clone();
    flags[4] |= 8;
    assert!(prefix::decode(&base, &flags, target.len()).is_err());
    let mut no_checksum = encoded.clone();
    no_checksum[4] &= !4;
    assert!(prefix::decode(&base, &no_checksum, target.len()).is_err());
}

#[test]
fn endpoint_prefix_pins_authorized_base_and_publishes_only_verified_complete_bytes() {
    let t = Temp::new();
    let c = context();
    let base = fixture(65537);
    let target = [b"prefix".as_slice(), &base].concat();
    let authority = allow();
    let mut ep = Endpoint::open_with_policy(
        &t.0,
        Limits::default(),
        authority.clone(),
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    let begin = frame(1, 0, offer(&base, &target));
    let ready = ep.handle(&c, &begin, 1).unwrap();
    assert_eq!(ready.kind, 101);
    assert_eq!(ready.body[..2], [4, 1]);
    assert_eq!(ep.handle(&c, &begin, 1).unwrap(), ready);
    let encoded = prefix::encode(&base, &target).unwrap();
    let data = frame(2, 1, body(&encoded));
    let accepted = ep.handle(&c, &data, 2).unwrap();
    assert_eq!(accepted.body, (encoded.len() as u64).to_le_bytes());
    assert_eq!(ep.handle(&c, &data, 2).unwrap(), accepted);
    assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
    authority.0.store(false, Ordering::SeqCst);
    assert_eq!(
        ep.handle(&c, &frame(3, 2, vec![]), 3),
        Err(Error::Unauthorized)
    );
    authority.0.store(true, Ordering::SeqCst);
    assert_eq!(ep.handle(&c, &frame(3, 2, vec![]), 3).unwrap().kind, 103);
    assert_eq!(ep.graph_cache_stats().entries, 0);
    drop(ep);
    let mut ep = Endpoint::open_with_policy(
        &t.0,
        Limits::default(),
        authority,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    assert_eq!(ep.read_object(&c, object_id(&target)).unwrap(), target);
    let mut other = c.clone();
    other.policy_epoch += 1;
    assert_eq!(ep.handle(&other, &begin, 4), Err(Error::Missing));
}

#[test]
fn batched_prefix_reports_selection_and_falls_back_only_before_begin() {
    let t = Temp::new();
    let c = context();
    let base = fixture(65537);
    let mut target = base.clone();
    target[12345] ^= 4;
    let mut ep =
        Endpoint::open_with_policy(&t.0, Limits::default(), allow(), ColdGraphPolicy::Disabled)
            .unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    let mut calls = 0;
    let stats = client::transfer_zstd_prefix_batched(&base, &target, [2; 32], false, |frames| {
        calls += 1;
        let mut replies = Vec::new();
        for f in frames {
            let r = ep.handle(&c, f, 1)?;
            let terminal = r.kind == 103;
            replies.push(r);
            if terminal {
                break;
            }
        }
        Ok(replies)
    })
    .unwrap();
    assert_eq!(stats.mode as u8, 4);
    assert_eq!(calls, 1);
    assert!(stats.request_bytes + stats.response_bytes < 1000);
    assert_eq!(ep.read_object(&c, object_id(&target)).unwrap(), target);
    let unrelated = fixture(31);
    let stats = client::transfer_zstd_prefix_batched(&base, &unrelated, [3; 32], false, |frames| {
        frames.iter().map(|f| ep.handle(&c, f, 2)).collect()
    })
    .unwrap();
    assert_eq!(stats.mode, Mode::Raw);
    let empty = Temp::new();
    let mut absent = Endpoint::open_with_policy(
        &empty.0,
        Limits::default(),
        allow(),
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let mut calls = 0;
    assert_eq!(
        client::transfer_zstd_prefix_batched(&base, &target, [4; 32], false, |frames| {
            calls += 1;
            frames.iter().map(|f| absent.handle(&c, f, 3)).collect()
        })
        .unwrap_err(),
        Error::Missing
    );
    assert_eq!(calls, 1);
}

#[test]
fn sender_rejects_wrong_cumulative_encoded_receipt() {
    let base = fixture(4096);
    let mut target = base.clone();
    target[9] ^= 5;
    let error = client::transfer_zstd_prefix_batched(&base, &target, [5; 32], false, |frames| {
        Ok(frames
            .iter()
            .map(|f| match f.kind {
                1 => f.reply(101, [vec![4, 1], 1u64.to_le_bytes().to_vec()].concat()),
                2 => f.reply(102, 0u64.to_le_bytes().to_vec()),
                _ => f.reply(103, object_id(&target).to_vec()),
            })
            .collect())
    })
    .unwrap_err();
    assert_eq!(error, Error::Integrity);
}

#[test]
fn prefix_reservations_are_shared_and_cancel_releases_them() {
    let t = Temp::new();
    let c = context();
    let base = fixture(16384);
    let mut target = base.clone();
    target[9] ^= 7;
    let limits = Limits {
        resident_bytes: 20 << 20,
        ..Limits::default()
    };
    let mut ep =
        Endpoint::open_with_policy(&t.0, limits, allow(), ColdGraphPolicy::Disabled).unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    let begin = frame(1, 0, offer(&base, &target));
    for id in [2, 3, 4] {
        let mut f = begin.clone();
        f.transfer = [id; 32];
        assert_eq!(ep.handle(&c, &f, 1).unwrap().kind, 101);
    }
    let mut fourth = begin.clone();
    fourth.transfer = [5; 32];
    assert_eq!(ep.handle(&c, &fourth, 1), Err(Error::Limit));
    let cancel = Frame {
        kind: 6,
        seq: 1,
        transfer: [2; 32],
        body: vec![],
    };
    assert_eq!(ep.handle(&c, &cancel, 2).unwrap().kind, 105);
    assert_eq!(ep.handle(&c, &fourth, 2).unwrap().kind, 101);
}

#[test]
fn prefix_admission_precedes_corrupt_base_read_and_final_hash_is_mandatory() {
    let t = Temp::new();
    let c = context();
    let base = fixture(4096);
    let mut target = base.clone();
    target[9] ^= 7;
    let mut ep =
        Endpoint::open_with_policy(&t.0, Limits::default(), allow(), ColdGraphPolicy::Disabled)
            .unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    drop(ep);
    let path =
        t.0.join(spiralcore_transfer::hex(&c.namespace().unwrap()))
            .join(format!(
                "{}.object",
                spiralcore_transfer::hex(&object_id(&base))
            ));
    let original = std::fs::read(&path).unwrap();
    let mut corrupt = original.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    std::fs::write(&path, corrupt).unwrap();
    let limits = Limits {
        resident_bytes: 6 << 20,
        ..Limits::default()
    };
    let mut ep =
        Endpoint::open_with_policy(&t.0, limits, allow(), ColdGraphPolicy::Disabled).unwrap();
    assert_eq!(
        ep.handle(&c, &frame(1, 0, offer(&base, &target)), 0),
        Err(Error::Limit)
    );
    drop(ep);
    let mut ep =
        Endpoint::open_with_policy(&t.0, Limits::default(), allow(), ColdGraphPolicy::Disabled)
            .unwrap();
    assert_eq!(
        ep.handle(&c, &frame(1, 0, offer(&base, &target)), 0),
        Err(Error::Integrity)
    );
    std::fs::write(path, original).unwrap();
    let mut wrong = offer(&base, &target);
    wrong[9] ^= 1;
    ep.handle(&c, &frame(1, 0, wrong), 1).unwrap();
    let encoded = prefix::encode(&base, &target).unwrap();
    ep.handle(&c, &frame(2, 1, body(&encoded)), 1).unwrap();
    assert_eq!(
        ep.handle(&c, &frame(3, 2, vec![]), 1),
        Err(Error::Integrity)
    );
    assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
}

#[test]
fn prefix_offer_and_encoded_stream_are_strictly_versioned_and_bounded() {
    let base = fixture(4096);
    let target = vec![9; 4096];
    let mut valid = offer(&base, &target);
    let decoded = Offer::decode(&valid).unwrap();
    assert_eq!(decoded.mode, Mode::ZstdPrefixV1);
    valid[0] = u8::MAX;
    assert_eq!(Offer::decode(&valid), Err(Error::Unsupported));
    let mut oversized = decoded.clone();
    oversized.len = prefix::MAX_PREFIX_OBJECT as u64 + 1;
    assert_eq!(oversized.encode(), Err(Error::Limit));
    let mut no_base = decoded;
    no_base.base = None;
    assert_eq!(no_base.encode(), Err(Error::Malformed));
    let t = Temp::new();
    let c = context();
    let mut ep =
        Endpoint::open_with_policy(&t.0, Limits::default(), allow(), ColdGraphPolicy::Disabled)
            .unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    ep.handle(&c, &frame(1, 0, offer(&base, &target)), 1)
        .unwrap();
    let encoded = prefix::encode(&base, &target).unwrap();
    let mut invalid = body(&encoded);
    invalid[5] = 2;
    assert_eq!(
        ep.handle(&c, &frame(2, 1, invalid), 1),
        Err(Error::Unsupported)
    );
    assert_eq!(
        ep.handle(&c, &frame(2, 1, body(&encoded)), 1),
        Err(Error::Unsupported)
    );
    assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
}
