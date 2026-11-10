use spiralcore_transfer::{
    Error, Id, client,
    endpoint::Context,
    protocol::{Frame, Mode, Offer},
    resource_profile::ResourceProfile,
    v1::{Capabilities, ObjectEndpoint, OrderedBinding, OrderedLane, OrderedObjectEndpoint},
    wan,
};
const APP: &[u8] = b"spiralcore.flow.application.v1\0";
const ART: &[u8] = b"spiralcore.flow.artifact.v1\0";
struct Store(std::path::PathBuf);
impl Store {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "ordered-retention-{}-{}",
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
fn receiver(
    store: &Store,
    c: &Context,
) -> (OrderedBinding, OrderedObjectEndpoint<wan::BenchAuthority>) {
    let caps = Capabilities::small_v1();
    let endpoint = ObjectEndpoint::open_with_resource_profile(
        &store.0,
        ResourceProfile::SmallV1.receiver_limits(),
        wan::BenchAuthority,
        caps.negotiate(&caps).unwrap(),
        ResourceProfile::SmallV1,
    )
    .unwrap();
    let binding = OrderedBinding::new(c, c.namespace().unwrap(), 0, APP, ART).unwrap();
    let ordered = OrderedObjectEndpoint::new(endpoint, binding.clone(), c).unwrap();
    (binding, ordered)
}
fn send(
    endpoint: &mut OrderedObjectEndpoint<wan::BenchAuthority>,
    binding: &OrderedBinding,
    c: &Context,
    lane: OrderedLane,
    index: u64,
    bytes: &[u8],
) -> Id {
    let receipt = client::transfer_batched(
        bytes,
        Mode::Raw,
        None,
        binding.transfer_id(lane, index),
        false,
        |frames| {
            frames
                .iter()
                .map(|f| endpoint.handle(lane, index, c, f, 0))
                .collect()
        },
    )
    .unwrap();
    assert_eq!(endpoint.read_object(c, receipt.object).unwrap(), bytes);
    receipt.object
}
#[test]
fn ordered_progress_keeps_retry_until_successful_next_admission_and_rejects_old_scope() {
    let store = Store::new();
    let c = wan::context([181; 32]);
    let (binding, mut endpoint) = receiver(&store, &c);
    let bytes = b"first exact immutable bytes";
    send(
        &mut endpoint,
        &binding,
        &c,
        OrderedLane::Application,
        0,
        bytes,
    );
    let finish = Frame {
        kind: 3,
        transfer: binding.transfer_id(OrderedLane::Application, 0),
        seq: 2,
        body: vec![],
    };
    assert_eq!(
        endpoint
            .handle(OrderedLane::Application, 0, &c, &finish, 0)
            .unwrap()
            .kind,
        103
    );
    let malformed = Frame {
        kind: 1,
        transfer: binding.transfer_id(OrderedLane::Application, 1),
        seq: 0,
        body: vec![],
    };
    assert!(
        endpoint
            .handle(OrderedLane::Application, 1, &c, &malformed, 0)
            .is_err()
    );
    assert_eq!(
        endpoint
            .handle(OrderedLane::Application, 0, &c, &finish, 0)
            .unwrap()
            .kind,
        103
    );
    let offer = Offer {
        mode: Mode::Raw,
        len: 4,
        object: spiralcore_transfer::object_id(b"next"),
        manifest: None,
        base: None,
    };
    let begin = Frame {
        body: offer.encode().unwrap(),
        ..malformed
    };
    assert_eq!(
        endpoint
            .handle(OrderedLane::Application, 1, &c, &begin, 0)
            .unwrap()
            .kind,
        101
    );
    assert_eq!(
        endpoint.handle(OrderedLane::Application, 0, &c, &finish, 0),
        Err(Error::Sequence)
    );
    let mut wrong = c.clone();
    wrong.tenant = "foreign".into();
    assert_eq!(
        endpoint.handle(OrderedLane::Application, 1, &wrong, &begin, 0),
        Err(Error::Unauthorized)
    );
    let other = OrderedBinding::new(&c, c.namespace().unwrap(), 1, APP, ART).unwrap();
    let forged = Frame {
        transfer: other.transfer_id(OrderedLane::Application, 1),
        ..begin
    };
    assert_eq!(
        endpoint.handle(OrderedLane::Application, 1, &c, &forged, 0),
        Err(Error::Sequence)
    );
}
#[test]
fn refused_second_lane_cannot_evict_a_verified_publication() {
    let store = Store::new();
    let c = wan::context([187; 32]);
    let resources = ResourceProfile::SmallV1;
    let mut limits = resources.receiver_limits();
    limits.max_publications = 4;
    limits.namespace_bytes = 24 * 1024;
    limits.cache_bytes = limits.namespace_bytes;
    let caps = Capabilities::small_v1();
    let core = ObjectEndpoint::open_with_resource_profile(
        &store.0,
        limits,
        wan::BenchAuthority,
        caps.negotiate(&caps).unwrap(),
        resources,
    )
    .unwrap();
    let binding = OrderedBinding::new(&c, c.namespace().unwrap(), 0, APP, ART).unwrap();
    let mut endpoint = OrderedObjectEndpoint::new(core, binding.clone(), &c).unwrap();
    for index in 0..4 {
        send(
            &mut endpoint,
            &binding,
            &c,
            OrderedLane::Application,
            index,
            &vec![index as u8; 4 * 1024],
        );
    }
    let begin = |lane, index, bytes: &[u8]| Frame {
        kind: 1,
        transfer: binding.transfer_id(lane, index),
        seq: 0,
        body: Offer::cold(bytes, Mode::Raw).encode().unwrap(),
    };
    let active = begin(OrderedLane::Application, 4, &[7; 2048]);
    assert_eq!(
        endpoint
            .handle(OrderedLane::Application, 4, &c, &active, 0)
            .unwrap()
            .kind,
        101,
    );
    let before = endpoint.snapshot(&c).unwrap();
    assert_eq!(before.publications, 3);
    let refused = begin(OrderedLane::Artifact, 0, &[9; 16 * 1024]);
    assert_eq!(
        endpoint.handle(OrderedLane::Artifact, 0, &c, &refused, 0),
        Err(Error::Limit)
    );
    assert_eq!(endpoint.snapshot(&c).unwrap(), before);
}

#[test]
fn ordered_cache_evicts_verified_old_applications_and_preserves_two_setup_pins() {
    let store = Store::new();
    let c = wan::context([182; 32]);
    let (binding, mut endpoint) = receiver(&store, &c);
    let first = send(
        &mut endpoint,
        &binding,
        &c,
        OrderedLane::Artifact,
        0,
        b"setup immutable a",
    );
    let second = send(
        &mut endpoint,
        &binding,
        &c,
        OrderedLane::Artifact,
        1,
        b"setup immutable b",
    );
    let third = Offer {
        mode: Mode::Raw,
        len: 5,
        object: spiralcore_transfer::object_id(b"third"),
        manifest: None,
        base: None,
    };
    let denied = Frame {
        kind: 1,
        transfer: binding.transfer_id(OrderedLane::Artifact, 2),
        seq: 0,
        body: third.encode().unwrap(),
    };
    let before_third = endpoint.snapshot(&c).unwrap();
    assert_eq!(
        endpoint.handle(OrderedLane::Artifact, 2, &c, &denied, 0),
        Err(Error::Limit)
    );
    assert_eq!(endpoint.snapshot(&c).unwrap(), before_third);
    let mut oldest = None;
    for index in 0..200u64 {
        let mut bytes = vec![0x61; 600];
        bytes[..8].copy_from_slice(&index.to_be_bytes());
        let id = send(
            &mut endpoint,
            &binding,
            &c,
            OrderedLane::Application,
            index,
            &bytes,
        );
        oldest.get_or_insert(id);
        let snapshot = endpoint.snapshot(&c).unwrap();
        assert!(snapshot.attempts <= 2);
        assert!(snapshot.publications <= 64);
        assert_eq!(snapshot.pinned, 2);
    }
    assert!(endpoint.snapshot(&c).unwrap().evictions > 0);
    assert_eq!(
        endpoint.read_object(&c, oldest.unwrap()),
        Err(Error::Unknown)
    );
    assert_eq!(
        endpoint.read_object(&c, first).unwrap(),
        b"setup immutable a"
    );
    assert_eq!(
        endpoint.read_object(&c, second).unwrap(),
        b"setup immutable b"
    );
    let literal = vec![0x79; 600];
    let missing = Offer {
        mode: Mode::SparseV1,
        len: literal.len() as u64,
        object: spiralcore_transfer::object_id(&literal),
        manifest: None,
        base: oldest,
    };
    let begin = Frame {
        kind: 1,
        transfer: binding.transfer_id(OrderedLane::Application, 200),
        seq: 0,
        body: missing.encode().unwrap(),
    };
    assert_eq!(
        endpoint.handle(OrderedLane::Application, 200, &c, &begin, 0),
        Err(Error::Missing)
    );
    let previous = Frame {
        kind: 3,
        transfer: binding.transfer_id(OrderedLane::Application, 199),
        seq: 2,
        body: vec![],
    };
    assert_eq!(
        endpoint
            .handle(OrderedLane::Application, 199, &c, &previous, 0)
            .unwrap()
            .kind,
        103
    );
    // Explicit same-ordinal literal recovery, not a hidden retry or new scope.
    send(
        &mut endpoint,
        &binding,
        &c,
        OrderedLane::Application,
        200,
        &literal,
    );
    assert_eq!(
        endpoint.handle(OrderedLane::Application, 199, &c, &previous, 0),
        Err(Error::Sequence)
    );
}

#[test]
fn poisoned_cache_victim_fences_without_deleting_unverified_bytes() {
    use std::io::{Seek, SeekFrom, Write};
    let store = Store::new();
    let c = wan::context([183; 32]);
    let (binding, mut endpoint) = receiver(&store, &c);
    let mut first = None;
    for index in 0..64u64 {
        let mut bytes = vec![0x61; 600];
        bytes[..8].copy_from_slice(&index.to_be_bytes());
        let id = send(
            &mut endpoint,
            &binding,
            &c,
            OrderedLane::Application,
            index,
            &bytes,
        );
        first.get_or_insert(id);
    }
    let path = store
        .0
        .join(spiralcore_transfer::hex(&c.namespace().unwrap()))
        .join(format!(
            "{}.object",
            spiralcore_transfer::hex(&first.unwrap())
        ));
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(49)).unwrap();
    file.write_all(&[0xff]).unwrap();
    file.sync_all().unwrap();
    drop(file);
    let offer = Offer::cold(b"new bytes after poison", Mode::Raw);
    let begin = Frame {
        kind: 1,
        transfer: binding.transfer_id(OrderedLane::Application, 64),
        seq: 0,
        body: offer.encode().unwrap(),
    };
    assert_eq!(
        endpoint.handle(OrderedLane::Application, 64, &c, &begin, 0),
        Err(Error::Storage)
    );
    assert!(
        path.exists(),
        "poison is retained, not reported as verified eviction"
    );
    assert_eq!(endpoint.snapshot(&c), Err(Error::Storage));
}

