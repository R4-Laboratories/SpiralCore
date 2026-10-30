use spiralcore_transfer::{object_id, schema, Error};

const TRAIN: &[u8] =
    b"{\"long_field_name\":1,\"status_message\":\"train\",\"enabled_flag\":true}\n";

#[test]
fn frozen_training_template_preserves_held_out_raw_tokens() {
    let artifact = schema::fit(&[TRAIN]).unwrap();
    assert_eq!(artifact, schema::fit(&[TRAIN]).unwrap());
    let target = b"{\"long_field_name\":-1.20e+03,\"status_message\":\"held\\u0020out\",\"enabled_flag\":null}\n";
    let encoded = schema::encode(&artifact, target).unwrap();
    assert_eq!(&encoded[8..40], &object_id(&artifact));
    assert_eq!(encoded[48], 1, "held-out values use the frozen layout");
    assert!(encoded.len() - 48 < target.len() + 5);
    assert_eq!(
        schema::decode(&artifact, &encoded, target.len()).unwrap(),
        target
    );
    assert_eq!(
        artifact,
        schema::fit(&[TRAIN]).unwrap(),
        "encoding never fits held-out data"
    );
}

#[test]
fn arbitrary_bytes_layout_drift_and_nested_values_remain_exact_literals() {
    let artifact = schema::fit(&[TRAIN]).unwrap();
    for target in [
        b"\0\xffbinary\0\n\r\xff".as_slice(),
        b"{\"status_message\":\"x\",\"long_field_name\":1,\"enabled_flag\":true}\n",
        b"{\"long_field_name\":{\"nested\":1},\"status_message\":\"x\",\"enabled_flag\":true}\n",
        b"{\"long_field_name\":01,\"status_message\":\"x\",\"enabled_flag\":true}\n",
        b"[1,2,3]\n",
        b"{\"unclosed\":\"value}",
    ] {
        let encoded = schema::encode(&artifact, target).unwrap();
        assert_eq!(encoded[48], 0);
        assert_eq!(
            schema::decode(&artifact, &encoded, target.len()).unwrap(),
            target
        );
    }
}

#[test]
fn malformed_artifact_is_rejected_even_for_a_literal_target() {
    let artifact = schema::fit(&[TRAIN]).unwrap();
    for cut in 0..artifact.len() {
        assert!(
            schema::encode(&artifact[..cut], b"binary").is_err(),
            "cut={cut}"
        );
    }
    let mut trailing = artifact.clone();
    trailing.push(0);
    assert!(schema::encode(&trailing, b"binary").is_err());
    let mut wrong_profile = artifact.clone();
    wrong_profile[8..10].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(
        schema::encode(&wrong_profile, b"binary"),
        Err(Error::Unsupported)
    );
}

#[test]
fn wrong_schema_and_encoded_lengths_are_rejected() {
    let artifact = schema::fit(&[TRAIN]).unwrap();
    let encoded = schema::encode(&artifact, TRAIN).unwrap();
    for cut in 0..encoded.len() {
        assert!(
            schema::decode(&artifact, &encoded[..cut], TRAIN.len()).is_err(),
            "cut={cut}"
        );
    }
    let other = schema::fit(&[b"{\"different_key\":0}\n"]).unwrap();
    assert_eq!(
        schema::decode(&other, &encoded, TRAIN.len()),
        Err(Error::Integrity)
    );
    assert!(schema::decode(&artifact, &encoded, TRAIN.len() - 1).is_err());
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(schema::decode(&artifact, &trailing, TRAIN.len()).is_err());
}

#[test]
fn duplicate_keys_whitespace_crlf_escapes_and_numeric_spellings_are_preserved() {
    let train = b" \t{ \"duplicate_key\" : 0, \"duplicate_key\":null }\r\n";
    let artifact = schema::fit(&[train]).unwrap();
    for token in [
        b"-0".as_slice(),
        b"1.0",
        b"1e0",
        b"-1.20E+999999",
        b"false",
        b"\"\\u0041\\/\\n\\\"\"",
        b"\"\xf0\x9f\x98\x80\"",
    ] {
        let target = [
            b" \t{ \"duplicate_key\" : ".as_slice(),
            token,
            b", \"duplicate_key\":true }\r\n",
        ]
        .concat();
        let encoded = schema::encode(&artifact, &target).unwrap();
        assert_eq!(encoded[48], 1);
        assert_eq!(
            schema::decode(&artifact, &encoded, target.len()).unwrap(),
            target
        );
    }
}

#[test]
fn empty_mixed_and_newline_heavy_objects_have_exact_fallback() {
    let artifact = schema::fit(&[TRAIN, &TRAIN[..TRAIN.len() - 1], b"{}\n"]).unwrap();
    for target in [
        Vec::new(),
        [TRAIN, b"\xff\0\n", b"{}\n", &TRAIN[..TRAIN.len() - 1]].concat(),
        vec![b'\n'; schema::MAX_RECORDS + 1],
        vec![b'x'; schema::MAX_TEMPLATE_RECORD + 1],
    ] {
        let encoded = schema::encode(&artifact, &target).unwrap();
        assert!(encoded.len() <= schema::MAX_ENCODED);
        assert_eq!(
            schema::decode(&artifact, &encoded, target.len()).unwrap(),
            target
        );
        if target.is_empty() {
            assert_eq!(encoded.len(), 48);
        }
        if target.len() == schema::MAX_RECORDS + 1 {
            assert_eq!(u32::from_le_bytes(encoded[44..48].try_into().unwrap()), 1);
            assert_eq!(encoded[48], 0);
        }
    }
}

