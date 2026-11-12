use crate as spiralcore_mvm_flowmux;
use crate::{Binding, Duplex, Event, ObjectChannel, Role};
use spiralcore_transfer::{
    endpoint::{Authority, Context, Limits},
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
};
use std::{
    net::{TcpListener, TcpStream},
    path::PathBuf,
    time::Duration,
};
struct Allow;
impl Authority for Allow {
    fn authorize(&self, _: &Context) -> spiralcore_transfer::Result<()> {
        Ok(())
    }
}
struct Store(PathBuf);
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn context() -> Context {
    Context {
        tenant: "duplex-test".into(),
        domain: "cooperating".into(),
        sender: "roles".into(),
        recipient: "roles".into(),
        mailbox: "application-stream".into(),
        capability: "test-only-selected".into(),
        authorization_scope: "flow".into(),
        policy_epoch: 0,
        execution_epoch: 0,
    }
}
#[test]
fn simultaneous_identical_objects_keep_distinct_direction_ids_and_reverse_after_fin() {
    let c = context();
    let binding = Binding {
        flow: c.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let profile = caps.negotiate(&caps).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, peer) = listener.accept().unwrap();
    let left_store =
        Store(std::env::temp_dir().join(format!("spiral-duplex-left-{}", std::process::id())));
    let right_store =
        Store(std::env::temp_dir().join(format!("spiral-duplex-right-{}", std::process::id())));
    let make = |stream, peer, store: &Store, role| {
        let channel = ObjectChannel::from_connected(
            stream,
            binding,
            peer,
            Duration::from_secs(2),
            caps,
            caps,
        )
        .unwrap();
        let sender = ObjectSender::new_with_profile(ObjectMode::Whole, &c, None, profile).unwrap();
        let endpoint = ObjectEndpoint::open(&store.0, Limits::default(), Allow, profile).unwrap();
        Duplex::spawn(channel, sender, endpoint, c.clone(), role)
            .unwrap()
            .split()
    };
    let (mut left_tx, mut left_rx, left_guard) =
        make(client, address, &left_store, Role::Initiator);
    let (mut right_tx, mut right_rx, right_guard) =
        make(server, peer, &right_store, Role::Acceptor);
    std::thread::scope(|scope| {
        let left = scope.spawn(move || {
            let receipt = left_tx.send_block(vec![42; 8192]).unwrap();
            left_tx.half_close().unwrap();
            receipt
        });
        let right = scope.spawn(move || {
            let receipt = right_tx.send_block(vec![42; 8192]).unwrap();
            right_tx.half_close().unwrap();
            receipt
        });
        assert_eq!(left_rx.receive().unwrap(), Event::Data(vec![42; 8192]));
        assert_eq!(right_rx.receive().unwrap(), Event::Data(vec![42; 8192]));
        assert_eq!(left_rx.receive().unwrap(), Event::HalfClose);
        assert_eq!(right_rx.receive().unwrap(), Event::HalfClose);
        let left = left.join().unwrap();
        let right = right.join().unwrap();
        assert_eq!(left.index, 0);
        assert_eq!(right.index, 0);
        assert_ne!(left.transfer, right.transfer);
        assert_eq!(left.object.transfer.object, right.object.transfer.object);
    });
    let left = left_guard.join().unwrap();
    let right = right_guard.join().unwrap();
    assert_eq!(left.0.wire_bytes, right.1.wire_bytes);
    assert_eq!(left.1.wire_bytes, right.0.wire_bytes);
}

