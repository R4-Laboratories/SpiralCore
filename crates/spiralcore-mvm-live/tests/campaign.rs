use spiralcore_mvm_adapter::live::HandshakeRole;
use spiralcore_mvm_live::{
    campaign,
    config::{Arm, Pair, Plan, Published, Stage},
};
use std::{os::unix::net::UnixStream, path::Path};

fn run(pair: &Pair, cache: &Path) -> (serde_json::Value, Vec<Published>) {
    let (sent, published, _) = run_receipts(pair, cache);
    (sent, published)
}

fn run_receipts(
    pair: &Pair,
    cache: &Path,
) -> (serde_json::Value, Vec<Published>, Vec<serde_json::Value>) {
    let (a, b) = UnixStream::pair().unwrap();
    let (sent, received, records) = std::thread::scope(|scope| {
        let receiver = scope.spawn(|| {
            let mut records = Vec::new();
            let result = campaign::receiver(
                &pair.receiver,
                cache,
                b,
                HandshakeRole::Acceptor,
                &mut records,
            )
            .unwrap();
            assert!(!records.is_empty());
            result
        });
        let mut records = Vec::new();
        let sent =
            campaign::sender(&pair.sender, a, HandshakeRole::Initiator, &mut records).unwrap();
        assert!(!records.is_empty());
        (sent, receiver.join().unwrap(), records)
    });
    assert_eq!(sent["event"], "sender_done");
    assert_eq!(received["event"], "receiver_done");
    for (send, receive) in [
        ("envelope_bytes_sent", "envelope_bytes_received"),
        ("envelope_bytes_received", "envelope_bytes_sent"),
        ("framing_bytes_sent", "framing_bytes_received"),
        ("framing_bytes_received", "framing_bytes_sent"),
        ("frames_sent", "frames_received"),
        ("frames_received", "frames_sent"),
    ] {
        assert_eq!(sent["counters"][send], received["counters"][receive]);
    }
    let published = serde_json::from_value(received["published"].clone()).unwrap();
    let records = serde_json::Deserializer::from_slice(&records)
        .into_iter()
        .map(Result::unwrap)
        .collect();
    (sent, published, records)
}

#[test]
fn mixed_persistent_campaign_crosses_real_stream_and_reopens_exact_bytes() {
    let store = tempfile::tempdir().unwrap();
    let cache = store.path().join("cache");
    let plan = Plan {
        stage: Stage::Two,
        arm: Arm::Warm,
        size: 4096,
        trial: 4,
        bandwidth_profile: Default::default(),
    };
    let pair = Pair::provision(plan.clone()).unwrap();
    let (sent, published) = run(&pair, &cache);
    assert_eq!(published.len(), 11);
    assert_eq!(sent["operations"], 11);
    let mut verified = Vec::new();
    campaign::verify(&plan, &cache, &published, &mut verified).unwrap();
    assert!(!verified.is_empty());
}

#[test]
fn raw_and_whole_baselines_use_the_same_persistent_verified_channel() {
    for arm in [Arm::Raw, Arm::Whole] {
        let store = tempfile::tempdir().unwrap();
        let pair = Pair::provision(Plan {
            stage: Stage::One,
            arm,
            size: 4096,
            trial: 5,
            bandwidth_profile: Default::default(),
        })
        .unwrap();
        let (sent, published) = run(&pair, &store.path().join("cache"));
        assert_eq!(sent["operations"], 7);
        assert_eq!(published.len(), 7);
    }
}

#[test]
fn same_epoch_new_socket_and_endpoint_recover_published_objects_with_fresh_envelopes() {
    let store = tempfile::tempdir().unwrap();
    let cache = store.path().join("cache");
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Warm,
        size: 4096,
        trial: 6,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let (first, published) = run(&pair, &cache);
    // Both actors and the Endpoint have dropped. A new canonical stream uses
    // the same still-valid epoch/grant and fresh envelope IDs, retaining only
    // durable publications. This does not claim a durable replay inbox.
    let (again, recovered) = run(&pair, &cache);
    assert_eq!(
        serde_json::to_value(&published).unwrap(),
        serde_json::to_value(&recovered).unwrap()
    );
    assert!(
        again["counters"]["envelope_bytes_sent"].as_u64().unwrap()
            < first["counters"]["envelope_bytes_sent"].as_u64().unwrap()
    );
    let publication = &recovered[0];
    let path = cache
        .join(spiralcore_transfer::hex(&publication.namespace))
        .join(format!(
            "{}.object",
            spiralcore_transfer::hex(&publication.object)
        ));
    let mut bytes = std::fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    std::fs::write(&path, bytes).unwrap();
    assert!(campaign::verify(&pair.sender.plan, &cache, &recovered, &mut Vec::new()).is_err());
}