#[test]
fn malformed_templates_duplicates_and_profile_caps_are_rejected() {
    let artifact = schema::fit(&[TRAIN]).unwrap();
    for (offset, value) in [
        (10, (schema::MAX_TEMPLATES + 1) as u16),
        (12, (schema::MAX_SLOTS + 1) as u16),
    ] {
        let mut bad = artifact.clone();
        bad[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        assert_eq!(schema::encode(&bad, b"binary"), Err(Error::Limit));
    }
    let mut bad_fragment = artifact.clone();
    bad_fragment[14..18].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(schema::encode(&bad_fragment, b"binary").is_err());
    let mut invalid_layout = artifact.clone();
    invalid_layout[18] = b'[';
    assert_eq!(
        schema::encode(&invalid_layout, b"binary"),
        Err(Error::Malformed)
    );
    let mut duplicate = artifact.clone();
    duplicate[10..12].copy_from_slice(&2u16.to_le_bytes());
    duplicate.extend_from_slice(&artifact[12..]);
    assert_eq!(schema::encode(&duplicate, b"binary"), Err(Error::Malformed));
    assert_eq!(
        schema::encode(&vec![0; schema::MAX_SCHEMA + 1], b"binary"),
        Err(Error::Limit)
    );
}

#[test]
fn hostile_stream_counts_indices_tokens_and_lengths_fail_preflight() {
    let artifact = schema::fit(&[TRAIN]).unwrap();
    let encoded = schema::encode(&artifact, TRAIN).unwrap();
    assert_eq!(encoded[48], 1);
    let mut excessive_count = encoded.clone();
    excessive_count[44..48].copy_from_slice(&((schema::MAX_RECORDS + 1) as u32).to_le_bytes());
    assert_eq!(
        schema::decode(&artifact, &excessive_count, TRAIN.len()),
        Err(Error::Limit)
    );
    let mut index = encoded.clone();
    index[49..51].copy_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(
        schema::decode(&artifact, &index, TRAIN.len()),
        Err(Error::Malformed)
    );
    for size in [0, u32::MAX] {
        let mut bad = encoded.clone();
        bad[51..55].copy_from_slice(&size.to_le_bytes());
        assert!(schema::decode(&artifact, &bad, TRAIN.len()).is_err());
    }
    let mut invalid_token = encoded.clone();
    invalid_token[55] = b'X';
    assert_eq!(
        schema::decode(&artifact, &invalid_token, TRAIN.len()),
        Err(Error::Malformed)
    );
    let mut short_output = encoded.clone();
    short_output[40..44].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(
        schema::decode(&artifact, &short_output, 1),
        Err(Error::Limit)
    );
    let mut unknown_tag = encoded.clone();
    unknown_tag[48] = 255;
    assert_eq!(
        schema::decode(&artifact, &unknown_tag, TRAIN.len()),
        Err(Error::Unsupported)
    );
    assert_eq!(
        schema::decode(&artifact, &encoded, schema::MAX_TARGET + 1),
        Err(Error::Limit)
    );
}

#[test]
fn training_limits_and_template_count_are_bounded() {
    assert_eq!(
        schema::fit(&[&vec![0; schema::MAX_TRAINING_BYTES + 1]]),
        Err(Error::Limit)
    );
    assert_eq!(
        schema::fit(&[&vec![b'\n'; schema::MAX_TRAINING_RECORDS + 1]]),
        Err(Error::Limit)
    );
    assert_eq!(
        schema::fit(&vec![&[][..]; schema::MAX_TRAINING_RECORDS + 1]),
        Err(Error::Limit)
    );
    let records: Vec<Vec<u8>> = (0..schema::MAX_TEMPLATES + 1)
        .map(|i| format!("{{\"distinct_field_{i}\":0}}\n").into_bytes())
        .collect();
    let refs: Vec<&[u8]> = records.iter().map(Vec::as_slice).collect();
    let artifact = schema::fit(&refs).unwrap();
    assert_eq!(
        u16::from_le_bytes(artifact[10..12].try_into().unwrap()) as usize,
        schema::MAX_TEMPLATES
    );
    let last = records.last().unwrap();
    let encoded = schema::encode(&artifact, last).unwrap();
    assert_eq!(
        encoded[48], 0,
        "fitting cap does not silently add held-out layouts"
    );
    assert_eq!(
        schema::decode(&artifact, &encoded, last.len()).unwrap(),
        *last
    );
    assert_eq!(
        schema::encode(&artifact, &vec![0; schema::MAX_TARGET + 1]),
        Err(Error::Limit)
    );
}

#[test]
fn invalid_primitive_spellings_and_slot_overflow_are_literals() {
    let train = b"{\"long_field_name\":0}\n";
    let artifact = schema::fit(&[train]).unwrap();
    for bad in [
        b"+1".as_slice(),
        b"01",
        b"1.",
        b"1e+",
        b"NaN",
        b"TRUE",
        b"\"\\q\"",
        b"\"\x01\"",
        b"[1]",
        b"{}",
    ] {
        let target = [b"{\"long_field_name\":".as_slice(), bad, b"}\n"].concat();
        let encoded = schema::encode(&artifact, &target).unwrap();
        assert_eq!(encoded[48], 0);
        assert_eq!(
            schema::decode(&artifact, &encoded, target.len()).unwrap(),
            target
        );
    }
    let many = format!(
        "{{{}}}\n",
        (0..schema::MAX_SLOTS + 1)
            .map(|i| format!("\"k{i}\":0"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let unsupported = schema::fit(&[many.as_bytes()]).unwrap();
    assert_eq!(
        u16::from_le_bytes(unsupported[10..12].try_into().unwrap()),
        0
    );
}
