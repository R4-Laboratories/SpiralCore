//! Deterministic public bytes for the live interoperability witness.
#[derive(Clone)]
pub struct Fixture {
    pub kind: &'static str,
    pub bytes: Vec<u8>,
}

/// Registered mixed shapes match gateway `object_fixtures`: requested sizes
/// name tiers; the canonical trace has its own exact length.
pub fn mixed_recipe(
    size: usize,
    trial: u64,
) -> crate::Result<spiralcore_transfer::shared_runtime::Recipe> {
    use spiralcore_transfer::shared_runtime::{OutputKind, Recipe};
    let steps = match size {
        1_048_576 => 289,
        16_777_216 => 4096,
        _ => return Err("mixed campaign requires requested size 1 MiB or 16 MiB".into()),
    };
    if trial > 2 {
        return Err("mixed trial outside registered dose 0..2".into());
    }
    Ok(Recipe {
        steps,
        payload: 1.0 + trial as f64 / 8.0,
        output: OutputKind::Trace,
    })
}

// Same public SHA-256 fixture definition as gateway::measurement::fixture;
// reuse the existing hash primitive without adding a second dependency tree.
fn opaque(size: usize, seed: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(size);
    for index in 0..size.div_ceil(32) {
        let block = spiralcore_transfer::hash(
            b"spiralcore.gateway.fixture.v1",
            &[&seed.to_be_bytes(), &(index as u64).to_be_bytes()],
        );
        out.extend_from_slice(&block[..32.min(size - out.len())]);
    }
    out
}

fn schema_control(length: usize, steps: u32, trial: u64, drift: bool) -> crate::Result<Vec<u8>> {
    use std::io::Write;
    let seed = 0x4350_555f_5343_4831_u64
        ^ trial
        ^ (u64::from(steps) << 32)
        ^ if drift { 0x4452_4946_545f_3031 } else { 0 };
    let count = length / 256;
    let entropy = opaque(count * 115 + length % 256, seed);
    let mut out = Vec::with_capacity(length);
    for (index, row) in entropy[..count * 115].chunks_exact(115).enumerate() {
        let value = 1_000_000_000_u64 + u64::from(u32::from_le_bytes(row[..4].try_into()?));
        let key = if drift && index > 0 {
            "drifted"
        } else {
            "message"
        };
        write!(out, "{{\"value\":{value},\"{key}\":\"")?;
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for byte in &row[4..] {
            out.extend_from_slice(&[HEX[(byte >> 4) as usize], HEX[(byte & 15) as usize]]);
        }
        out.extend_from_slice(b"\"}\n");
    }
    out.extend(entropy[count * 115..].iter().map(|byte| b'a' + byte % 26));
    if out.len() != length {
        return Err("schema control length".into());
    }
    Ok(out)
}

/// All arms receive identical logical payloads. Setup publication is explicit
/// and paid only by the two arms that need those artifacts.
pub fn mixed_objects(size: usize, trial: u64, include_setup: bool) -> crate::Result<Vec<Fixture>> {
    use spiralcore_transfer::{
        schema,
        shared_runtime::{self, State},
    };
    let recipe = mixed_recipe(size, trial)?;
    let checkpoint = State::seeded(4096)?.encode()?;
    let generated = shared_runtime::replay(&checkpoint, &recipe, shared_runtime::MAX_OUTPUT)?;
    let expected = if size == 1 << 20 {
        1_047_202
    } else {
        14_851_384
    };
    if generated.len() != expected || checkpoint.len() != 3382 {
        return Err("registered runtime shape changed".into());
    }
    let mut residual = generated.clone();
    let unrelated = opaque(
        generated.len(),
        0x5350_4952_414c_u64 ^ trial ^ (u64::from(recipe.steps) << 32),
    );
    let mut edited = unrelated.clone();
    for i in 1..=16 {
        let offset = i * generated.len() / 17;
        for bytes in [&mut residual, &mut edited] {
            for byte in &mut bytes[offset..offset + 32] {
                *byte ^= 0xa5;
            }
        }
    }
    let mut result = Vec::with_capacity(if include_setup { 10 } else { 8 });
    if include_setup {
        result.push(Fixture {
            kind: "runtime-checkpoint",
            bytes: checkpoint,
        });
        result.push(Fixture {
            kind: "schema",
            bytes: schema::fit(&[b"{\"value\":0,\"message\":\"public training\"}\n"])?,
        });
    }
    let fit = schema_control(generated.len(), recipe.steps, trial, false)?;
    let drift = schema_control(generated.len(), recipe.steps, trial, true)?;
    result.extend([
        Fixture {
            kind: "generated",
            bytes: generated,
        },
        Fixture {
            kind: "residual",
            bytes: residual.clone(),
        },
        Fixture {
            kind: "repeat",
            bytes: residual,
        },
        Fixture {
            kind: "unrelated",
            bytes: unrelated,
        },
        Fixture {
            kind: "opaque-edited",
            bytes: edited.clone(),
        },
        Fixture {
            kind: "opaque-edited-repeat",
            bytes: edited,
        },
        Fixture {
            kind: "schema-fit",
            bytes: fit,
        },
        Fixture {
            kind: "schema-drift",
            bytes: drift,
        },
    ]);
    Ok(result)
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
