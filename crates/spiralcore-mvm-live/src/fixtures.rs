//! Deterministic public bytes for the live interoperability witness.
#[derive(Clone)]
pub struct Fixture {
    pub kind: &'static str,
    pub bytes: Vec<u8>,
}

pub fn objects(size: usize, trial: u64) -> std::io::Result<Vec<Fixture>> {
    if !(64..=(16 << 20)).contains(&size) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fixture size outside profile",
        ));
    }
    let random = |mut seed: u64| -> Vec<u8> {
        (0..size)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed as u8
            })
            .collect()
    };
    let mut previous = random(0x5842_aae1_42fc_1983 ^ trial);
    let mut result = vec![Fixture {
        kind: "seed",
        bytes: previous.clone(),
    }];
    for kind in [
        "repeat",
        "localized",
        "unrelated",
        "localized-after",
        "truncate",
        "append",
    ] {
        match kind {
            "localized" | "localized-after" => {
                let middle = previous.len() / 2;
                for byte in &mut previous[middle..middle + 32] {
                    *byte ^= 0xa5;
                }
            }
            "unrelated" => previous = random(0x4b1d_8739_0921_4401 ^ trial),
            "truncate" => previous.truncate(previous.len() - 31),
            "append" => previous.extend_from_slice(b"append\0bytes\0end\0"),
            "repeat" => {}
            _ => unreachable!(),
        }
        result.push(Fixture {
            kind,
            bytes: previous.clone(),
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn deterministic_mixed_sequence_preserves_exact_lengths_and_binary_edits() {
        let cases = super::objects(4096, 7).unwrap();
        assert_eq!(cases.len(), 7);
        assert_eq!(cases[0].bytes, cases[1].bytes);
        assert_eq!(cases[0].bytes, super::objects(4096, 7).unwrap()[0].bytes);
        assert_ne!(cases[0].bytes, super::objects(4096, 8).unwrap()[0].bytes);
        assert_eq!(
            cases[0]
                .bytes
                .iter()
                .zip(&cases[2].bytes)
                .filter(|(a, b)| a != b)
                .count(),
            32
        );
        assert_ne!(cases[2].bytes, cases[3].bytes);
        assert_eq!(&cases[4].bytes[..4065], cases[5].bytes);
        assert_eq!(&cases[6].bytes[..4065], cases[5].bytes);
        assert_eq!(&cases[6].bytes[4065..], b"append\0bytes\0end\0");
        assert!(super::objects(63, 0).is_err());
        assert!(super::objects((16 << 20) + 1, 0).is_err());
    }
}
