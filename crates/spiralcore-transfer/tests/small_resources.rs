use spiralcore_transfer::{resource_profile::ResourceProfile, v1::Capabilities, Error};

#[test]
fn small_profile_is_explicit_bounded_and_never_negotiates_general_fallback() {
    let resources = ResourceProfile::SmallV1;
    assert_eq!(resources.max_target_len(), 16 * 1024);
    assert_eq!(resources.max_artifact_len(), 64 * 1024);
    assert_eq!(resources.max_prediction_len(), 64 * 1024);
    assert!(resources.operation_charge().unwrap() <= 8 * 1024 * 1024);
    let small = Capabilities::small_v1();
    assert_eq!(small.profile, 2);
    assert_eq!(small.max_object_bytes, 64 * 1024);
    assert_eq!(
        small.negotiate(&Capabilities::full()),
        Err(Error::Unsupported)
    );
    assert_eq!(
        Capabilities::full().negotiate(&small),
        Err(Error::Unsupported)
    );
    let agreed = small.negotiate(&small).unwrap();
    assert_eq!(agreed.capabilities().profile, 2);
    assert!(!agreed.supports(spiralcore_transfer::protocol::Mode::Dag));
    assert!(!agreed.supports(spiralcore_transfer::protocol::Mode::ZstdDictionaryV1));
}

struct Store(std::path::PathBuf);
impl Store {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "spiral-small-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn small_facade_all_modes_publish_exactly_with_one_aggregate_envelope() {
    use spiralcore_transfer::{
        schema,
        session::SelectionPolicy,
        shared_runtime::{OutputKind, Recipe, State},
        shared_transfer::PreparedPrediction,
        v1::{ObjectEndpoint, ObjectMode, ObjectSender},
        wan,
    };
    let resources = ResourceProfile::SmallV1;
    let negotiated = Capabilities::small_v1()
        .negotiate(&Capabilities::small_v1())
        .unwrap();
    let c = wan::context([177; 32]);
    let artifact = schema::fit(&[b"{\"value\":0}\n", b"{\"value\":1}\n"]).unwrap();
    let checkpoint = State::seeded(32).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 4,
        payload: 1.25,
        output: OutputKind::Trace,
    };
    for mode in [
        ObjectMode::Raw,
        ObjectMode::Whole,
        ObjectMode::Warm,
        ObjectMode::Runtime,
        ObjectMode::Combined,
    ] {
        let store = Store::new();
        let mut receiver = ObjectEndpoint::open_with_resource_profile(
            &store.0,
            resources.receiver_limits(),
            wan::BenchAuthority,
            negotiated,
            resources,
        )
        .unwrap();
        assert_eq!(
            receiver.set_codec_cache_enabled(true),
            Err(Error::Unsupported)
        );
        let prediction =
            PreparedPrediction::new_with_resource_profile(&checkpoint, &recipe, resources).unwrap();
        let target = prediction.prediction().to_vec();
        let mut sender = match mode {
            ObjectMode::Runtime => {
                ObjectSender::with_runtime_resource_profile(&c, prediction, negotiated, resources)
            }
            ObjectMode::Combined => ObjectSender::with_combined_resource_profile(
                &c,
                prediction,
                &artifact,
                SelectionPolicy::Exhaustive,
                negotiated,
                resources,
            ),
            _ => ObjectSender::new_with_resource_profile(
                mode,
                &c,
                (mode == ObjectMode::Warm).then_some(artifact.as_slice()),
                negotiated,
                resources,
            ),
        }
        .unwrap();
        let mut exchange = |frames: &[spiralcore_transfer::protocol::Frame]| -> spiralcore_transfer::Result<
            Vec<spiralcore_transfer::protocol::Frame>,
        > {
            let mut replies = Vec::new();
            for frame in frames {
                let reply = receiver.handle(&c, frame, 0)?;
                let done = reply.kind == 103;
                replies.push(reply);
                if done {
                    break;
                }
            }
            Ok(replies)
        };
        if matches!(mode, ObjectMode::Runtime | ObjectMode::Combined) {
            sender.seed_checkpoint(&c, [1; 32], &mut exchange).unwrap();
        }
        if matches!(mode, ObjectMode::Warm | ObjectMode::Combined) {
            sender.seed_schema(&c, [2; 32], &mut exchange).unwrap();
        }
        let stats = sender
            .send(&c, target.clone(), [3; 32], true, &mut exchange)
            .unwrap();
        assert_eq!(stats.object, spiralcore_transfer::object_id(&target));
        let repeated = sender
            .send(&c, target.clone(), [4; 32], true, &mut exchange)
            .unwrap();
        assert_eq!(repeated.object, stats.object);
        assert!(sender.retained_charge() < resources.operation_charge().unwrap());
        assert_eq!(receiver.read_object(&c, stats.object).unwrap(), target);
    }
}

