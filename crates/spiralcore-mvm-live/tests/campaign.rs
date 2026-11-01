use spiralcore_mvm_adapter::live::HandshakeRole;
use spiralcore_mvm_live::{
    campaign,
    config::{Arm, Pair, Plan, Published, Stage},
};
use std::{os::unix::net::UnixStream, path::Path};

fn run(pair: &Pair, cache: &Path) -> (serde_json::Value, Vec<Published>) {
    let (a, b) = UnixStream::pair().unwrap();
    let (sent, received) = std::thread::scope(|scope| {
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
        (sent, receiver.join().unwrap())
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
    (sent, published)
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
