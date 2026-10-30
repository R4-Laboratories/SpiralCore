use spiralcore_transfer::{schema, schema_zstd, zstd_dictionary, Error};

fn training() -> Vec<Vec<u8>> {
    (0..128).map(|i| format!("{{\"long_identifier\":{i},\"status_message\":\"training-{i}\",\"enabled_flag\":true}}\n").into_bytes()).collect()
}

#[test]
fn prepared_schema_owns_artifact_and_preserves_existing_stream_bytes() {
    let rows = training();
    let refs: Vec<&[u8]> = rows.iter().map(Vec::as_slice).collect();
    let mut artifact = schema::fit(&refs).unwrap();
    let frozen = artifact.clone();
    let prepared = schema::PreparedSchema::new(&artifact).unwrap();
    artifact.fill(255);
    assert_eq!(prepared.artifact(), frozen);
    assert_eq!(
        prepared.artifact_id(),
        spiralcore_transfer::object_id(&frozen)
    );
    for target in [b"".as_slice(), b"\0\xff\n", refs[0], refs[77]] {
        let expected = schema::encode(&frozen, target).unwrap();
        assert_eq!(prepared.encode(target).unwrap(), expected);
        assert_eq!(prepared.decode(&expected, target.len()).unwrap(), target);
    }
    assert!(prepared.retained_bytes() as u64 <= prepared.retained_charge());
}

#[test]
fn schema_zstd_reuses_contexts_and_preserves_exact_values_and_fallback() {
    let rows = training();
    let refs: Vec<&[u8]> = rows.iter().map(Vec::as_slice).collect();
    let artifact = schema::fit(&refs).unwrap();
    let mut encoder = schema_zstd::PreparedEncoder::new(&artifact).unwrap();
    let mut decoder = schema_zstd::PreparedDecoder::new(&artifact).unwrap();
    let batch = rows.concat();
    for target in [
        b"".as_slice(),
        refs[1],
        batch.as_slice(),
        b"\xff\0binary",
        refs[31],
        batch.as_slice(),
    ] {
        let encoded = encoder.encode(target).unwrap();
        assert_eq!(encoded, schema_zstd::encode(&artifact, target).unwrap());
        assert_eq!(decoder.decode(&encoded, target.len()).unwrap(), target);
        assert_eq!(
            schema_zstd::decode(&artifact, &encoded, target.len()).unwrap(),
            target
        );
        assert!(encoded.len() <= schema_zstd::encoded_cap(target.len()).unwrap());
        assert!(encoder.retained_bytes() as u64 <= encoder.retained_charge());
        assert!(decoder.retained_bytes() as u64 <= decoder.retained_charge());
    }
}

#[test]
fn prepared_dictionary_matches_fresh_bytes_after_alternating_inputs_and_errors() {
    let rows = training();
    let refs: Vec<&[u8]> = rows.iter().map(Vec::as_slice).collect();
    let mut artifact = zstd_dictionary::train(&refs, 2048).unwrap();
    let frozen = artifact.clone();
    let mut encoder = zstd_dictionary::PreparedEncoder::new(&artifact).unwrap();
    let mut decoder = zstd_dictionary::PreparedDecoder::new(&artifact).unwrap();
    artifact.fill(0);
    assert_eq!(encoder.artifact(), frozen);
    let batch = rows.concat();
    let large = batch.repeat(32);
    for target in [
        refs[1],
        batch.as_slice(),
        large.as_slice(),
        b"\xff\0binary",
        b"",
        refs[31],
        batch.as_slice(),
    ] {
        let encoded = encoder.encode(target).unwrap();
        assert_eq!(encoded, zstd_dictionary::encode(&frozen, target).unwrap());
        assert_eq!(decoder.decode(&encoded, target.len()).unwrap(), target);
        let mut bad = encoded.clone();
        let last = bad.len() - 1;
        bad[last] ^= 1;
        assert_eq!(decoder.decode(&bad, target.len()), Err(Error::Integrity));
        assert_eq!(decoder.decode(&encoded, target.len()).unwrap(), target);
        assert!(encoder.retained_bytes() as u64 <= encoder.retained_charge());
        assert!(decoder.retained_bytes() as u64 <= decoder.retained_charge());
    }
}

fn frame(bytes: &[u8], checksum: bool, content_size: bool) -> Vec<u8> {
    use zstd::zstd_safe::{compress_bound, CCtx, CParameter};
    let mut ctx = CCtx::try_create().unwrap();
    ctx.set_parameter(CParameter::CompressionLevel(3)).unwrap();
    ctx.set_parameter(CParameter::ChecksumFlag(checksum))
        .unwrap();
    ctx.set_parameter(CParameter::ContentSizeFlag(content_size))
        .unwrap();
    let mut encoded = Vec::with_capacity(compress_bound(bytes.len()));
    ctx.compress2(&mut encoded, bytes).unwrap();
    encoded
}