#[test]
fn combined_arms_share_payloads_and_reopen_exact_bytes() {
    let mut baseline = None;
    for name in ["whole-mixed", "combined", "combined-cpu"] {
        let arm: Arm = serde_json::from_value(serde_json::json!(name)).unwrap();
        let pair = Pair::provision(Plan {
            stage: Stage::Two,
            arm,
            size: 1 << 20,
            trial: 0,
            bandwidth_profile: Default::default(),
        })
        .unwrap();
        let store = tempfile::tempdir().unwrap();
        let cache = store.path().join("cache");
        let (sent, published, rows) = run_receipts(&pair, &cache);
        let payloads: Vec<_> = published
            .iter()
            .filter(|p| p.kind != "schema" && p.kind != "runtime-checkpoint")
            .map(|p| (p.kind.clone(), p.object, p.length))
            .collect();
        assert_eq!(payloads.len(), 8);
        assert!(payloads.iter().all(|(_, _, length)| *length == 1_047_202));
        if let Some(expected) = &baseline {
            assert_eq!(&payloads, expected);
        } else {
            baseline = Some(payloads);
        }
        assert_eq!(
            sent["operations"],
            if name == "whole-mixed" { 8 } else { 10 }
        );
        campaign::verify(&pair.sender.plan, &cache, &published, &mut Vec::new()).unwrap();
        if name != "whole-mixed" {
            let operations: Vec<_> = rows
                .iter()
                .filter(|r| r["event"] == "sender_operation")
                .collect();
            let get = |kind: &str| *operations.iter().find(|r| r["kind"] == kind).unwrap();
            assert_eq!(get("generated")["mode"], "RuntimeV1");
            assert_eq!(get("residual")["mode"], "SparseV1");
            for repeat in ["repeat", "opaque-edited-repeat"] {
                assert_eq!(get(repeat)["selection"]["exact_repeat_probe"], true);
            }
            let policy = if name == "combined-cpu" {
                "cpu-conscious"
            } else {
                "exhaustive"
            };
            assert_eq!(get("unrelated")["selection"]["selection_policy"], policy);
            assert_eq!(
                get("unrelated")["selection"]["policy_skipped"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
                name == "combined"
            );
            for kind in ["schema-fit", "schema-drift"] {
                assert_eq!(get(kind)["mode"], "SchemaZstdV1");
                assert!(
                    get(kind)["selection"]["policy_skipped"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
            }
        }
        if name == "combined-cpu" {
            let (again, recovered) = run(&pair, &cache);
            assert_eq!(
                serde_json::to_value(&published).unwrap(),
                serde_json::to_value(recovered).unwrap()
            );
            assert!(
                again["counters"]["envelope_bytes_sent"].as_u64().unwrap()
                    < sent["counters"]["envelope_bytes_sent"].as_u64().unwrap()
            );
        }
    }
}

#[test]
fn mixed_public_fixtures_match_retained_gateway_identities() {
    // 2026-11-02-cpu-conscious/compact-receipts.tar.gz:
    // candidate-evidence/campaign/fixtures-0/manifest.json, near1m tier.
    let expected = [
        "2ced35de02b6adc2ba856a16a95bf0757fea42b5fc7f2c3f90ddf6960858a8b6",
        "ca02bc9131dfdfb15331aad2f45eb459ddb32d1a1c278edb9ca6dcc38de7db91",
        "0ccc92643d5a8e54196dc945cba834dfe99f8156f8e1ce9ec6a3a09c4acd5762",
        "adf81bac62cb24f56ee7b0d51a0b6a796771a77cbec209cee682ecc9077f0050",
        "adf81bac62cb24f56ee7b0d51a0b6a796771a77cbec209cee682ecc9077f0050",
        "9a43e5fb12e719863c0d9493f20dc7a532125d4c7d4a5b9f7dbc832fe42ca253",
        "d1f16de525d3c33b522ac762a849dd0d0451085d9a955e2a4b9d15c48c85caad",
        "d1f16de525d3c33b522ac762a849dd0d0451085d9a955e2a4b9d15c48c85caad",
        "b4fa64a2ae71b8f447a89a9664d52fb89ee545f791c59d80ff02d862505f76bb",
        "0177b0f91cb2fb073434af9525bb24137eb84e5ebf0529cb01dd20acdb51b200",
    ];
    let cases = spiralcore_mvm_live::fixtures::mixed_objects(1 << 20, 0, true).unwrap();
    assert_eq!(cases.len(), expected.len());
    for (case, expected) in cases.iter().zip(expected) {
        assert_eq!(
            spiralcore_transfer::hex(&spiralcore_transfer::object_id(&case.bytes)),
            expected,
            "{}",
            case.kind
        );
    }
    assert!(spiralcore_mvm_live::fixtures::mixed_recipe(4096, 0).is_err());
    assert!(spiralcore_mvm_live::fixtures::mixed_recipe(1 << 20, 3).is_err());
}

#[test]
fn canonical_service_bootstrap_reserves_private_receipt_keys_and_shared_routes() {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Warm,
        size: 4096,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let sender = pair
        .sender
        .service
        .as_ref()
        .expect("canonical service profile");
    let receiver = pair.receiver.service.as_ref().unwrap();
    assert_eq!(sender.profile, "canonical-ephemeral-v1");
    assert!(sender.incoming_control.signing.is_none());
    assert!(sender.outgoing_control.signing.is_some());
    assert_eq!(
        sender.outgoing_control.binding.claims.route_id,
        pair.sender
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id
    );
    assert_eq!(
        receiver.outgoing_control.binding.claims.route_id,
        pair.receiver
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id
    );
    assert_ne!(
        sender.outgoing_control.key_id,
        pair.sender.outgoing.as_ref().unwrap().key_id
    );
    assert_eq!(
        sender.peer_receipt_verification,
        ed25519_dalek::SigningKey::from_bytes(&receiver.local_receipt_signing)
            .verifying_key()
            .to_bytes()
    );
}

#[test]
fn service_campaign_counts_destination_receipts_in_both_directions() {
    let store = tempfile::tempdir().unwrap();
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 2,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let (sent, _, _) = run_receipts(&pair, &store.path().join("cache"));
    assert_eq!(sent["service_profile"], "canonical-ephemeral-v1");
    let calls = sent["service"]["application_calls"].as_u64().unwrap();
    assert!(calls > 0);
    assert_eq!(sent["counters"]["frames_sent"].as_u64().unwrap(), calls * 2);
    assert_eq!(
        sent["counters"]["frames_received"].as_u64().unwrap(),
        calls * 2
    );
}

#[test]
fn qualified_16m_whole_mixed_service_session_reopens_exactly() {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::WholeMixed,
        size: 16 << 20,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let store = tempfile::tempdir().unwrap();
    let (sent, published, _) = run_receipts(&pair, &store.path().join("cache"));
    assert_eq!(published.len(), 8);
    assert!(published.iter().all(|p| p.length == 14_851_384));
    let calls = sent["service"]["application_calls"].as_u64().unwrap();
    assert!(calls > 512 && calls <= 2048);
    assert_eq!(sent["service"]["identity_tier"], "broker-owned");
    assert_eq!(sent["service"]["witness_tier"], "native-16m-v1");
    assert_eq!(sent["counters"]["frames_sent"].as_u64().unwrap(), calls * 2);
    assert_eq!(
        sent["counters"]["frames_received"].as_u64().unwrap(),
        calls * 2
    );
    eprintln!(
        "NATIVE_16M_WHOLE_MIXED_UNIT {}",
        serde_json::to_string(&sent).unwrap()
    );
    eprintln!(
        "QUALIFICATION_SCOPE paired socket unit in one process; not guest/WAN performance; process CPU includes both endpoint threads"
    );
}

#[test]
fn shared_broker_dispatches_both_endpoints_without_clock_or_fidelity_loss() {
    let store = tempfile::tempdir().unwrap();
    let cache = store.path().join("cache");
    let mut pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Warm,
        size: 4096,
        trial: 19,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    // This test hosts both sides of a Call in one accounting domain. The
    // synthetic grant explicitly admits both local role charges; production
    // provisioning continues to grant one outstanding Call per remote broker.
    let authority = ed25519_dalek::SigningKey::from_bytes(&[91; 32]);
    for bootstrap in [&mut pair.sender, &mut pair.receiver] {
        let service = bootstrap.service.as_mut().unwrap();
        for material in [
            bootstrap.incoming.as_mut().unwrap(),
            bootstrap.outgoing.as_mut().unwrap(),
            &mut service.incoming_control,
            &mut service.outgoing_control,
        ] {
            material.capability.grant.max_outstanding_calls = 2;
            material.capability =
                mvm_core::mailbox::sign_capability(material.capability.grant.clone(), &authority)
                    .unwrap();
            material.binding = mvm_core::mailbox::transport::sign_transport_binding(
                material.binding.claims.clone(),
                &authority,
            )
            .unwrap();
            material.authority = authority.verifying_key().to_bytes();
        }
    }
    let broker = campaign::broker_context().unwrap();
    let (a, b) = UnixStream::pair().unwrap();
    let (sent, received) = std::thread::scope(|scope| {
        let receiver = scope.spawn(|| {
            campaign::receiver_with_broker(
                &pair.receiver,
                &broker,
                &cache,
                b,
                HandshakeRole::Acceptor,
                &mut Vec::new(),
            )
            .unwrap()
        });
        let sent = campaign::sender_with_broker(
            &pair.sender,
            &broker,
            a,
            HandshakeRole::Initiator,
            &mut Vec::new(),
        )
        .unwrap();
        (sent, receiver.join().unwrap())
    });
    assert_eq!(
        sent["service"]["ordering"],
        "stable-scope-increasing-sequence"
    );
    assert_eq!(
        received["service"]["broker_clock"],
        "shared-serialized-dispatch"
    );
    assert_eq!(
        sent["counters"]["envelope_bytes_sent"],
        received["counters"]["envelope_bytes_received"]
    );
    let published: Vec<Published> = serde_json::from_value(received["published"].clone()).unwrap();
    campaign::verify(&pair.sender.plan, &cache, &published, &mut Vec::new()).unwrap();
}
