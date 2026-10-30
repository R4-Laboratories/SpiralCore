use spiralcore_transfer::{
    client::transfer,
    dag::Graph,
    endpoint::{Authority, Context, Endpoint, Limits},
    protocol::{Frame, Mode},
    Error, Result,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-transfer-{}-{}",
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
struct Policy;
impl Authority for Policy {
    fn authorize(&self, c: &Context) -> Result<()> {
        if *c == context() {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}

#[test]
fn raw_zstd_repeat_and_warm_delta_reconstruct_exactly() {
    let t = Temp::new();
    let c = context();
    let mut ep = Endpoint::open(&t.0, Limits::default(), Policy).unwrap();
    let mut bytes = Vec::new();
    for i in 0u64..2048 {
        bytes.extend_from_slice(&spiralcore_transfer::hash(b"fixture", &[&i.to_le_bytes()]));
    }
    let mut exchange = |f: &Frame| Frame::decode(&ep.handle_bytes(&c, &f.encode()?, 1)?);
    let cold = transfer(&bytes, Mode::Zstd, None, [1; 32], &mut exchange).unwrap();
    let repeated = transfer(&bytes, Mode::Raw, None, [2; 32], &mut exchange).unwrap();
    assert_eq!(repeated.data_bytes, 0);
    assert!(repeated.request_bytes + repeated.response_bytes < 1024);
    let base = Graph::fold(&bytes, 262144).unwrap();
    bytes[101] ^= 17;
    bytes.extend_from_slice(b"tail\0");
    let warm = transfer(&bytes, Mode::Dag, Some(&base), [3; 32], &mut exchange).unwrap();
    assert!(warm.records_sent > 0);
    assert!(warm.request_bytes + warm.response_bytes < cold.request_bytes / 4);
    assert_eq!(ep.read_object(&c, warm.object).unwrap(), bytes);
    drop(ep);
    let mut ep = Endpoint::open(&t.0, Limits::default(), Policy).unwrap();
    assert_eq!(ep.read_object(&c, warm.object).unwrap(), bytes);
}

#[test]
fn unrelated_reply_and_zstd_trailing_frames_are_rejected() {
    let result = transfer(b"abc", Mode::Raw, None, [3; 32], |f| {
        Ok(Frame {
            transfer: [4; 32],
            ..f.reply(103, spiralcore_transfer::object_id(b"abc").to_vec())
        })
    });
    assert_eq!(result.unwrap_err(), Error::Integrity);
    let bytes = zstd::bulk::compress(b"abc", 1).unwrap();
    assert_eq!(
        spiralcore_transfer::endpoint::decode_zstd(&bytes, 3).unwrap(),
        b"abc"
    );
    let mut extra = bytes.clone();
    extra.extend_from_slice(&bytes);
    assert!(spiralcore_transfer::endpoint::decode_zstd(&extra, 6).is_err());
    let mut junk = bytes.clone();
    junk.push(0);
    assert!(spiralcore_transfer::endpoint::decode_zstd(&junk, 3).is_err());
    assert!(spiralcore_transfer::endpoint::decode_zstd(&bytes, 2).is_err());
}

#[test]
fn zstd_decoder_enforces_actual_window_limit() {
    use std::io::Write;
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 1).unwrap();
    encoder.window_log(21).unwrap();
    encoder.include_contentsize(false).unwrap();
    encoder.write_all(&[7; 60000]).unwrap();
    let encoded = encoder.finish().unwrap();
    assert!(spiralcore_transfer::endpoint::decode_zstd(&encoded, 60000).is_err());
}

#[test]
fn missing_record_batch_is_repaired_with_complete_descendants() {
    let t = Temp::new();
    let c = context();
    let mut ep = Endpoint::open(&t.0, Limits::default(), Policy).unwrap();
    let bytes = (0u64..32)
        .flat_map(|i| spiralcore_transfer::hash(b"repair-fixture", &[&i.to_le_bytes()]))
        .collect::<Vec<_>>();
    let graph = Graph::fold(&bytes, 262144).unwrap();
    let mut omitted = false;
    let stats = transfer(&bytes, Mode::Dag, None, [5; 32], |f| {
        let mut delivered = f.clone();
        if f.kind == 2 && !omitted {
            omitted = true;
            delivered.body = vec![2, 0, 0, 0, 0, 0, 0];
        }
        ep.handle(&c, &delivered, 1)
    })
    .unwrap();
    assert!(omitted);
    assert!(stats.records_sent > graph.nodes.len());
    assert_eq!(ep.read_object(&c, stats.object).unwrap(), bytes);
}

#[test]
fn peer_cannot_request_records_outside_the_offered_object() {
    let result = transfer(b"abc", Mode::Dag, None, [6; 32], |f| {
        Ok(match f.kind {
            1 => {
                let mut b = vec![2, 0];
                b.extend_from_slice(&1u64.to_le_bytes());
                f.reply(101, b)
            }
            2 => f.reply(102, 0u64.to_le_bytes().to_vec()),
            3 => {
                let mut b = 1u16.to_le_bytes().to_vec();
                b.extend_from_slice(&[99; 32]);
                f.reply(104, b)
            }
            _ => panic!("unexpected request"),
        })
    });
    assert_eq!(result.unwrap_err(), Error::Integrity);
}