#[test]
fn small_capacity_artifact_shape_and_profile_refusals_precede_exchange_or_store_creation() {
    use spiralcore_transfer::{
        shared_runtime::{OutputKind, Recipe, State},
        shared_transfer::PreparedPrediction,
        v1::{ObjectEndpoint, ObjectMode, ObjectSender},
        wan,
    };
    let resources = ResourceProfile::SmallV1;
    let small = Capabilities::small_v1()
        .negotiate(&Capabilities::small_v1())
        .unwrap();
    let general = Capabilities::full()
        .negotiate(&Capabilities::full())
        .unwrap();
    let c = wan::context([178; 32]);
    let store = Store::new();
    assert!(matches!(
        ObjectEndpoint::open_with_resource_profile(
            &store.0,
            resources.receiver_limits(),
            wan::BenchAuthority,
            general,
            resources
        ),
        Err(Error::Unsupported)
    ));
    assert!(!store.0.exists());
    assert!(matches!(
        ObjectEndpoint::open_with_resource_profile(
            &store.0,
            Default::default(),
            wan::BenchAuthority,
            small,
            resources
        ),
        Err(Error::Limit)
    ));
    assert!(!store.0.exists());
    assert!(matches!(
        ObjectSender::new_with_resource_profile(ObjectMode::Whole, &c, None, general, resources),
        Err(Error::Unsupported)
    ));
    assert!(matches!(
        ObjectSender::new_with_resource_profile(
            ObjectMode::Warm,
            &c,
            Some(&vec![0; 65537]),
            small,
            resources
        ),
        Err(Error::Limit)
    ));
    let mut sender =
        ObjectSender::new_with_resource_profile(ObjectMode::Whole, &c, None, small, resources)
            .unwrap();
    let mut oversized_capacity = Vec::with_capacity(16385);
    oversized_capacity.resize(600, 7);
    let mut calls = 0;
    assert_eq!(
        sender
            .send(&c, oversized_capacity, [7; 32], false, |_| {
                calls += 1;
                Err(Error::Integrity)
            })
            .unwrap_err(),
        Error::Limit
    );
    assert_eq!(calls, 0);
    let checkpoint = State::seeded(4096).unwrap().encode().unwrap();
    let large = Recipe {
        steps: 81,
        payload: 1.25,
        output: OutputKind::Trace,
    };
    assert!(matches!(
        PreparedPrediction::new_with_resource_profile(&checkpoint, &large, resources),
        Err(Error::Limit)
    ));
}

#[test]
fn runtime_receiver_rejects_large_declared_or_computed_prediction_before_evolution() {
    use spiralcore_transfer::{
        client,
        endpoint::{ColdGraphPolicy, Endpoint},
        protocol::{Frame, Mode, Offer},
        shared_runtime::{OutputKind, Recipe, State},
        wan,
    };
    let resources = ResourceProfile::SmallV1;
    let c = wan::context([179; 32]);
    let checkpoint = State::seeded(4096).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 81,
        payload: 1.25,
        output: OutputKind::Trace,
    };
    for declared in [65537u32, 16384] {
        let store = Store::new();
        let mut receiver = Endpoint::open_with_resource_profile(
            &store.0,
            resources.receiver_limits(),
            wan::BenchAuthority,
            resources,
        )
        .unwrap();
        client::transfer_batched(&checkpoint, Mode::Raw, None, [1; 32], false, |frames| {
            let mut replies = Vec::new();
            for frame in frames {
                replies.push(receiver.handle(&c, frame, 0)?);
            }
            Ok(replies)
        })
        .unwrap();
        let offer = Offer {
            mode: Mode::RuntimeV1,
            len: 16384,
            object: [8; 32],
            manifest: None,
            base: Some(spiralcore_transfer::object_id(&checkpoint)),
        };
        let begin = Frame {
            kind: 1,
            transfer: [2; 32],
            seq: 0,
            body: offer.encode().unwrap(),
        };
        assert_eq!(
            receiver.handle(&c, &begin, 0).unwrap().kind,
            101,
            "bounded Runtime admission must not use global32MiB allowance"
        );
        let encoded = recipe.encode().unwrap();
        let mut body = vec![5, 0, 0, 0, 0];
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
        body.extend_from_slice(&encoded);
        body.extend_from_slice(&declared.to_le_bytes());
        body.extend_from_slice(&[9; 32]);
        let setup = Frame {
            kind: 2,
            transfer: [2; 32],
            seq: 1,
            body,
        };
        assert_eq!(receiver.handle(&c, &setup, 0), Err(Error::Limit));
        assert!(matches!(
            receiver.read_object(&c, [8; 32]),
            Err(Error::Unknown)
        ));
    }
    // The existing General profile remains a distinct constructor, not a
    // fallback entered after either bounded refusal above.
    let store = Store::new();
    assert!(Endpoint::open_with_policy(
        &store.0,
        wan::limits(),
        wan::BenchAuthority,
        ColdGraphPolicy::Disabled
    )
    .is_ok());
}

