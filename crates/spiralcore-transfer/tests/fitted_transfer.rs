use spiralcore_transfer::{
    client,
    endpoint::{Authority, ColdGraphPolicy, Context, Endpoint, Limits},
    protocol::Frame,
    schema, zstd_dictionary, Result,
};
use spiralcore_transfer::{
    object_id,
    protocol::{Mode, Offer},
    Error,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-fitted-{}-{}",
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
fn fixtures() -> (Vec<(Mode, Vec<u8>)>, Vec<u8>) {
    let train: Vec<_> = (0..512).map(|i| format!("{{\"kind\":\"metric-reading\",\"region\":\"east\",\"host\":\"node-{i}\",\"sequence\":{i},\"status\":\"healthy\",\"message\":\"operating normally\"}}\n").into_bytes()).collect();
    let refs: Vec<_> = train.iter().map(Vec::as_slice).collect();
    let target = (700..1200).map(|i| format!("{{\"kind\":\"metric-reading\",\"region\":\"west\",\"host\":\"node-{i}\",\"sequence\":{i},\"status\":\"healthy\",\"message\":\"operating normally\"}}\n")).collect::<String>().into_bytes();
    (
        vec![
            (Mode::SchemaV1, schema::fit(&refs).unwrap()),
            (
                Mode::ZstdDictionaryV1,
                zstd_dictionary::train(&refs, 8192).unwrap(),
            ),
        ],
        target,
    )
}
fn offer(mode: Mode, artifact: &[u8], target: &[u8]) -> Offer {
    Offer {
        mode,
        len: target.len() as u64,
        object: object_id(target),
        manifest: None,
        base: Some(object_id(artifact)),
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
fn encoded(mode: Mode, artifact: &[u8], target: &[u8]) -> Vec<u8> {
    match mode {
        Mode::SchemaV1 => schema::encode(artifact, target).unwrap(),
        Mode::ZstdDictionaryV1 => zstd_dictionary::encode(artifact, target).unwrap(),
        _ => unreachable!(),
    }
}
fn body(mode: Mode, encoded: &[u8]) -> Vec<u8> {
    [
        vec![mode as u8],
        (encoded.len() as u32).to_le_bytes().to_vec(),
        vec![1],
        encoded.to_vec(),
    ]
    .concat()
}
fn send<F: FnMut(&[Frame]) -> Result<Vec<Frame>>>(
    mode: Mode,
    artifact: &[u8],
    target: &[u8],
    exchange: F,
) -> Result<client::Stats> {
    match mode {
        Mode::SchemaV1 => {
            client::transfer_schema_batched(artifact, target, [7; 32], false, exchange)
        }
        Mode::ZstdDictionaryV1 => {
            client::transfer_zstd_dictionary_batched(artifact, target, [7; 32], false, exchange)
        }
        _ => unreachable!(),
    }
}

#[test]
fn fitted_offer_requires_artifact_and_bounded_output() {
    for tag in [6, 7] {
        let mut bytes = Offer {
            mode: Mode::SparseV1,
            len: 500,
            object: object_id(b"target"),
            manifest: None,
            base: Some(object_id(b"artifact")),
        }
        .encode()
        .unwrap();
        bytes[0] = tag;
        let valid = Offer::decode(&bytes).expect("versioned fitted profile must be supported");
        let mut absent = valid.clone();
        absent.base = None;
        assert_eq!(absent.encode(), Err(Error::Malformed));
        let mut large = valid;
        large.len = (16 << 20) + 1;
        assert_eq!(large.encode(), Err(Error::Limit));
    }
}

#[test]
fn fitted_transfer_is_exact_durable_and_scoped_to_artifact_namespace() {
    let (artifacts, target) = fixtures();
    for (mode, artifact) in artifacts {
        let temp = Temp::new();
        let c = context();
        let authority = Allow(Arc::new(AtomicBool::new(true)));
        let mut ep = Endpoint::open_with_policy(
            &temp.0,
            Limits::default(),
            authority.clone(),
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        let mut calls = 0;
        assert_eq!(
            send(mode, &artifact, &target, |frames| {
                calls += 1;
                frames.iter().map(|f| ep.handle(&c, f, 0)).collect()
            })
            .unwrap_err(),
            Error::Missing
        );
        assert_eq!(calls, 1);
        client::transfer(&artifact, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
        let mut recipient = c.clone();
        recipient.recipient = "another-recipient".into();
        let mut policy = c.clone();
        policy.policy_epoch += 1;
        let mut execution = c.clone();
        execution.execution_epoch += 1;
        for wrong in [recipient, policy, execution] {
            assert_eq!(
                ep.handle(
                    &wrong,
                    &frame(1, 0, offer(mode, &artifact, &target).encode().unwrap()),
                    1
                ),
                Err(Error::Missing)
            );
        }
        let stats = send(mode, &artifact, &target, |frames| {
            frames.iter().map(|f| ep.handle(&c, f, 1)).collect()
        })
        .unwrap();
        assert_eq!(stats.mode, mode);
        assert!(stats.request_bytes + stats.response_bytes < target.len() as u64);
        assert_eq!(ep.read_object(&c, object_id(&target)).unwrap(), target);
        drop(ep);
        let mut ep = Endpoint::open_with_policy(
            &temp.0,
            Limits::default(),
            authority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        assert_eq!(ep.read_object(&c, object_id(&target)).unwrap(), target);
    }
}

#[test]
fn fitted_finish_rechecks_authority_and_digest_without_partial_publication() {
    let (artifacts, target) = fixtures();
    for (mode, artifact) in artifacts {
        for wrong_digest in [false, true] {
            let temp = Temp::new();
            let c = context();
            let authority = Allow(Arc::new(AtomicBool::new(true)));
            let mut ep = Endpoint::open_with_policy(
                &temp.0,
                Limits::default(),
                authority.clone(),
                ColdGraphPolicy::Disabled,
            )
            .unwrap();
            client::transfer(&artifact, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
            let mut o = offer(mode, &artifact, &target);
            if wrong_digest {
                o.object[0] ^= 1;
            }
            ep.handle(&c, &frame(1, 0, o.encode().unwrap()), 1).unwrap();
            let bytes = encoded(mode, &artifact, &target);
            let mut seq = 1;
            for chunk in bytes.chunks(spiralcore_transfer::MAX_CHUNK) {
                ep.handle(&c, &frame(2, seq, body(mode, chunk)), 2).unwrap();
                seq += 1;
            }
            assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
            if !wrong_digest {
                authority.0.store(false, Ordering::SeqCst);
            }
            assert_eq!(
                ep.handle(&c, &frame(3, seq, vec![]), 3),
                Err(if wrong_digest {
                    Error::Integrity
                } else {
                    Error::Unauthorized
                })
            );
            authority.0.store(true, Ordering::SeqCst);
            assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
        }
    }
}

#[test]
fn fitted_sender_falls_back_before_begin_and_rejects_false_receipts() {
    let (artifacts, target) = fixtures();
    for (mode, artifact) in artifacts {
        let c = context();
        for small in [b"".as_slice(), b"\0\xff\0"] {
            let temp = Temp::new();
            let mut ep = Endpoint::open_with_policy(
                &temp.0,
                Limits::default(),
                Allow(Arc::new(AtomicBool::new(true))),
                ColdGraphPolicy::Disabled,
            )
            .unwrap();
            let stats = send(mode, &artifact, small, |frames| {
                frames.iter().map(|f| ep.handle(&c, f, 0)).collect()
            })
            .unwrap();
            assert_eq!(stats.mode, Mode::Raw);
            assert_eq!(ep.read_object(&c, object_id(small)).unwrap(), small);
        }
        assert_eq!(
            send(mode, &[], b"", |_| panic!(
                "invalid artifact reached transport"
            ))
            .unwrap_err(),
            Error::Malformed
        );
        assert_eq!(
            send(mode, &artifact, &target, |frames| Ok(frames
                .iter()
                .map(|f| match f.kind {
                    1 => f.reply(
                        101,
                        [vec![mode as u8, 1], 1u64.to_le_bytes().to_vec()].concat()
                    ),
                    2 => f.reply(102, 0u64.to_le_bytes().to_vec()),
                    _ => f.reply(103, object_id(&target).to_vec()),
                })
                .collect()))
            .unwrap_err(),
            Error::Integrity
        );
    }
}

#[test]
fn fitted_admission_precedes_corrupt_artifact_read_and_cancel_releases_budget() {
    let (artifacts, target) = fixtures();
    for (mode, artifact) in artifacts {
        let temp = Temp::new();
        let c = context();
        let authority = Allow(Arc::new(AtomicBool::new(true)));
        let mut ep = Endpoint::open_with_policy(
            &temp.0,
            Limits::default(),
            authority.clone(),
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        client::transfer(&artifact, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
        drop(ep);
        let p = temp
            .0
            .join(spiralcore_transfer::hex(&c.namespace().unwrap()))
            .join(format!(
                "{}.object",
                spiralcore_transfer::hex(&object_id(&artifact))
            ));
        let original = std::fs::read(&p).unwrap();
        let mut corrupt = original.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        std::fs::write(&p, corrupt).unwrap();
        let low = Limits {
            resident_bytes: 1 << 20,
            ..Limits::default()
        };
        let mut ep =
            Endpoint::open_with_policy(&temp.0, low, authority.clone(), ColdGraphPolicy::Disabled)
                .unwrap();
        let begin = frame(1, 0, offer(mode, &artifact, &target).encode().unwrap());
        assert_eq!(ep.handle(&c, &begin, 0), Err(Error::Limit));
        drop(ep);
        let mut ep = Endpoint::open_with_policy(
            &temp.0,
            Limits::default(),
            authority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        assert_eq!(ep.handle(&c, &begin, 0), Err(Error::Integrity));
        std::fs::write(p, original).unwrap();
        ep.handle(&c, &begin, 0).unwrap();
        assert_eq!(ep.handle(&c, &frame(6, 1, vec![]), 1).unwrap().kind, 105);
        let mut next = begin;
        next.transfer = [8; 32];
        assert_eq!(ep.handle(&c, &next, 2).unwrap().kind, 101);
        let mut bad = body(mode, b"x");
        bad[5] = 2;
        let mut data = frame(2, 1, bad);
        data.transfer = [8; 32];
        assert_eq!(ep.handle(&c, &data, 2), Err(Error::Unsupported));
        assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
    }
}