#[test]
fn binding_and_progress_refuse_bad_domains_scope_gaps_and_unqualified_general() {
    let c = wan::context([184; 32]);
    let ns = c.namespace().unwrap();
    assert!(OrderedBinding::new(&c, ns, 0, APP, APP).is_err());
    assert!(OrderedBinding::new(&c, [0; 32], 0, APP, ART).is_err());
    assert!(OrderedBinding::new(&c, ns, 2, APP, ART).is_err());
    assert!(OrderedBinding::new(&c, ns, 0, b"", ART).is_err());
    let store = Store::new();
    let (binding, mut endpoint) = receiver(&store, &c);
    let offer = Offer::cold(b"one", Mode::Raw);
    for ordinal in [1, u64::MAX] {
        let frame = Frame {
            kind: 1,
            transfer: binding.transfer_id(OrderedLane::Application, ordinal),
            seq: 0,
            body: offer.encode().unwrap(),
        };
        assert_eq!(
            endpoint.handle(OrderedLane::Application, ordinal, &c, &frame, 0),
            Err(Error::Sequence)
        );
    }
    let foreign = OrderedBinding::new(&c, [77; 32], 0, APP, ART).unwrap();
    let wrong = Frame {
        kind: 1,
        transfer: foreign.transfer_id(OrderedLane::Application, 0),
        seq: 0,
        body: offer.encode().unwrap(),
    };
    assert_eq!(
        endpoint.handle(OrderedLane::Application, 0, &c, &wrong, 0),
        Err(Error::Sequence)
    );
    let general_store = Store::new();
    let caps = Capabilities::full();
    let general = ObjectEndpoint::open(
        &general_store.0,
        Default::default(),
        wan::BenchAuthority,
        caps.negotiate(&caps).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        OrderedObjectEndpoint::new(general, binding, &c),
        Err(Error::Unsupported)
    ));
}