type Halves = (
    spiralcore_mvm_flowmux::DuplexWriter,
    spiralcore_mvm_flowmux::DuplexReader,
    spiralcore_mvm_flowmux::JoinGuard,
);
struct Peers {
    left: Halves,
    right: Halves,
    _stores: [Store; 2],
}
fn prediction() -> spiralcore_transfer::shared_transfer::PreparedPrediction {
    use spiralcore_transfer::{
        shared_runtime::{OutputKind, Recipe, State},
        shared_transfer::PreparedPrediction,
    };
    PreparedPrediction::new(
        &State::seeded(48).unwrap().encode().unwrap(),
        &Recipe {
            steps: 16,
            payload: 1.25,
            output: OutputKind::Trace,
        },
    )
    .unwrap()
}
fn peers(label: &str, mode: ObjectMode, roles: [Role; 2], timeout: Duration) -> Peers {
    let mut c = context();
    c.capability = label.into();
    let binding = Binding {
        flow: c.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let profile = caps.negotiate(&caps).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, peer) = listener.accept().unwrap();
    let stores = [
        Store(
            std::env::temp_dir().join(format!("spiral-duplex-{label}-left-{}", std::process::id())),
        ),
        Store(std::env::temp_dir().join(format!(
            "spiral-duplex-{label}-right-{}",
            std::process::id()
        ))),
    ];
    let artifact = spiralcore_transfer::schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let make = |stream, peer, store: &Store, role| {
        let channel =
            ObjectChannel::from_connected(stream, binding, peer, timeout, caps, caps).unwrap();
        let sender = match mode {
            ObjectMode::Raw | ObjectMode::Whole => {
                ObjectSender::new_with_profile(mode, &c, None, profile)
            }
            ObjectMode::Warm => ObjectSender::new_with_profile(mode, &c, Some(&artifact), profile),
            ObjectMode::Runtime => ObjectSender::with_runtime_profile(&c, prediction(), profile),
            ObjectMode::Combined => ObjectSender::with_combined_policy_and_profile(
                &c,
                prediction(),
                &artifact,
                spiralcore_transfer::session::SelectionPolicy::Exhaustive,
                profile,
            ),
        }
        .unwrap();
        let endpoint = ObjectEndpoint::open(&store.0, Limits::default(), Allow, profile).unwrap();
        Duplex::spawn(channel, sender, endpoint, c.clone(), role)
            .unwrap()
            .split()
    };
    Peers {
        left: make(client, address, &stores[0], roles[0]),
        right: make(server, peer, &stores[1], roles[1]),
        _stores: stores,
    }
}

#[test]
fn artifacts_are_not_application_and_ack_path_survives_application_half_close() {
    let Peers {
        left: (mut left_tx, mut left_rx, left_guard),
        right: (mut right_tx, mut right_rx, right_guard),
        _stores,
    } = peers(
        "artifact-halfclose",
        ObjectMode::Warm,
        [Role::Initiator, Role::Acceptor],
        Duration::from_secs(2),
    );
    left_tx.seed_schema().unwrap();
    left_tx.seed_base(vec![10; 8192]).unwrap();
    let mut target = vec![10; 8192];
    target[100] = 11;
    left_tx.send_block(target.clone()).unwrap();
    left_tx.half_close().unwrap();
    assert_eq!(right_rx.receive().unwrap(), Event::Data(target));
    assert_eq!(right_rx.receive().unwrap(), Event::HalfClose);
    right_tx.send_block(vec![99; 4096]).unwrap();
    assert_eq!(left_rx.receive().unwrap(), Event::Data(vec![99; 4096]));
    right_tx.half_close().unwrap();
    assert_eq!(left_rx.receive().unwrap(), Event::HalfClose);
    let left = left_guard.join().unwrap();
    let right = right_guard.join().unwrap();
    assert_eq!(left.0.wire_bytes, right.1.wire_bytes);
    assert_eq!(left.1.wire_bytes, right.0.wire_bytes);
}

