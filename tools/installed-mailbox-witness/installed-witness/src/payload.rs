pub fn expected_body() -> Vec<u8> {
    (0..1024u32).map(|i| ((i * 73 + 19) % 251) as u8).collect()
}


pub fn paired_body(slot: u8) -> Vec<u8> {
    let mut bytes = expected_body();
    bytes[..4].copy_from_slice(&[0x53, 0x43, slot, 0]);
    bytes
}