#[test]
fn all_pinned_exhaustion_preserves_setup_and_completed_replay() {
    let store = Store::new();
    let c = wan::context([185; 32]);
    let caps = Capabilities::small_v1();
    let mut limits = ResourceProfile::SmallV1.receiver_limits();
    limits.max_publications = 2;
    let inner = ObjectEndpoint::open_with_resource_profile(
        &store.0,
        limits,
        wan::BenchAuthority,
        caps.negotiate(&caps).unwrap(),
        ResourceProfile::SmallV1,
    )
    .unwrap();
    let binding = OrderedBinding::new(&c, c.namespace().unwrap(), 0, APP, ART).unwrap();
    let mut endpoint = OrderedObjectEndpoint::new(inner, binding.clone(), &c).unwrap();
    let a = send(&mut endpoint, &binding, &c, OrderedLane::Artifact, 0, b"a");
    let b = send(&mut endpoint, &binding, &c, OrderedLane::Artifact, 1, b"b");
    let before = endpoint.snapshot(&c).unwrap();
    let begin = Frame {
        kind: 1,
        transfer: binding.transfer_id(OrderedLane::Application, 0),
        seq: 0,
        body: Offer::cold(b"blocked", Mode::Raw).encode().unwrap(),
    };
    assert_eq!(
        endpoint.handle(OrderedLane::Application, 0, &c, &begin, 0),
        Err(Error::Limit)
    );
    assert_eq!(endpoint.snapshot(&c).unwrap(), before);
    let prior = Frame {
        kind: 3,
        transfer: binding.transfer_id(OrderedLane::Artifact, 1),
        seq: 2,
        body: vec![],
    };
    assert_eq!(
        endpoint
            .handle(OrderedLane::Artifact, 1, &c, &prior, 0)
            .unwrap()
            .kind,
        103
    );
    assert_eq!(endpoint.read_object(&c, a).unwrap(), b"a");
    assert_eq!(endpoint.read_object(&c, b).unwrap(), b"b");
}