#[test]
fn small_receiver_refuses_oversized_existing_general_base_before_admitting_attempt() {
    use spiralcore_transfer::{
        client,
        endpoint::{ColdGraphPolicy, Endpoint},
        protocol::{Frame, Mode, Offer},
        wan,
    };
    let store = Store::new();
    let c = wan::context([182; 32]);
    let base = vec![7; 128 * 1024];
    let base_id = spiralcore_transfer::object_id(&base);
    {
        let mut general = Endpoint::open_with_policy(
            &store.0,
            wan::limits(),
            wan::BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        client::transfer_batched(&base, Mode::Raw, None, [1; 32], false, |frames| {
            frames.iter().map(|f| general.handle(&c, f, 0)).collect()
        })
        .unwrap();
    }
    let resources = ResourceProfile::SmallV1;
    let mut small = Endpoint::open_with_resource_profile(
        &store.0,
        resources.receiver_limits(),
        wan::BenchAuthority,
        resources,
    )
    .unwrap();
    let offer = Offer {
        mode: Mode::SparseV1,
        len: 600,
        object: [33; 32],
        manifest: None,
        base: Some(base_id),
    };
    let begin = Frame {
        kind: 1,
        transfer: [2; 32],
        seq: 0,
        body: offer.encode().unwrap(),
    };
    assert_eq!(small.handle(&c, &begin, 0), Err(Error::Limit));
    assert_eq!(
        small.handle(
            &c,
            &Frame {
                kind: 5,
                transfer: [2; 32],
                seq: 0,
                body: vec![]
            },
            0
        ),
        Err(Error::Unknown)
    );
    drop(small);
    let mut general = Endpoint::open_with_policy(
        &store.0,
        wan::limits(),
        wan::BenchAuthority,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    assert_eq!(general.read_object(&c, base_id).unwrap(), base);
}

#[test]
fn public_small_selector_refuses_oversized_target_and_base_even_when_codecs_are_pruned() {
    use spiralcore_transfer::{schema, schema_selection::PreparedSelector};
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut selector =
        PreparedSelector::new_with_resource_profile(&artifact, ResourceProfile::SmallV1).unwrap();
    let big = vec![7; 4 * 1024 * 1024];
    assert!(
        matches!(
            selector.prepare_warm(&big, Some(&big), false, false),
            Err(Error::Limit)
        ),
        "a Sparse winner cannot bypass the local target limit by pruning both codecs"
    );
    let short = vec![7; 600];
    assert!(
        matches!(
            selector.prepare_warm(&short, Some(&big), true, false),
            Err(Error::Limit)
        ),
        "base must be checked before residual preparation even for a small target"
    );
}

#[test]
fn public_small_prediction_keeps_profile_bound_when_preparing_later_targets() {
    use spiralcore_transfer::{
        shared_runtime::{OutputKind, Recipe, State},
        shared_transfer::PreparedPrediction,
    };
    let checkpoint = State::seeded(32).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 4,
        payload: 1.25,
        output: OutputKind::Trace,
    };
    let prediction = PreparedPrediction::new_with_resource_profile(
        &checkpoint,
        &recipe,
        ResourceProfile::SmallV1,
    )
    .unwrap();
    assert!(
        matches!(prediction.prepare(&vec![7; 16385]), Err(Error::Limit)),
        "bounded prediction must not admit General-size residual preparation"
    );
    assert!(prediction.prepare(prediction.prediction()).is_ok());
}
