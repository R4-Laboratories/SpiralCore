//! Backend-independent SHA-256 known answers, including padding and multiblock inputs.
//! Run the same tests with the default and hardware-sha256 Cargo configurations.

use sha2::{Digest, Sha256};

#[test]
fn sha256_known_answers_survive_backend_selection() {
    let vectors: [(&[u8], &str); 4] = [
        (b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        (b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        (b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"),
        (b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu", "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"),
    ];
    for (input, expected) in vectors {
        assert_eq!(
            spiralcore_transfer::hex(&Sha256::digest(input).into()),
            expected
        );
        for split in 0..=input.len() {
            let mut h = Sha256::new();
            h.update(&input[..split]);
            h.update(&input[split..]);
            assert_eq!(spiralcore_transfer::hex(&h.finalize().into()), expected);
        }
    }
    assert_eq!(
        spiralcore_transfer::hex(&Sha256::digest(vec![b'a'; 1_000_000]).into()),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}
