use spiralcore_gateway_core::packet::{PacketDecoder, PacketEncoder, PacketMode};
use std::time::Duration;

fn udp(seed: u64, pattern: u8, port: u16) -> Vec<u8> {
    let mut p = vec![0; 1280];
    p[..20].copy_from_slice(&[
        0x45, 99, 5, 0, 12, 34, 0x40, 0, 64, 17, 79, 93, 10, 55, 66, 77, 172, 31, 44, 55,
    ]);
    p[20..22].copy_from_slice(&port.to_be_bytes());
    p[22..24].copy_from_slice(&4242u16.to_be_bytes());
    p[24..26].copy_from_slice(&1260u16.to_be_bytes());
    p[26..28].copy_from_slice(&[83, 29]);
    let mut x = seed | 1;
    for (i, b) in p[28..].iter_mut().enumerate() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = match pattern {
            0 => x as u8,
            1 => b"a new packet payload pattern 1234"[i % 32],
            _ => x as u8 & 15,
        };
    }
    p
}
fn roundtrip(encoder: &mut PacketEncoder, packet: &[u8], sequence: u64) -> usize {
    let frames = encoder.encode(packet, sequence, 700).unwrap();
    let mut decoder = PacketDecoder::new().unwrap();
    let mut decoded = None;
    for frame in &frames {
        if let Some(p) = decoder.receive(frame, Duration::ZERO).unwrap() {
            assert!(decoded.replace(p).is_none());
        }
    }
    assert_eq!(decoded.unwrap(), packet);
    frames.iter().map(Vec::len).sum()
}
#[test]
fn fast_profile_preserves_exact_packets_and_existing_modes() {
    for mode in [
        PacketMode::Raw,
        PacketMode::Zstd,
        PacketMode::FastZstd,
        PacketMode::Adaptive,
    ] {
        let mut encoder = PacketEncoder::new(mode).unwrap();
        for n in 0..12 {
            roundtrip(&mut encoder, &udp(77 + n, u8::from(n % 2 == 0), 4123), n);
        }
        let counters = encoder.counters();
        assert_eq!(counters.packets, 12);
        assert_eq!(counters.raw_packets + counters.compressed_packets, 12);
        if mode == PacketMode::Raw {
            assert_eq!(counters.compression_attempts, 0);
        } else if mode != PacketMode::Adaptive {
            assert_eq!(counters.compression_attempts, 12);
        }
    }
}
#[test]
fn negative_history_skips_with_reprobes_and_detects_current_new_pattern() {
    let mut adaptive = PacketEncoder::new(PacketMode::Adaptive).unwrap();
    let mut fast = PacketEncoder::new(PacketMode::FastZstd).unwrap();
    for n in 0..11 {
        let p = udp(101 + n, 0, 4123);
        assert_eq!(roundtrip(&mut adaptive, &p, n), roundtrip(&mut fast, &p, n));
    }
    let c = adaptive.counters();
    assert_eq!(c.compression_attempts, 3);
    assert_eq!(c.adaptive_skips, 8);
    assert_eq!(c.reprobes, 1);
    let pattern = udp(811, 1, 4123);
    assert_eq!(
        roundtrip(&mut adaptive, &pattern, 11),
        roundtrip(&mut fast, &pattern, 11)
    );
    assert_eq!(adaptive.counters().guard_probes, 1);
    assert_eq!(adaptive.counters().compressed_packets, 1);
}
#[test]
fn entropy_only_transition_exposes_byte_regression_until_reprobe() {
    let mut adaptive = PacketEncoder::new(PacketMode::Adaptive).unwrap();
    let mut fast = PacketEncoder::new(PacketMode::FastZstd).unwrap();
    for n in 0..2 {
        roundtrip(&mut adaptive, &udp(501 + n, 0, 4123), n);
    }
    let entropy_only = udp(9001, 2, 4123);
    let skipped = roundtrip(&mut adaptive, &entropy_only, 2);
    let forced = roundtrip(&mut fast, &entropy_only, 2);
    assert!(
        skipped > forced,
        "retain a real heuristic miss, not a universal byte-saving claim"
    );
    for n in 3..=9 {
        roundtrip(&mut adaptive, &entropy_only, n);
    }
    assert_eq!(adaptive.counters().reprobes, 1);
    assert_eq!(adaptive.counters().compressed_packets, 1);
    assert_eq!(adaptive.counters().adaptive_skips, 7);
}
#[test]
fn flow_state_is_bounded_and_unsupported_packets_always_probe() {
    let mut adaptive = PacketEncoder::new(PacketMode::Adaptive).unwrap();
    for n in 0..65 {
        roundtrip(&mut adaptive, &udp(301 + n, 0, 4000 + n as u16), n);
    }
    assert_eq!(adaptive.counters().flow_evictions, 1);
    for n in 65..77 {
        let mut p = udp(301 + n, 0, 4123);
        p[9] = 253;
        roundtrip(&mut adaptive, &p, n);
    }
    assert_eq!(adaptive.counters().compression_attempts, 77);
    assert_eq!(adaptive.counters().adaptive_skips, 0);
}

#[test]
fn advisory_flow_parser_handles_ipv6_tcp_and_falls_back_on_unsupported_shapes() {
    let original = udp(7123, 0, 4123);
    let address_bytes = udp(81123, 0, 4123);
    let mut ipv6 = vec![0; 1280];
    ipv6[..8].copy_from_slice(&[0x63, 41, 72, 99, 4, 216, 17, 64]);
    ipv6[8..24].copy_from_slice(&address_bytes[300..316]);
    ipv6[24..40].copy_from_slice(&address_bytes[400..416]);
    ipv6[40..].copy_from_slice(&original[20..1260]);
    ipv6[44..46].copy_from_slice(&1240u16.to_be_bytes());
    let mut tcp = original.clone();
    tcp[9] = 6;
    tcp[32] = 5 << 4;
    tcp[33] = 0x18;
    for packet in [&ipv6, &tcp] {
        let mut adaptive = PacketEncoder::new(PacketMode::Adaptive).unwrap();
        for n in 0..3 {
            roundtrip(&mut adaptive, packet, n);
        }
        assert_eq!(adaptive.counters().compression_attempts, 2);
        assert_eq!(adaptive.counters().adaptive_skips, 1);
    }
    let mut ipv4_fragment = original.clone();
    ipv4_fragment[6] = 0x20;
    let mut ipv6_extension = ipv6.clone();
    ipv6_extension[6] = 0;
    let mut invalid_tcp_offset = tcp.clone();
    invalid_tcp_offset[32] = 4 << 4;
    let mut tcp_syn = tcp.clone();
    tcp_syn[33] = 2;
    let mut invalid_udp_length = original.clone();
    invalid_udp_length[25] ^= 1;
    let mut short_payload = original[..100].to_vec();
    short_payload[2..4].copy_from_slice(&100u16.to_be_bytes());
    short_payload[24..26].copy_from_slice(&80u16.to_be_bytes());
    for packet in [
        ipv4_fragment,
        ipv6_extension,
        invalid_tcp_offset,
        tcp_syn,
        invalid_udp_length,
        short_payload,
    ] {
        let mut adaptive = PacketEncoder::new(PacketMode::Adaptive).unwrap();
        for n in 0..4 {
            roundtrip(&mut adaptive, &packet, n);
        }
        assert_eq!(adaptive.counters().compression_attempts, 4);
        assert_eq!(adaptive.counters().adaptive_skips, 0);
    }
}
