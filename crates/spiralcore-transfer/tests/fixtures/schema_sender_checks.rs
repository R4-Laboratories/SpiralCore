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

#[test]
fn first_match_preserves_exact_charge_and_terminal_work_budget() {
    let row = b"{\"identifier\":7,\"status\":true}\n";
    let artifact = fit(&[row]);
    let templates = super::read_artifact(&artifact).unwrap();
    let record = super::parse_record(row).unwrap();
    let charge = 2 * templates.items[0].literal_len;
    for start in [
        0,
        super::MAX_MATCH_WORK - charge - 1,
        super::MAX_MATCH_WORK - charge,
        super::MAX_MATCH_WORK - charge + 1,
        super::MAX_MATCH_WORK,
    ] {
        let mut expected_work = start;
        let matched = if start < super::MAX_MATCH_WORK {
            templates.find(&record.template, &mut expected_work)
        } else {
            None
        };
        let expected = if matched.is_some() {
            baseline::encode(&artifact, row).unwrap()[48..].to_vec()
        } else {
            let mut bytes = Vec::new();
            super::write_literal(&mut bytes, row);
            bytes
        };
        let mut work = start;
        let mut actual = Vec::new();
        if !super::write_first_template(&templates.items[0], row, &mut work, &mut actual) {
            super::write_literal(&mut actual, row);
        }
        assert_eq!(work, expected_work, "starting work {start}");
        assert_eq!(actual, expected, "starting work {start}");
    }
}

#[test]
fn failed_speculation_never_spends_work_or_emits_partial_bytes() {
    let row = b"{\"identifier\":7,\"status\":true}\n";
    let artifact = fit(&[row]);
    let templates = super::read_artifact(&artifact).unwrap();
    for bytes in [
        b"{\"identifier\":7,\"status\":true}\nextra".as_slice(),
        b"{\"identifier\":7, \"status\":true}\n",
        b"{\"identifier\":7,\"status\":tru}\n",
        b"{\"identifier\":7,\"status\":\"\xff\"}\n",
    ] {
        let mut work = super::MAX_MATCH_WORK - 1;
        let mut encoded = vec![17, 23];
        assert!(!super::write_first_template(
            &templates.items[0],
            bytes,
            &mut work,
            &mut encoded
        ));
        assert_eq!(work, super::MAX_MATCH_WORK - 1);
        assert_eq!(encoded, [17, 23]);
    }
}

// The immutable oracle's fit, constants/Template/Templates and complete parser
// sections are byte-identical to production at SC-OPT-017 baseline 4bb79f9.
// Its original full-file SHA256 is retained in the existing source packet:
// 6c228201197e1ce26a6ac8137da490e0040ed26ab06d4576b330e8b240a2577a.
fn fit_same_result(training: &[&[u8]]) {
    match (baseline::fit(training), super::fit(training)) {
        (Ok(expected), Ok(actual)) => assert_eq!(actual, expected),
        (Err(expected), Err(actual)) => {
            assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
        }
        (expected, actual) => panic!("fit result changed: {expected:?} vs {actual:?}"),
    }
}

#[test]
fn fit_duplicate_mixed_lexical_and_invalid_records_keep_original_artifact() {
    let first = b" {\"key\":0,\"key\":\"value\",\"flag\":true,\"none\":null}\r\n";
    let variants: &[&[u8]] = &[
        first,
        b" {\"key\":-0.00e+12,\"key\":\"\\u0041\\n\",\"flag\":false,\"none\":null}\r\n",
        b" {\"key\":01,\"key\":\"bad-number\",\"flag\":true,\"none\":null}\r\n",
        b" {\"key\":0,\"key\":\"bad\\q\",\"flag\":true,\"none\":null}\r\n",
        b" {\"key\":0,\"key\":\"\xff\",\"flag\":true,\"none\":null}\r\n",
        b"{\"second\":null}\n",
        b" {\"key\":[],\"key\":\"nested\",\"flag\":true,\"none\":null}\r\n",
        b"{\"flag\":true,\"key\":0}\n",
        b" \t{}\r\n",
        b"\xff\0binary\n",
        b"",
        b"\n",
    ];
    let rows: Vec<&[u8]> = variants.iter().copied().cycle().take(512).collect();
    fit_same_result(&rows);
    let joined = rows.concat();
    fit_same_result(&[&joined]);
    for end in 0..first.len() {
        fit_same_result(&[first, &first[..end], variants[1], variants[5]]);
    }
    for row in [b"{}\n".as_slice(), first] {
        fit_same_result(&vec![row; super::MAX_TRAINING_RECORDS]);
    }
    for slots in [0, 1, 31, 32, 33] {
        let fields: Vec<String> = (0..slots).map(|i| format!("\"field_{i}\":{i}")).collect();
        let row = format!("{{{}}}\n", fields.join(",")).into_bytes();
        fit_same_result(&[&row, &row, variants[5], &row]);
    }
}

#[test]
fn fit_order_and_training_schema_limits_match_original() {
    let rows: Vec<Vec<u8>> = (0..super::MAX_TEMPLATES + 2)
        .map(|i| format!("{{\"key_{i}\":0}}\n").into_bytes())
        .collect();
    let mut training = Vec::new();
    for row in &rows {
        training.extend_from_slice(&[rows[0].as_slice(), row.as_slice()]);
    }
    fit_same_result(&training);
    training.reverse();
    fit_same_result(&training);
    fit_same_result(&vec![b"{}\n".as_slice(); super::MAX_TRAINING_RECORDS + 1]);
    let too_many_lines = b"\n".repeat(super::MAX_TRAINING_RECORDS + 1);
    fit_same_result(&[&too_many_lines]);
    for len in [super::MAX_TRAINING_BYTES, super::MAX_TRAINING_BYTES + 1] {
        let invalid = vec![b'x'; len];
        fit_same_result(&[&invalid]);
    }
    for len in [super::MAX_TEMPLATE_RECORD, super::MAX_TEMPLATE_RECORD + 1] {
        let mut row = b"{\"large\":\"".to_vec();
        row.resize(len - 3, b'x');
        row.extend_from_slice(b"\"}\n");
        fit_same_result(&[&row, &row, b"{\"other\":1}\n"]);
    }
    let large_layouts: Vec<Vec<u8>> = (0..4)
        .map(|i| format!("{{\"{i}{}\":0}}\n", "x".repeat(22000)).into_bytes())
        .collect();
    let mut training: Vec<&[u8]> = large_layouts.iter().map(Vec::as_slice).collect();
    training.extend_from_slice(&[&large_layouts[0], b"{\"small\":1}\n", &large_layouts[1]]);
    fit_same_result(&training);
}
