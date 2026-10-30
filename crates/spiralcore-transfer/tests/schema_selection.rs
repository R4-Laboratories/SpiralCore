use spiralcore_transfer::protocol::{Mode, Offer};
use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Endpoint},
    object_id, schema,
    schema_selection::PreparedSelector,
    wan::{self, BenchAuthority},
    zstd_whole, Error,
};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "schema-selection-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
    fn endpoint(&self) -> Endpoint<BenchAuthority> {
        Endpoint::open_with_policy(
            &self.0,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn exchange(
    ep: &mut Endpoint<BenchAuthority>,
    scope: [u8; 32],
    frames: &[spiralcore_transfer::protocol::Frame],
    bytes: &mut u64,
) -> spiralcore_transfer::Result<Vec<spiralcore_transfer::protocol::Frame>> {
    *bytes += wan::encode_batch(frames)?.len() as u64;
    let mut replies = Vec::new();
    for f in frames {
        let r = ep
            .handle(&wan::context(scope), f, 0)
            .unwrap_or_else(|e| f.reply(127, (e as u16).to_le_bytes().to_vec()));
        let terminal = matches!(r.kind, 103 | 127);
        replies.push(r);
        if terminal {
            break;
        }
    }
    *bytes += wan::encode_batch(&replies)?.len() as u64 + 16;
    Ok(replies)
}
fn random(n: usize) -> Vec<u8> {
    let mut state = 0x5ef3_9ab2_3020_a1dfu64;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

#[test]
fn whole_zstd_mode_is_explicit_and_requires_no_artifact() {
    let mut wire = Offer::cold(b"whole object", Mode::Raw).encode().unwrap();
    wire[0] = 9;
    let decoded = Offer::decode(&wire).expect("Mode9 must admit a bounded whole-object frame");
    assert_eq!(format!("{:?}", decoded.mode), "ZstdWholeV1");
    assert!(decoded.base.is_none());
    let mut forbidden = decoded.clone();
    forbidden.base = Some([7; 32]);
    assert!(forbidden.encode().is_err());
    let mut too_large = decoded;
    too_large.len = 16 * 1024 * 1024 + 1;
    assert!(too_large.encode().is_err());
}

#[test]
fn whole_contexts_preserve_parameters_bytes_and_recover_after_bad_frames() {
    let mut encoder = zstd_whole::PreparedEncoder::new().unwrap();
    let mut decoder = zstd_whole::PreparedDecoder::new().unwrap();
    let targets = [
        Vec::new(),
        vec![0; 65537],
        random(65537),
        b"exact\0trailing\0\0".to_vec(),
    ];
    for target in targets.iter().cycle().take(8) {
        let encoded = encoder.encode(target).unwrap();
        assert_eq!(encoded, zstd_whole::encode(target).unwrap());
        assert_eq!(decoder.decode(&encoded, target.len()).unwrap(), *target);
        assert_eq!(zstd_whole::decode(&encoded, target.len()).unwrap(), *target);
        assert!(decoder.decode(&encoded, target.len() + 1).is_err());
        for end in [0, 1, 4, encoded.len() - 1] {
            assert!(decoder.decode(&encoded[..end], target.len()).is_err());
        }
        let mut corrupt = encoded.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(decoder.decode(&corrupt, target.len()).is_err());
        let mut extra = encoded.clone();
        extra.extend_from_slice(&encoded);
        assert!(decoder.decode(&extra, target.len()).is_err());
        for flag in [1, 2, 8, 16] {
            let mut bad = encoded.clone();
            bad[4] |= flag;
            assert!(decoder.decode(&bad, target.len()).is_err());
        }
        let mut no_checksum = encoded.clone();
        no_checksum[4] &= !4;
        assert!(decoder.decode(&no_checksum, target.len()).is_err());
        assert_eq!(decoder.decode(&encoded, target.len()).unwrap(), *target);
    }
    assert!(encoder.retained_bytes() as u64 <= encoder.retained_charge());
    assert!(decoder.retained_bytes() as u64 <= decoder.retained_charge());
    assert!(zstd_whole::encoded_cap(client::MAX_BATCHED_OBJECT + 1).is_err());
}

#[test]
fn prepared_selection_is_minimum_charges_seed_and_matches_serialized_transfer() {
    let artifact =
        schema::fit(&[b"{\"long_identifier\":0,\"status_message\":\"training-value\"}\n"]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let target_record = b"{\"long_identifier\":17,\"status_message\":\"held-out-value\"}\n";
    let targets = [
        Vec::new(),
        b"tiny".to_vec(),
        random(65537),
        target_record.repeat(4096),
    ];
    for (case, target) in targets.iter().enumerate() {
        for ready in [false, true] {
            for warm_probe in [false, true] {
                let t = Temp::new();
                let mut ep = t.endpoint();
                ep.set_codec_cache_enabled(true).unwrap();
                let scope = [1; 32];
                let mut setup_bytes = 0;
                if ready {
                    client::transfer_batched(&artifact, Mode::Raw, None, [7; 32], false, |f| {
                        exchange(&mut ep, scope, f, &mut setup_bytes)
                    })
                    .unwrap();
                }
                let plan = selector.prepare(target, ready, warm_probe).unwrap();
                let receipt = plan.selection().clone();
                let best = receipt
                    .candidates
                    .iter()
                    .min_by_key(|c| c.total_http_body_bytes)
                    .unwrap();
                assert_eq!(receipt.selected_mode, best.mode);
                assert_eq!(
                    receipt.candidates[2].setup_http_body_bytes,
                    if ready {
                        0
                    } else {
                        368 + artifact.len() as u64 + 133
                    }
                );
                assert_eq!(
                    receipt.needs_schema_seed,
                    !ready && receipt.selected_mode == Mode::SchemaZstdV1
                );
                if receipt.needs_schema_seed {
                    client::transfer_batched(&artifact, Mode::Raw, None, [7; 32], false, |f| {
                        exchange(&mut ep, scope, f, &mut setup_bytes)
                    })
                    .unwrap();
                }
                let mut actual = 0;
                let stats =
                    client::transfer_schema_selection_batched(plan, [10 + case as u8; 32], |f| {
                        exchange(&mut ep, scope, f, &mut actual)
                    })
                    .unwrap();
                assert_eq!(stats.mode, receipt.selected_mode);
                assert_eq!(actual, best.target_http_body_bytes);
                if !ready {
                    assert_eq!(actual + setup_bytes, best.total_http_body_bytes);
                }
                assert_eq!(
                    ep.read_object(&wan::context(scope), object_id(target))
                        .unwrap(),
                    *target
                );
                drop(ep);
                let mut reopened = t.endpoint();
                assert_eq!(
                    reopened
                        .read_object(&wan::context(scope), object_id(target))
                        .unwrap(),
                    *target
                );
            }
        }
    }
    assert_eq!(selector.artifact(), artifact);
    assert_eq!(selector.artifact_id(), object_id(&artifact));
    assert!(selector.retained_bytes() as u64 <= selector.retained_charge());
}

#[test]
fn unseeded_claimed_warm_schema_is_rejected_and_whole_cache_is_scoped() {
    let key: String = random(2048)
        .iter()
        .map(|b| char::from(b'a' + b % 26))
        .collect();
    let row = format!("{{\"{key}\":7}}\n").into_bytes();
    let artifact = schema::fit(&[&row]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let plan = selector.prepare(&row, true, false).unwrap();
    assert_eq!(plan.selection().selected_mode, Mode::SchemaZstdV1);
    let t = Temp::new();
    let mut ep = t.endpoint();
    ep.set_codec_cache_enabled(true).unwrap();
    let mut bytes = 0;
    assert!(matches!(
        client::transfer_schema_selection_batched(plan, [1; 32], |f| exchange(
            &mut ep, [1; 32], f, &mut bytes
        )),
        Err(Error::Missing)
    ));
    assert!(ep
        .read_object(&wan::context([1; 32]), object_id(&row))
        .is_err());
    let cold = selector.prepare(&row, false, false).unwrap();
    assert_ne!(cold.selection().selected_mode, Mode::SchemaZstdV1);
    assert!(!cold.selection().needs_schema_seed);
    let mut encoder = zstd_whole::PreparedEncoder::new().unwrap();
    for (i, scope) in [[1; 32], [1; 32], [2; 32]].into_iter().enumerate() {
        let target = vec![i as u8; 65537];
        let stats = client::transfer_prepared_zstd_whole_batched(
            &mut encoder,
            &target,
            [30 + i as u8; 32],
            false,
            |f| exchange(&mut ep, scope, f, &mut bytes),
        )
        .unwrap();
        assert_eq!(stats.mode, Mode::ZstdWholeV1);
        assert_eq!(
            ep.read_object(&wan::context(scope), object_id(&target))
                .unwrap(),
            target
        );
    }
    let cache = ep.codec_cache_stats();
    assert_eq!(cache.preparations, 2);
    assert_eq!(cache.hits, 1);
    assert_eq!(cache.entries, 2);
}

#[test]
fn exact_candidate_costs_and_raw_tie_match_independent_frame_lengths() {
    let artifact = schema::fit(&[b"{\"identifier\":0}\n"]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let mut found_tie = false;
    for len in 0..129 {
        let target = vec![0; len];
        let whole = zstd_whole::encode(&target).unwrap();
        let schema = spiralcore_transfer::schema_zstd::encode(&artifact, &target).unwrap();
        for probe in [false, true] {
            let plan = selector.prepare(&target, true, probe).unwrap();
            let c = &plan.selection().candidates;
            let common = 368 + u64::from(probe) * 40;
            assert_eq!(
                c[0].target_http_body_bytes,
                common + len as u64 + len.div_ceil(spiralcore_transfer::MAX_CHUNK) as u64 * 133
            );
            assert_eq!(
                c[1].target_http_body_bytes,
                common
                    + whole.len() as u64
                    + whole.len().div_ceil(spiralcore_transfer::MAX_CHUNK) as u64 * 134
            );
            assert_eq!(
                c[2].target_http_body_bytes,
                common
                    + 32
                    + schema.len() as u64
                    + schema.len().div_ceil(spiralcore_transfer::MAX_CHUNK) as u64 * 134
            );
            if c[0].total_http_body_bytes == c[1].total_http_body_bytes {
                found_tie = true;
                assert_eq!(plan.selection().selected_mode, Mode::Raw);
            }
        }
    }
    assert!(
        found_tie,
        "fixture must exercise an actual Raw/Whole byte tie"
    );
}

#[test]
fn whole_receiver_rejects_corruption_then_reuses_bounded_contexts() {
    use spiralcore_transfer::protocol::Frame;
    let t = Temp::new();
    let mut limits = wan::limits();
    limits.resident_bytes = 20 << 20;
    let mut ep = Endpoint::open_with_policy(
        &t.0,
        limits.clone(),
        BenchAuthority,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    ep.set_codec_cache_enabled(true).unwrap();
    let target = vec![0; 65537];
    let mut encoded = zstd_whole::encode(&target).unwrap();
    *encoded.last_mut().unwrap() ^= 1;
    let begin = Frame {
        kind: 1,
        transfer: [50; 32],
        seq: 0,
        body: Offer::cold(&target, Mode::ZstdWholeV1).encode().unwrap(),
    };
    ep.handle(&wan::context([1; 32]), &begin, 0).unwrap();
    let mut body = vec![9];
    body.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
    body.push(1);
    body.extend_from_slice(&encoded);
    ep.handle(
        &wan::context([1; 32]),
        &Frame {
            kind: 2,
            transfer: [50; 32],
            seq: 1,
            body,
        },
        0,
    )
    .unwrap();
    assert!(matches!(
        ep.handle(
            &wan::context([1; 32]),
            &Frame {
                kind: 3,
                transfer: [50; 32],
                seq: 2,
                body: vec![]
            },
            0
        ),
        Err(Error::Integrity)
    ));
    assert!(ep
        .read_object(&wan::context([1; 32]), object_id(&target))
        .is_err());
    let mut encoder = zstd_whole::PreparedEncoder::new().unwrap();
    let mut bytes = 0;
    for index in 1..=12 {
        let data = vec![index; 65537];
        client::transfer_prepared_zstd_whole_batched(
            &mut encoder,
            &data,
            [80 + index; 32],
            false,
            |f| exchange(&mut ep, [index; 32], f, &mut bytes),
        )
        .unwrap();
        assert_eq!(
            ep.read_object(&wan::context([index; 32]), object_id(&data))
                .unwrap(),
            data
        );
        assert!(ep.codec_cache_stats().entries <= 8);
    }
    assert!(ep.codec_cache_stats().evictions > 0);
    let mut denied = wan::context([1; 32]);
    denied.policy_epoch += 1;
    assert!(matches!(
        ep.handle(&denied, &begin, 0),
        Err(Error::Unauthorized)
    ));
    drop(ep);
    let mut reopened =
        Endpoint::open_with_policy(&t.0, limits, BenchAuthority, ColdGraphPolicy::Disabled)
            .unwrap();
    assert_eq!(reopened.codec_cache_stats().entries, 0);
    assert_eq!(
        reopened
            .read_object(&wan::context([12; 32]), object_id(&vec![12; 65537]))
            .unwrap(),
        vec![12; 65537]
    );
}

#[test]
fn whole_codec_accepts_exact_sixteen_mib_and_rejects_larger_before_encode() {
    let mut target = vec![0; client::MAX_BATCHED_OBJECT];
    let encoded = zstd_whole::encode(&target).unwrap();
    assert_eq!(zstd_whole::decode(&encoded, target.len()).unwrap(), target);
    target.push(0);
    assert!(matches!(zstd_whole::encode(&target), Err(Error::Limit)));
    let artifact = schema::fit(&[]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    assert!(matches!(
        selector.prepare(&target, false, false),
        Err(Error::Limit)
    ));
}

#[test]
fn lazy_schema_seed_is_paid_once_then_ready_schema_commits_exactly() {
    let keys: Vec<_> = (0..8)
        .map(|i| format!("stable_schema_measurement_identifier_{i}"))
        .collect();
    let record = |values: &[u64]| {
        let fields: Vec<_> = keys
            .iter()
            .zip(values)
            .map(|(k, v)| format!("\"{k}\":{v}"))
            .collect();
        format!("{{{}}}\n", fields.join(","))
    };
    let training = record(&[0; 8]);
    let artifact = schema::fit(&[training.as_bytes()]).unwrap();
    let mut state = 0x8434_93ae_b2af_1d57u64;
    let mut target = Vec::new();
    for _ in 0..4096 {
        let mut values = [0; 8];
        for value in &mut values {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *value = state;
        }
        target.extend_from_slice(record(&values).as_bytes());
    }
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let t = Temp::new();
    let mut ep = t.endpoint();
    ep.set_codec_cache_enabled(true).unwrap();
    let scope = [8; 32];
    let cold = selector.prepare(&target, false, false).unwrap();
    let receipt = cold.selection().clone();
    assert_eq!(receipt.selected_mode, Mode::SchemaZstdV1, "{receipt:?}");
    assert!(receipt.needs_schema_seed);
    let mut paid = 0;
    client::transfer_batched(&artifact, Mode::Raw, None, [70; 32], false, |f| {
        exchange(&mut ep, scope, f, &mut paid)
    })
    .unwrap();
    client::transfer_schema_selection_batched(cold, [71; 32], |f| {
        exchange(&mut ep, scope, f, &mut paid)
    })
    .unwrap();
    assert_eq!(paid, receipt.candidates[2].total_http_body_bytes);
    assert_eq!(
        ep.read_object(&wan::context(scope), object_id(&target))
            .unwrap(),
        target
    );
    // A distinct exact target avoids a target-cache hit while retaining the schema.
    target.extend_from_slice(record(&[123; 8]).as_bytes());
    let warm = selector.prepare(&target, true, false).unwrap();
    let receipt = warm.selection().clone();
    assert_eq!(receipt.selected_mode, Mode::SchemaZstdV1);
    assert!(!receipt.needs_schema_seed);
    assert_eq!(receipt.candidates[2].setup_http_body_bytes, 0);
    let mut paid = 0;
    client::transfer_schema_selection_batched(warm, [72; 32], |f| {
        exchange(&mut ep, scope, f, &mut paid)
    })
    .unwrap();
    assert_eq!(paid, receipt.candidates[2].total_http_body_bytes);
    assert_eq!(
        ep.read_object(&wan::context(scope), object_id(&target))
            .unwrap(),
        target
    );
    drop(ep);
    assert_eq!(
        t.endpoint()
            .read_object(&wan::context(scope), object_id(&target))
            .unwrap(),
        target
    );
}
