use spiralcore_gateway_core::{
    object::{ObjectMode, ObjectSender},
    packet::{inspect_ip, PacketDecoder, PacketEncoder, PacketMode},
};
use spiralcore_transfer::{
    endpoint::{ColdGraphPolicy, Endpoint},
    object_id,
    protocol::Frame,
    wan, Error,
};
use std::time::Duration;

fn packet(size: usize, random: bool) -> Vec<u8> {
    let mut p = vec![0; size];
    p[0] = 0x45;
    p[2..4].copy_from_slice(&(size as u16).to_be_bytes());
    p[8] = 64;
    p[9] = 17;
    p[12..16].copy_from_slice(&[10, 0, 0, 1]);
    p[16..20].copy_from_slice(&[10, 0, 0, 2]);
    let mut x = 73u64;
    for b in &mut p[20..] {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = if random { x as u8 } else { 7 };
    }
    p
}
#[test]
fn packet_exact_reordered_fragmented_duplicate_and_corrupt() {
    for mode in [PacketMode::Raw, PacketMode::Zstd] {
        let p = packet(1280, true);
        let mut enc = PacketEncoder::new(mode).unwrap();
        let frames = enc.encode(&p, 1, 700).unwrap();
        assert_eq!(frames.len(), 2);
        assert!(frames.iter().all(|f| f.len() <= 700));
        let mut dec = PacketDecoder::new().unwrap();
        assert!(dec.receive(&frames[1], Duration::ZERO).unwrap().is_none());
        assert!(dec.receive(&frames[1], Duration::ZERO).unwrap().is_none());
        assert_eq!(dec.receive(&frames[0], Duration::ZERO).unwrap().unwrap(), p);
        assert!(dec.receive(&frames[0], Duration::ZERO).unwrap().is_none());
        let mut bad = enc.encode(&p, 2, 1336).unwrap().remove(0);
        let last = bad.len() - 1;
        bad[last] ^= 1;
        assert!(dec.receive(&bad, Duration::ZERO).is_err());
    }
}
#[test]
fn packet_compression_length_expiry_and_bounded_admission() {
    let p = packet(1280, false);
    assert_eq!(inspect_ip(&p).unwrap().destination.to_string(), "10.0.0.2");
    let mut enc = PacketEncoder::new(PacketMode::Zstd).unwrap();
    let frames = enc.encode(&p, 1, 1336).unwrap();
    assert!(frames[0].len() < p.len());
    let mut dec = PacketDecoder::new().unwrap();
    assert_eq!(dec.receive(&frames[0], Duration::ZERO).unwrap().unwrap(), p);
    assert!(enc.encode(&p, 2, 699).is_err());
    let mut wrong = p.clone();
    wrong.push(0);
    assert!(enc.encode(&wrong, 2, 1336).is_err());
    let mut raw = PacketEncoder::new(PacketMode::Raw).unwrap();
    let f = raw.encode(&p, 3, 700).unwrap();
    assert!(dec.receive(&f[0], Duration::ZERO).unwrap().is_none());
    assert!(dec
        .receive(&f[1], Duration::from_secs(3))
        .unwrap()
        .is_none());
    assert_eq!(dec.counters().expired_packets, 1);
    let mut limited = PacketDecoder::new().unwrap();
    for seq in 0..64 {
        let f = raw.encode(&p, seq, 700).unwrap();
        assert!(limited.receive(&f[0], Duration::ZERO).unwrap().is_none());
    }
    let f = raw.encode(&p, 64, 700).unwrap();
    assert!(matches!(
        limited.receive(&f[0], Duration::ZERO),
        Err(Error::Limit)
    ));
    assert_eq!(limited.counters().capacity_drops, 1);
}

#[test]
fn ipv6_replay_window_conflicting_fragments_and_trailing_zstd_are_refused() {
    let mut ip6 = vec![3; 1280];
    ip6[0] = 0x60;
    ip6[4..6].copy_from_slice(&1240u16.to_be_bytes());
    ip6[6] = 17;
    assert!(inspect_ip(&ip6).unwrap().source.is_ipv6());
    let mut raw = PacketEncoder::new(PacketMode::Raw).unwrap();
    let mut decoder = PacketDecoder::new().unwrap();
    let frames = raw.encode(&ip6, 1, 700).unwrap();
    assert!(decoder
        .receive(&frames[0], Duration::ZERO)
        .unwrap()
        .is_none());
    let mut conflict = frames[0].clone();
    conflict[56] ^= 1;
    assert!(matches!(
        decoder.receive(&conflict, Duration::ZERO),
        Err(Error::Integrity)
    ));
    assert_eq!(decoder.counters().conflicting_fragments, 1);
    let newer = raw.encode(&ip6, 100, 1336).unwrap();
    assert_eq!(
        decoder.receive(&newer[0], Duration::ZERO).unwrap().unwrap(),
        ip6
    );
    assert!(decoder
        .receive(&frames[0], Duration::ZERO)
        .unwrap()
        .is_none());
    assert_eq!(decoder.counters().replay_drops, 1);
    let mut compressed = PacketEncoder::new(PacketMode::Zstd).unwrap();
    let mut frame = compressed
        .encode(&packet(1280, false), 101, 1336)
        .unwrap()
        .remove(0);
    assert_eq!(frame[20], 1);
    let length = u16::from_be_bytes([frame[18], frame[19]]) + 1;
    frame[18..20].copy_from_slice(&length.to_be_bytes());
    frame.push(0);
    assert!(decoder.receive(&frame, Duration::ZERO).is_err());
    for malformed in [vec![], vec![0; 55], vec![0; 1337]] {
        assert!(decoder.receive(&malformed, Duration::ZERO).is_err());
    }
}
#[test]
fn shared_object_modes_reopen_exact_and_reject_scope_before_exchange() {
    let context = wan::context([7; 32]);
    let schema = spiralcore_transfer::schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    for mode in [ObjectMode::Raw, ObjectMode::Whole, ObjectMode::Warm] {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let mut endpoint = Endpoint::open_with_policy(
            &cache,
            wan::limits(),
            wan::BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        let mut sender = ObjectSender::new(
            mode,
            &context,
            if mode == ObjectMode::Warm {
                Some(&schema)
            } else {
                None
            },
        )
        .unwrap();
        let bytes = vec![17; 65537];
        let mut exchange = |frames: &[Frame]| {
            let mut replies = Vec::new();
            for f in frames {
                let r = endpoint.handle(&context, f, 0)?;
                let done = matches!(r.kind, 103 | 127);
                replies.push(r);
                if done {
                    break;
                }
            }
            Ok(replies)
        };
        sender
            .seed(&context, bytes.clone(), [1; 32], &mut exchange)
            .unwrap();
        let stats = sender
            .send(&context, bytes.clone(), [2; 32], false, &mut exchange)
            .unwrap();
        assert_eq!(stats.object, object_id(&bytes));
        if mode == ObjectMode::Warm {
            assert_eq!(stats.data_bytes, 0);
        }
        let mut other = context.clone();
        other.execution_epoch += 1;
        assert!(matches!(
            sender.send(&other, bytes.clone(), [3; 32], false, |_| panic!(
                "unauthorized exchange"
            )),
            Err(Error::Unauthorized)
        ));
        drop(endpoint);
        let mut endpoint = Endpoint::open_with_policy(
            &cache,
            wan::limits(),
            wan::BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        assert_eq!(
            endpoint.read_object(&context, object_id(&bytes)).unwrap(),
            bytes
        );
    }
}