#[test]
fn current_authority_precedes_cache_membership_and_ordered_diagnostics() {
    #[derive(Clone)]
    struct Gate(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl spiralcore_transfer::endpoint::Authority for Gate {
        fn authorize(&self, _: &Context) -> spiralcore_transfer::Result<()> {
            if self.0.load(std::sync::atomic::Ordering::Acquire) {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }
    }
    let store = Store::new();
    let c = wan::context([187; 32]);
    let caps = Capabilities::small_v1();
    let gate = Gate(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
        true,
    )));
    let inner = ObjectEndpoint::open_with_resource_profile(
        &store.0,
        ResourceProfile::SmallV1.receiver_limits(),
        gate.clone(),
        caps.negotiate(&caps).unwrap(),
        ResourceProfile::SmallV1,
    )
    .unwrap();
    let binding = OrderedBinding::new(&c, c.namespace().unwrap(), 0, APP, ART).unwrap();
    let mut endpoint = OrderedObjectEndpoint::new(inner, binding.clone(), &c).unwrap();
    let offer = Offer::cold(b"bytes", Mode::Raw);
    let begin = Frame {
        kind: 1,
        transfer: binding.transfer_id(OrderedLane::Application, 0),
        seq: 0,
        body: offer.encode().unwrap(),
    };
    gate.0.store(false, std::sync::atomic::Ordering::Release);
    assert_eq!(
        endpoint.handle(OrderedLane::Application, 0, &c, &begin, 0),
        Err(Error::Unauthorized)
    );
    assert_eq!(
        endpoint.read_object(&c, offer.object),
        Err(Error::Unauthorized)
    );
    assert_eq!(endpoint.snapshot(&c), Err(Error::Unauthorized));
}
