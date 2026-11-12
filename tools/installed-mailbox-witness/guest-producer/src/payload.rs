pub fn expected_body() -> Vec<u8> {
    (0..1024u32).map(|i| ((i * 73 + 19) % 251) as u8).collect()
}
