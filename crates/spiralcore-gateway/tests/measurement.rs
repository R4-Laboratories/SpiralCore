use spiralcore_gateway::measurement::{fixture, summarize_pcap};
#[test]
fn fixtures_are_exact_bounded_and_seeded() {
    let a = fixture(4096, "opaque", 7).unwrap();
    assert_eq!(a, fixture(4096, "opaque", 7).unwrap());
    assert_ne!(a, fixture(4096, "opaque", 8).unwrap());
    assert_ne!(a, fixture(4096, "pattern", 7).unwrap());
    assert!(fixture(0, "opaque", 1).is_err());
    assert!(fixture(1, "unknown", 1).is_err());
}
#[test]
fn pcap_counts_wire_lengths_not_snaplen_and_rejects_truncation() {
    let mut p = vec![0xd4, 0xc3, 0xb2, 0xa1, 2, 0, 4, 0];
    p.extend([0u8; 8]);
    p.extend(96u32.to_le_bytes());
    p.extend(1u32.to_le_bytes());
    let mut frame = vec![0u8; 42];
    frame[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
    frame[14] = 0x45;
    frame[16..18].copy_from_slice(&1028u16.to_be_bytes());
    frame[23] = 17;
    frame[34..36].copy_from_slice(&4433u16.to_be_bytes());
    frame[36..38].copy_from_slice(&5555u16.to_be_bytes());
    frame[38..40].copy_from_slice(&1008u16.to_be_bytes());
    p.extend([0u8; 8]);
    p.extend(42u32.to_le_bytes());
    p.extend(1042u32.to_le_bytes());
    p.extend(frame);
    let s = summarize_pcap(&p).unwrap();
    assert_eq!(s.ip_bytes, 1028);
    assert_eq!(s.udp_payload_bytes, 1000);
    assert_eq!(s.ethernet_bytes, 1042);
    assert_eq!(s.packets, 1);
    let mut coalesced = p.clone();
    coalesced[40 + 16..40 + 18].copy_from_slice(&2028u16.to_be_bytes());
    coalesced[36..40].copy_from_slice(&2042u32.to_le_bytes());
    coalesced[40 + 38..40 + 40].copy_from_slice(&2008u16.to_be_bytes());
    assert!(summarize_pcap(&coalesced).is_err());
    p.pop();
    assert!(summarize_pcap(&p).is_err());
}