#[test]
fn schema_zstd_rejects_hostile_frames_then_decodes_a_valid_frame() {
    let rows = training();
    let refs: Vec<&[u8]> = rows.iter().map(Vec::as_slice).collect();
    let artifact = schema::fit(&refs).unwrap();
    let raw = schema::encode(&artifact, refs[1]).unwrap();
    let valid = schema_zstd::encode(&artifact, refs[1]).unwrap();
    let mut decoder = schema_zstd::PreparedDecoder::new(&artifact).unwrap();
    for cut in 0..valid.len() {
        assert!(decoder.decode(&valid[..cut], refs[1].len()).is_err());
    }
    for bad in [
        frame(&raw, false, true),
        frame(&raw, true, false),
        [valid.as_slice(), valid.as_slice()].concat(),
        [valid.as_slice(), &[0]].concat(),
    ] {
        assert!(decoder.decode(&bad, refs[1].len()).is_err());
    }
    let mut damaged = valid.clone();
    let last = damaged.len() - 1;
    damaged[last] ^= 1;
    assert_eq!(
        decoder.decode(&damaged, refs[1].len()),
        Err(Error::Integrity)
    );
    let mut dictionary_flag = valid.clone();
    dictionary_flag[4] |= 1;
    assert_eq!(
        decoder.decode(&dictionary_flag, refs[1].len()),
        Err(Error::Malformed)
    );
    assert_eq!(
        decoder.decode(&frame(&[0; 49], true, true), 0),
        Err(Error::Limit)
    );
    assert_eq!(
        decoder.decode(&frame(&[0; 47], true, true), 0),
        Err(Error::Limit)
    );
    assert_eq!(
        decoder.decode(&[0x28, 0xb5, 0x2f, 0xfd, 4, 255], refs[1].len()),
        Err(Error::Limit)
    );
    assert_eq!(decoder.decode(&valid, refs[1].len()).unwrap(), refs[1]);
    let other = schema::fit(&[b"{\"other\":1}\n"]).unwrap();
    assert_eq!(
        schema_zstd::decode(&other, &valid, refs[1].len()),
        Err(Error::Integrity)
    );
}

#[test]
fn target_specific_intermediate_cap_covers_maximum_literal_record_overhead() {
    let artifact = schema::fit(&[]).unwrap();
    assert_eq!(schema::encoded_cap(0).unwrap(), 48);
    assert_eq!(schema::encoded_cap(1).unwrap(), 54);
    assert_eq!(
        schema::encoded_cap(schema::MAX_TARGET).unwrap(),
        schema::MAX_ENCODED
    );
    assert_eq!(
        schema::encoded_cap(schema::MAX_TARGET + 1),
        Err(Error::Limit)
    );
    let target: Vec<u8> = (0..schema::MAX_TARGET)
        .map(|i| if i % 256 == 255 { b'\n' } else { b'x' })
        .collect();
    let raw = schema::encode(&artifact, &target).unwrap();
    assert_eq!(raw.len(), schema::MAX_ENCODED);
    assert!(raw.len() > schema::MAX_TARGET);
    let encoded = schema_zstd::encode(&artifact, &target).unwrap();
    assert_eq!(
        schema_zstd::decode(&artifact, &encoded, target.len()).unwrap(),
        target
    );
}

#[test]
fn prepared_encoders_recover_after_rejected_oversize_input() {
    let rows = training();
    let refs: Vec<&[u8]> = rows.iter().map(Vec::as_slice).collect();
    let artifact = schema::fit(&refs).unwrap();
    let dictionary = zstd_dictionary::train(&refs, 2048).unwrap();
    let mut schema_encoder = schema_zstd::PreparedEncoder::new(&artifact).unwrap();
    let mut dict_encoder = zstd_dictionary::PreparedEncoder::new(&dictionary).unwrap();
    let huge = vec![0; schema::MAX_TARGET + 1];
    assert_eq!(schema_encoder.encode(&huge), Err(Error::Limit));
    assert_eq!(dict_encoder.encode(&huge), Err(Error::Limit));
    assert_eq!(
        schema_encoder.encode(refs[77]).unwrap(),
        schema_zstd::encode(&artifact, refs[77]).unwrap()
    );
    assert_eq!(
        dict_encoder.encode(refs[77]).unwrap(),
        zstd_dictionary::encode(&dictionary, refs[77]).unwrap()
    );
}