#[test]
fn full_delivery_queue_backpressures_then_times_out_without_extra_published_ack() {
    let Peers {
        left: (mut left_tx, _left_rx, left_guard),
        right: (_right_tx, mut right_rx, right_guard),
        _stores,
    } = peers(
        "queue-bound",
        ObjectMode::Whole,
        [Role::Initiator, Role::Acceptor],
        Duration::from_secs(2),
    );
    for i in 0..4 {
        left_tx.send_block(vec![i; 1024]).unwrap();
    }
    let start = std::time::Instant::now();
    assert!(
        matches!(
            left_tx.send_until(vec![4; 1024], start + Duration::from_millis(180)),
            Err(crate::Error::Timeout)
        ),
        "fifth block cannot receive Published while delivery queue remains full"
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    // The peer cannot send Published before delivery, so the short original
    // send deadline expires before the receiver's two-second queue budget.
    let first = left_tx
        .cancellation
        .first_failure()
        .expect("original failure");
    assert_eq!(first.site, crate::FailureSite::SenderReplyWait);
    assert_eq!(first.frame_kind, Some(3));
    assert_eq!(first.reason, crate::FailureReason::Timeout);
    assert!(matches!(
        left_tx.send_block(vec![5; 1024]),
        Err(crate::Error::Cancelled)
    ));
    assert_eq!(left_tx.cancellation.first_failure(), Some(first));
    assert!(left_guard.join().is_err());
    assert!(right_guard.join().is_err());
    // Fatal timeout fences delivery even for the four previously verified
    // queued blocks; acknowledged publication is not permission after cancel.
    for _ in 0..5 {
        assert!(matches!(
            right_rx.receive(),
            Err(spiralcore_mvm_flowmux::Error::Cancelled)
        ));
    }
}

#[test]
fn same_role_on_both_peers_refuses_cross_direction_application_identity() {
    let Peers {
        left: (mut left_tx, _left_rx, left_guard),
        right: (mut right_tx, _right_rx, right_guard),
        _stores,
    } = peers(
        "role-bound",
        ObjectMode::Whole,
        [Role::Initiator, Role::Initiator],
        Duration::from_secs(1),
    );
    std::thread::scope(|scope| {
        let left = scope.spawn(move || left_tx.send_block(vec![1; 1024]));
        let right = scope.spawn(move || right_tx.send_block(vec![1; 1024]));
        assert!(left.join().unwrap().is_err());
        assert!(right.join().unwrap().is_err());
    });
    let left = left_guard.join();
    let right = right_guard.join();
    assert!(
        matches!(left, Err(spiralcore_mvm_flowmux::Error::Protocol))
            || matches!(right, Err(spiralcore_mvm_flowmux::Error::Protocol))
    );
}

#[test]
fn guard_drop_cancels_and_joins_an_idle_owned_reader() {
    let Peers {
        left: (mut left_tx, mut left_rx, left_guard),
        right: (_right_tx, _right_rx, right_guard),
        _stores,
    } = peers(
        "guard-drop",
        ObjectMode::Whole,
        [Role::Initiator, Role::Acceptor],
        Duration::from_secs(1),
    );
    let start = std::time::Instant::now();
    drop(left_guard);
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(left_tx.send_block(vec![1; 100]).is_err());
    assert!(left_rx.receive().is_err());
    drop(right_guard);
}

#[test]
fn correct_object_published_from_wrong_direction_is_rejected() {
    use spiralcore_transfer::{hash, object_id, protocol::Frame};
    use std::io::{Read, Write};
    let c = context();
    let binding = Binding {
        flow: c.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let profile = caps.negotiate(&caps).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    server
        .set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let store =
        Store(std::env::temp_dir().join(format!("spiral-duplex-reflected-{}", std::process::id())));
    let channel =
        ObjectChannel::from_connected(client, binding, address, Duration::from_secs(1), caps, caps)
            .unwrap();
    let sender = ObjectSender::new_with_profile(ObjectMode::Whole, &c, None, profile).unwrap();
    let endpoint = ObjectEndpoint::open(&store.0, Limits::default(), Allow, profile).unwrap();
    let (mut writer, _reader, guard) = Duplex::spawn(channel, sender, endpoint, c, Role::Initiator)
        .unwrap()
        .split();
    std::thread::scope(|scope| {
        let peer = scope.spawn(move || {
            let mut header = [0; spiralcore_mvm_flowmux::HEADER_LEN];
            server.read_exact(&mut header).unwrap();
            let n = u32::from_be_bytes(header[116..120].try_into().unwrap()) as usize;
            assert!(n <= 65536);
            let mut body = vec![0; n];
            server.read_exact(&mut body).unwrap();
            let request = Frame::decode(&body).unwrap();
            let mut reply = request.reply(103, object_id(&vec![5; 4096]).to_vec());
            reply.transfer = hash(
                b"spiralcore.flow.application.v1\0",
                &[&binding.flow, &[1], &0u64.to_be_bytes()],
            );
            assert_ne!(request.transfer, reply.transfer);
            let encoded = reply.encode().unwrap();
            header[9] = 1;
            header[12..44].copy_from_slice(&binding.directional_flow(Role::Acceptor));
            header[116..120].copy_from_slice(&(encoded.len() as u32).to_be_bytes());
            server.write_all(&header).unwrap();
            server.write_all(&encoded).unwrap();
            let mut probe = [0; 1];
            assert_eq!(server.read(&mut probe).unwrap(), 0);
        });
        assert!(writer.send_block(vec![5; 4096]).is_err());
        assert!(writer.send_block(vec![5; 4096]).is_err());
        assert!(matches!(
            guard.join(),
            Err(spiralcore_mvm_flowmux::Error::Protocol)
        ));
        let original = writer
            .cancellation
            .first_failure()
            .expect("original worker error");
        assert_eq!(original.site, crate::FailureSite::WorkerExit);
        assert_eq!(original.reason, crate::FailureReason::Protocol);
        peer.join().unwrap();
    });
}

#[test]
fn combined_runtime_artifacts_and_repeated_object_deliver_only_two_logical_blocks() {
    let Peers {
        left: (mut left_tx, mut left_rx, left_guard),
        right: (mut right_tx, mut right_rx, right_guard),
        _stores,
    } = peers(
        "combined",
        ObjectMode::Combined,
        [Role::Initiator, Role::Acceptor],
        Duration::from_secs(2),
    );
    left_tx.seed_checkpoint().unwrap();
    left_tx.seed_schema().unwrap();
    let target = prediction().prediction().to_vec();
    let first = left_tx.send_block(target.clone()).unwrap();
    assert_eq!(
        first.object.transfer.mode,
        spiralcore_transfer::protocol::Mode::RuntimeV1
    );
    let second = left_tx.send_block(target.clone()).unwrap();
    assert_ne!(first.transfer, second.transfer);
    assert_eq!(second.index, 1);
    left_tx.half_close().unwrap();
    assert_eq!(right_rx.receive().unwrap(), Event::Data(target.clone()));
    assert_eq!(right_rx.receive().unwrap(), Event::Data(target));
    assert_eq!(right_rx.receive().unwrap(), Event::HalfClose);
    right_tx.half_close().unwrap();
    assert_eq!(left_rx.receive().unwrap(), Event::HalfClose);
    let left = left_guard.join().unwrap();
    let right = right_guard.join().unwrap();
    assert_eq!(left.0.wire_bytes, right.1.wire_bytes);
    assert_eq!(left.1.wire_bytes, right.0.wire_bytes);
    println!(
        "FLOWMUX_COUNTERS duplex-combined wire_tx={} wire_rx={} peer_wire_tx={} peer_wire_rx={}",
        left.0.wire_bytes, left.1.wire_bytes, right.0.wire_bytes, right.1.wire_bytes
    );
}

#[test]
fn thirty_two_interleaved_requests_bound_a_waiting_local_operation() {
    use spiralcore_transfer::{
        hash,
        protocol::{Frame, Mode, Offer},
    };
    use std::io::{Read, Write};
    let mut c = context();
    c.capability = "event-bound".into();
    let binding = Binding {
        flow: c.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let profile = caps.negotiate(&caps).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    peer.set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    peer.set_nodelay(true).unwrap();
    let store =
        Store(std::env::temp_dir().join(format!("spiral-duplex-events-{}", std::process::id())));
    let channel =
        ObjectChannel::from_connected(client, binding, address, Duration::from_secs(2), caps, caps)
            .unwrap();
    let sender = ObjectSender::new_with_profile(ObjectMode::Whole, &c, None, profile).unwrap();
    let endpoint = ObjectEndpoint::open(&store.0, Limits::default(), Allow, profile).unwrap();
    let (mut writer, _reader, guard) = Duplex::spawn(channel, sender, endpoint, c, Role::Initiator)
        .unwrap()
        .split();
    std::thread::scope(|scope| {
        let remote = scope.spawn(move || {
            let mut header = [0; spiralcore_mvm_flowmux::HEADER_LEN];
            peer.read_exact(&mut header).unwrap();
            let n = u32::from_be_bytes(header[116..120].try_into().unwrap()) as usize;
            assert!(n <= 65536);
            let mut body = vec![0; n];
            peer.read_exact(&mut body).unwrap();
            let request = Frame {
                kind: 1,
                transfer: hash(
                    b"spiralcore.flow.artifact.v1\0",
                    &[&binding.flow, &[1], &0u64.to_be_bytes()],
                ),
                seq: 0,
                body: Offer::cold(b"x", Mode::Raw).encode().unwrap(),
            }
            .encode()
            .unwrap();
            header[8] = 7;
            header[9] = 1;
            header[12..44].copy_from_slice(&binding.directional_flow(Role::Acceptor));
            header[116..120].copy_from_slice(&(request.len() as u32).to_be_bytes());
            for index in 0u64..33 {
                header[108..116].copy_from_slice(&index.to_be_bytes());
                peer.write_all(&header).unwrap();
                peer.write_all(&request).unwrap();
                if index == 32 {
                    break;
                }
                let mut ack = [0; spiralcore_mvm_flowmux::HEADER_LEN];
                peer.read_exact(&mut ack).unwrap();
                assert_eq!(ack[8], 7);
                let n = u32::from_be_bytes(ack[116..120].try_into().unwrap()) as usize;
                assert!(n <= 65536);
                let mut body = vec![0; n];
                peer.read_exact(&mut body).unwrap();
                assert_eq!(Frame::decode(&body).unwrap().kind, 101);
            }
            let mut probe = [0; 1];
            // The fatal limit closes with the33rd request unread. Windows can
            // report a reset rather than EOF; neither permits a response byte.
            match peer.read(&mut probe) {
                Ok(0) => {}
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
                other => panic!("expected terminal EOF/reset with no response bytes: {other:?}"),
            }
        });
        assert!(writer.send_block(vec![2; 1024]).is_err());
        assert!(matches!(
            guard.join(),
            Err(spiralcore_mvm_flowmux::Error::Limit)
        ));
        remote.join().unwrap();
    });
}

#[test]
fn oversized_application_offer_is_refused_before_shared_core_dispatch() {
    use spiralcore_transfer::{
        hash,
        protocol::{Frame, Mode, Offer},
    };
    use std::{
        io::{Read, Write},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };
    struct ObservedAuthority(Arc<AtomicUsize>);
    impl Authority for ObservedAuthority {
        fn authorize(&self, _: &Context) -> spiralcore_transfer::Result<()> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    let c = context();
    let binding = Binding {
        flow: c.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let profile = caps.negotiate(&caps).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let socket = TcpStream::connect(address).unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    peer.set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let store =
        Store(std::env::temp_dir().join(format!("flow-preallocation-{}", std::process::id())));
    let calls = Arc::new(AtomicUsize::new(0));
    let endpoint = ObjectEndpoint::open(
        &store.0,
        Limits::default(),
        ObservedAuthority(calls.clone()),
        profile,
    )
    .unwrap();
    let sender = ObjectSender::new_with_profile(ObjectMode::Raw, &c, None, profile).unwrap();
    let channel =
        ObjectChannel::from_connected(socket, binding, address, Duration::from_secs(1), caps, caps)
            .unwrap();
    let (_writer, _reader, guard) = Duplex::spawn(channel, sender, endpoint, c, Role::Initiator)
        .unwrap()
        .split();
    let frame = Frame {
        kind: 1,
        transfer: hash(
            b"spiralcore.flow.application.v1\0",
            &[&binding.flow, &[1], &0u64.to_be_bytes()],
        ),
        seq: 0,
        body: Offer::cold(&vec![7; spiralcore_mvm_flowmux::MAX_BLOCK + 1], Mode::Raw)
            .encode()
            .unwrap(),
    }
    .encode()
    .unwrap();
    let mut header = b"SCFLW001".to_vec();
    header.extend_from_slice(&[6, 1, 0, 0]);
    header.extend_from_slice(&binding.directional_flow(Role::Acceptor));
    header.extend_from_slice(&binding.policy_sha256);
    header.extend_from_slice(&binding.scope_commitment);
    header.extend_from_slice(&0u64.to_be_bytes());
    header.extend_from_slice(&(frame.len() as u32).to_be_bytes());
    assert_eq!(header.len(), spiralcore_mvm_flowmux::HEADER_LEN);
    peer.write_all(&header).unwrap();
    peer.write_all(&frame).unwrap();
    assert_eq!(peer.read(&mut [0; 1]).unwrap(), 0);
    assert!(matches!(
        guard.join(),
        Err(spiralcore_mvm_flowmux::Error::Limit)
    ));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "oversized application Begin must not dispatch/allocate core state"
    );
}

#[test]
fn cancellation_wakes_original_tx_owner_and_conflicting_registration_is_refused() {
    let mut pair = peers(
        "tx-release-cancel",
        ObjectMode::Whole,
        [Role::Initiator, Role::Acceptor],
        Duration::from_secs(2),
    );
    let tx = pair.left.0.tx.clone();
    let cancellation = pair.left.0.cancellation.clone();
    cancellation.bind_tx_wake(tx.gate()).unwrap();
    let other = crate::lock_wait::ReleaseMutex::new(());
    assert!(matches!(
        cancellation.bind_tx_wake(other.gate()),
        Err(crate::Error::Protocol)
    ));
    let held = tx.try_acquire().unwrap().unwrap();
    std::thread::scope(|scope| {
        let waiter = scope.spawn(|| {
            super::acquire_tx(
                &tx,
                std::time::Instant::now() + Duration::from_secs(2),
                &cancellation,
            )
            .err()
        });
        tx.wait_for_enrollment();
        cancellation.cancel();
        assert!(matches!(
            waiter.join().unwrap(),
            Some(crate::Error::Cancelled)
        ));
        drop(held);
    });
    let _ = pair.left.1.receive();
}
