use spiralcore_transfer::{
    client,
    endpoint::{Authority, ColdGraphPolicy, Context, Endpoint, Limits},
    object_id,
    protocol::{Frame, Mode, Offer},
    schema, schema_zstd, zstd_dictionary, Error, Result,
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
            "spiral-reuse-{}-{}",
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
fn records(key: usize, start: usize) -> Vec<Vec<u8>> {
    (start..start+256).map(|i|format!("{{\"kind{key}\":\"sensor-record\",\"region\":\"eastern-campus\",\"node\":\"node-{i:06}\",\"sequence\":{i},\"status\":\"operating normally\",\"units\":\"kelvin\"}}\n").into_bytes()).collect()
}
fn fixture(mode: Mode, key: usize) -> (Vec<u8>, Vec<u8>) {
    let training = records(key, 0);
    let samples: Vec<_> = training.iter().map(Vec::as_slice).collect();
    let artifact = if mode == Mode::SchemaZstdV1 {
        schema::fit(&samples).unwrap()
    } else {
        zstd_dictionary::train(&samples, 8192).unwrap()
    };
    (artifact, records(key, 1000).concat())
}
fn seed(ep: &mut Endpoint<Allow>, c: &Context, artifact: &[u8], id: u8) {
    client::transfer_batched(artifact, Mode::Raw, None, [id; 32], false, |frames| {
        frames.iter().map(|f| ep.handle(c, f, 0)).collect()
    })
    .unwrap();
}
fn send(
    ep: &mut Endpoint<Allow>,
    c: &Context,
    mode: Mode,
    artifact: &[u8],
    target: &[u8],
    id: u8,
) -> Result<client::Stats> {
    match mode {
        Mode::SchemaZstdV1 => {
            client::transfer_schema_zstd_batched(artifact, target, [id; 32], false, |frames| {
                frames.iter().map(|f| ep.handle(c, f, 1)).collect()
            })
        }
        Mode::ZstdDictionaryV1 => {
            client::transfer_zstd_dictionary_batched(artifact, target, [id; 32], false, |frames| {
                frames.iter().map(|f| ep.handle(c, f, 1)).collect()
            })
        }
        _ => unreachable!(),
    }
}
fn frame(kind: u8, seq: u64, id: u8, body: Vec<u8>) -> Frame {
    Frame {
        kind,
        seq,
        transfer: [id; 32],
        body,
    }
}
fn offer(mode: Mode, artifact: &[u8], target: &[u8]) -> Vec<u8> {
    Offer {
        mode,
        len: target.len() as u64,
        object: object_id(target),
        manifest: None,
        base: Some(object_id(artifact)),
    }
    .encode()
    .unwrap()
}

#[test]
fn compressed_schema_offer_is_versioned_and_bounded() {
    let mut encoded = Offer {
        mode: Mode::SchemaV1,
        len: 1000,
        object: object_id(b"target"),
        manifest: None,
        base: Some(object_id(b"schema")),
    }
    .encode()
    .unwrap();
    encoded[0] = 8;
    let mut o = Offer::decode(&encoded).expect("compressed schema profile must be supported");
    o.len = (16 << 20) + 1;
    assert_eq!(o.encode(), Err(Error::Limit));
    o.len = 1000;
    o.base = None;
    assert_eq!(o.encode(), Err(Error::Malformed));
}

#[test]
fn persistent_prepared_senders_and_receiver_reuse_exact_objects_then_restart() {
    for mode in [Mode::SchemaZstdV1, Mode::ZstdDictionaryV1] {
        let (artifact, _) = fixture(mode, 0);
        let t = Temp::new();
        let c = context();
        let mut ep =
            Endpoint::open_with_policy(&t.0, Limits::default(), allow(), ColdGraphPolicy::Disabled)
                .unwrap();
        ep.set_codec_cache_enabled(true).unwrap();
        seed(&mut ep, &c, &artifact, 1);
        let mut schema_encoder = if mode == Mode::SchemaZstdV1 {
            Some(schema_zstd::PreparedEncoder::new(&artifact).unwrap())
        } else {
            None
        };
        let mut dict_encoder = if mode == Mode::ZstdDictionaryV1 {
            Some(zstd_dictionary::PreparedEncoder::new(&artifact).unwrap())
        } else {
            None
        };
        let mut targets = vec![];
        for n in 0..4 {
            let target = records(0, 1000 + n * 256).concat();
            let stats = if let Some(p) = schema_encoder.as_mut() {
                client::transfer_prepared_schema_zstd_batched(
                    p,
                    &target,
                    [2 + n as u8; 32],
                    false,
                    |frames| frames.iter().map(|f| ep.handle(&c, f, 1)).collect(),
                )
                .unwrap()
            } else {
                client::transfer_prepared_zstd_dictionary_batched(
                    dict_encoder.as_mut().unwrap(),
                    &target,
                    [2 + n as u8; 32],
                    false,
                    |frames| frames.iter().map(|f| ep.handle(&c, f, 1)).collect(),
                )
                .unwrap()
            };
            assert_eq!(stats.mode, mode);
            assert_eq!(ep.read_object(&c, object_id(&target)).unwrap(), target);
            targets.push(target);
        }
        let stats = ep.codec_cache_stats();
        assert_eq!(
            (stats.misses, stats.hits, stats.preparations, stats.entries),
            (1, 3, 1, 1)
        );
        assert_eq!(stats.resident_bytes, 5 << 20);
        drop(ep);
        let mut ep =
            Endpoint::open_with_policy(&t.0, Limits::default(), allow(), ColdGraphPolicy::Disabled)
                .unwrap();
        assert_eq!(ep.codec_cache_stats().entries, 0);
        for target in targets {
            assert_eq!(ep.read_object(&c, object_id(&target)).unwrap(), target);
        }
    }
}

#[test]
fn warm_decoder_does_not_hide_corrupt_missing_or_unauthorized_artifact() {
    for mode in [Mode::SchemaZstdV1, Mode::ZstdDictionaryV1] {
        let (artifact, target) = fixture(mode, 0);
        let t = Temp::new();
        let c = context();
        let authority = allow();
        let mut ep = Endpoint::open_with_policy(
            &t.0,
            Limits::default(),
            authority.clone(),
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        ep.set_codec_cache_enabled(true).unwrap();
        seed(&mut ep, &c, &artifact, 1);
        send(&mut ep, &c, mode, &artifact, &target, 2).unwrap();
        let next = records(0, 2000).concat();
        let path =
            t.0.join(spiralcore_transfer::hex(&c.namespace().unwrap()))
                .join(format!(
                    "{}.object",
                    spiralcore_transfer::hex(&object_id(&artifact))
                ));
        let original = std::fs::read(&path).unwrap();
        let mut corrupt = original.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        std::fs::write(&path, corrupt).unwrap();
        assert_eq!(
            send(&mut ep, &c, mode, &artifact, &next, 3).unwrap_err(),
            Error::Integrity
        );
        assert_eq!(ep.codec_cache_stats().hits, 0);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            send(&mut ep, &c, mode, &artifact, &next, 4).unwrap_err(),
            Error::Missing
        );
        std::fs::write(&path, original).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        authority.0.store(false, Ordering::SeqCst);
        assert_eq!(
            send(&mut ep, &c, mode, &artifact, &next, 5).unwrap_err(),
            Error::Unauthorized
        );
        authority.0.store(true, Ordering::SeqCst);
        let mut wrong = c.clone();
        wrong.policy_epoch += 1;
        assert_eq!(
            send(&mut ep, &wrong, mode, &artifact, &next, 6).unwrap_err(),
            Error::Missing
        );
        wrong = c.clone();
        wrong.recipient = "other".into();
        assert_eq!(
            send(&mut ep, &wrong, mode, &artifact, &next, 7).unwrap_err(),
            Error::Missing
        );
        send(&mut ep, &c, mode, &artifact, &next, 8).unwrap();
        assert_eq!(ep.codec_cache_stats().hits, 1);
        assert_eq!(ep.read_object(&c, object_id(&next)).unwrap(), next);
    }
}

#[test]
fn codec_cache_is_bounded_pinned_and_released_without_exceeding_shared_budget() {
    let t = Temp::new();
    let c = context();
    let mut ep =
        Endpoint::open_with_policy(&t.0, Limits::default(), allow(), ColdGraphPolicy::Disabled)
            .unwrap();
    ep.set_codec_cache_enabled(true).unwrap();
    let mode = Mode::SchemaZstdV1;
    let (pinned, target) = fixture(mode, 0);
    seed(&mut ep, &c, &pinned, 1);
    ep.handle(&c, &frame(1, 0, 2, offer(mode, &pinned, &target)), 1)
        .unwrap();
    assert_eq!(ep.set_codec_cache_enabled(false), Err(Error::Sequence));
    for n in 1..10 {
        let (artifact, target) = fixture(mode, n);
        seed(&mut ep, &c, &artifact, (n * 2 + 1) as u8);
        send(&mut ep, &c, mode, &artifact, &target, (n * 2 + 2) as u8).unwrap();
        assert!(ep.codec_cache_stats().entries <= 8);
    }
    assert_eq!(ep.codec_cache_stats().entries, 8);
    assert_eq!(ep.codec_cache_stats().evictions, 2);
    assert_eq!(ep.codec_cache_stats().resident_bytes, 40 << 20);
    ep.handle(&c, &frame(6, 1, 2, vec![]), 2).unwrap();
    send(&mut ep, &c, mode, &pinned, &target, 25).unwrap();
    assert_eq!(ep.codec_cache_stats().hits, 1);
    ep.set_codec_cache_enabled(false).unwrap();
    assert_eq!(ep.codec_cache_stats().resident_bytes, 0);
    let t = Temp::new();
    let mut low = Endpoint::open_with_policy(
        &t.0,
        Limits {
            resident_bytes: 8 << 20,
            ..Limits::default()
        },
        allow(),
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    low.set_codec_cache_enabled(true).unwrap();
    seed(&mut low, &c, &pinned, 1);
    send(&mut low, &c, mode, &pinned, &target, 2).unwrap();
    assert_eq!(low.codec_cache_stats().entries, 0);
    assert_eq!(low.read_object(&c, object_id(&target)).unwrap(), target);
}

#[test]
fn bad_compressed_transfer_cannot_publish_and_cached_decoder_recovers() {
    let mode = Mode::SchemaZstdV1;
    let (artifact, target) = fixture(mode, 0);
    let t = Temp::new();
    let c = context();
    let mut ep =
        Endpoint::open_with_policy(&t.0, Limits::default(), allow(), ColdGraphPolicy::Disabled)
            .unwrap();
    ep.set_codec_cache_enabled(true).unwrap();
    seed(&mut ep, &c, &artifact, 1);
    ep.handle(&c, &frame(1, 0, 2, offer(mode, &artifact, &target)), 0)
        .unwrap();
    let mut encoded = schema_zstd::encode(&artifact, &target).unwrap();
    *encoded.last_mut().unwrap() ^= 1;
    let body = [
        vec![8],
        (encoded.len() as u32).to_le_bytes().to_vec(),
        vec![1],
        encoded,
    ]
    .concat();
    ep.handle(&c, &frame(2, 1, 2, body), 0).unwrap();
    assert_eq!(
        ep.handle(&c, &frame(3, 2, 2, vec![]), 0),
        Err(Error::Integrity)
    );
    assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
    send(&mut ep, &c, mode, &artifact, &target, 3).unwrap();
    assert_eq!(ep.codec_cache_stats().hits, 1);
    assert_eq!(ep.read_object(&c, object_id(&target)).unwrap(), target);
}
