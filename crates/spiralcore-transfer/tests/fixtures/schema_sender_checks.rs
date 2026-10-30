//! Byte-identical encoder from 97b01a1, compiled privately to retain Reader access.
#[allow(dead_code)]
#[path = "schema_sender_baseline.rs"]
mod baseline;

fn check(artifact: &[u8], target: &[u8]) {
    let old = baseline::encode(artifact, target).unwrap();
    let new = crate::schema::encode(artifact, target).unwrap();
    let prepared = crate::schema::PreparedSchema::new(artifact).unwrap();
    assert_eq!(old, new, "SCVAL bytes differ for {} bytes", target.len());
    assert_eq!(prepared.encode(target).unwrap(), old);
    assert_eq!(
        baseline::decode(artifact, &new, target.len()).unwrap(),
        target
    );
    assert_eq!(prepared.decode(&new, target.len()).unwrap(), target);
    // Compress the old intermediate with the unchanged compressor/settings.
    let mut context = crate::zstd_dictionary::new_encoder().unwrap();
    let mut old_compressed = Vec::with_capacity(zstd::zstd_safe::compress_bound(old.len()));
    context.compress2(&mut old_compressed, &old).unwrap();
    assert_eq!(
        crate::schema_zstd::encode(artifact, target).unwrap(),
        old_compressed
    );
}

fn fit(rows: &[&[u8]]) -> Vec<u8> {
    let old = baseline::fit(rows).unwrap();
    assert_eq!(crate::schema::fit(rows).unwrap(), old);
    old
}

#[test]
fn lexical_layouts_invalid_tokens_and_every_truncation_preserve_old_bytes() {
    let first = b" {\"key\":0,\"key\":\"value\",\"enabled\":true,\"missing\":null}\r\n";
    let second = b"{\"different\":false}\n";
    let empty = b" \t{}\r\n";
    let artifact = fit(&[first, second, empty]);
    let cases: &[&[u8]] = &[
        first,
        second,
        empty,
        b"",
        b"\n",
        b"\0\xff\xfe\n",
        b" {\"key\":-0,\"key\":\"\\u0041\\n\\\"\\\\\",\"enabled\":false,\"missing\":null}\r\n",
        " {\"key\":1.20e+03,\"key\":\"é水\",\"enabled\":true,\"missing\":null}\r\n".as_bytes(),
        b" {\"key\":01,\"key\":\"value\",\"enabled\":true,\"missing\":null}\r\n",
        b" {\"key\":1e,\"key\":\"value\",\"enabled\":true,\"missing\":null}\r\n",
        b" {\"key\":{},\"key\":\"value\",\"enabled\":true,\"missing\":null}\r\n",
        b" {\"key\":0,\"key\":\"bad\\q\",\"enabled\":true,\"missing\":null}\r\n",
        b" {\"key\":0,\"key\":\"bad\x01\",\"enabled\":true,\"missing\":null}\r\n",
        b" {\"key\":0,\"key\":\"value\",\"enabled\":true,\"missing\":null} \r\n",
        b" {\"key\":0, \"key\":\"value\",\"enabled\":true,\"missing\":null}\r\n",
        b" {\"key\":0,\"key\":\"value\",\"enabled\":true,\"missing\":null}",
        b" {\"key\":0,\"key\":\"value\",\"enabled\":true,\"missing\":null}\r\ntrailing",
        b"{\"enabled\":true,\"key\":0}\n",
    ];
    for target in cases {
        check(&artifact, target);
    }
    for end in 0..first.len() {
        check(&artifact, &first[..end]);
    }
    check(&artifact, &cases.concat());
    check(&fit(&[]), &cases.concat());
    check(&fit(&[empty]), empty);
}

#[test]
fn slots_record_sizes_and_line_limit_preserve_old_bytes() {
    for slots in [1, 31, 32, 33] {
        let fields: Vec<String> = (0..slots).map(|i| format!("\"field_{i}\":{i}")).collect();
        let record = format!("{{{}}}\n", fields.join(",")).into_bytes();
        let artifact = fit(&[&record]);
        check(&artifact, &record);
        check(&artifact, &record.repeat(3));
    }
    let artifact = fit(&[b"{\"long_string\":\"x\"}\n"]);
    for len in [
        super::MAX_TEMPLATE_RECORD - 1,
        super::MAX_TEMPLATE_RECORD,
        super::MAX_TEMPLATE_RECORD + 1,
    ] {
        let mut record = b"{\"long_string\":\"".to_vec();
        record.resize(len - 3, b'x');
        record.extend_from_slice(b"\"}\n");
        assert_eq!(record.len(), len);
        check(&artifact, &record);
    }
    let artifact = fit(&[b"{}\n"]);
    for records in [
        super::MAX_RECORDS - 1,
        super::MAX_RECORDS,
        super::MAX_RECORDS + 1,
    ] {
        check(&artifact, &b"{}\n".repeat(records));
    }
}

#[test]
fn deterministic_mixed_sequences_preserve_matching_and_fallback_bytes() {
    let first = b"{\"identifier\":0,\"message\":\"seed\",\"flag\":true}\n";
    let second = b"{\"other\":null}\n";
    let artifact = fit(&[first, second]);
    let mut state = 0x790a_48d2_9173_633bu64;
    let mut target = Vec::new();
    for index in 0..512 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        match state % 7 {
            0 => target.extend_from_slice(second),
            1 => target.extend_from_slice(b"{\"identifier\":01,\"message\":\"x\",\"flag\":true}\n"),
            2 => target.extend_from_slice(b"\xff\0invalid\n"),
            3 => target
                .extend_from_slice(b" {\"identifier\":0,\"message\":\"space\",\"flag\":true}\n"),
            _ => target.extend_from_slice(
                format!(
                    "{{\"identifier\":{index},\"message\":\"v-{state}\\u0041\",\"flag\":false}}\n"
                )
                .as_bytes(),
            ),
        }
    }
    check(&artifact, &target);
}
