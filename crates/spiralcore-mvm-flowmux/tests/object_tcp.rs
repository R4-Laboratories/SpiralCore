use spiralcore_mvm_flowmux::{Binding, ObjectChannel, ServeEvent};
use spiralcore_transfer::{
    endpoint::{Authority, Context, Limits},
    object_id, schema,
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSelection, ObjectSender},
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    time::Duration,
};

struct Policy;
impl Authority for Policy {
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
        tenant: "flow-test".into(),
        domain: "cooperating".into(),
        sender: "a".into(),
        recipient: "b".into(),
        mailbox: "application-stream".into(),
        capability: "test-only-selected".into(),
        authorization_scope: "flow".into(),
        policy_epoch: 0,
        execution_epoch: 0,
    }
}

#[test]
fn warm_core_published_receipts_over_real_tcp_preserve_exact_object() {
    let c = context();
    let binding = Binding {
        flow: c.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, peer) = listener.accept().unwrap();
    let base: Vec<u8> = (0..8192).map(|i| ((i * 173 + i / 7) % 251) as u8).collect();
    let mut target = base.clone();
    target[4096] ^= 7;
    let expected = target.clone();
    let store =
        Store(std::env::temp_dir().join(format!("spiral-flow-object-{}", std::process::id())));
    std::thread::scope(|scope| {
        let server_context = c.clone();
        let worker = scope.spawn(move || {
            let mut channel = ObjectChannel::from_connected(
                server,
                binding,
                peer,
                Duration::from_secs(2),
                caps,
                caps,
            )
            .unwrap();
            let mut endpoint =
                ObjectEndpoint::open(&store.0, Limits::default(), Policy, channel.profile())
                    .unwrap();
            let mut published = 0;
            while let ServeEvent::Handled { reply, .. } = channel
                .serve_once(&mut endpoint, &server_context, 0)
                .unwrap()
            {
                if reply.kind == 103 {
                    published += 1;
                }
            }
            assert_eq!(
                endpoint
                    .read_object(&server_context, object_id(&expected))
                    .unwrap(),
                expected
            );
            assert!(published >= 2, "only actual core publication is counted");
            channel.half_close().unwrap();
            channel.counters()
        });
        let mut channel = ObjectChannel::from_connected(
            client,
            binding,
            address,
            Duration::from_secs(2),
            caps,
            caps,
        )
        .unwrap();
        let schema = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
        let mut sender =
            ObjectSender::new_with_profile(ObjectMode::Warm, &c, Some(&schema), channel.profile())
                .unwrap();
        channel.seed_base(&mut sender, &c, base, [1; 32]).unwrap();
        let receipt = channel
            .send_block(&mut sender, &c, target, [2; 32], false)
            .unwrap();
        assert!(matches!(receipt.selection, ObjectSelection::Warm(_)));
        channel.half_close().unwrap();
        assert_eq!(channel.receive_terminal().unwrap(), ServeEvent::HalfClose);
        let server_counts = worker.join().unwrap();
        let client_counts = channel.counters();
        assert_eq!(client_counts.0.wire_bytes, server_counts.1.wire_bytes);
        assert_eq!(client_counts.1.wire_bytes, server_counts.0.wire_bytes);
        println!(
            "FLOWMUX_COUNTERS warm target_len=8192 wire_tx={} wire_rx={} peer_wire_tx={} peer_wire_rx={}",
            client_counts.0.wire_bytes,
            client_counts.1.wire_bytes,
            server_counts.0.wire_bytes,
            server_counts.1.wire_bytes
        );
    });
}

#[test]
fn runtime_and_combined_use_shared_artifacts_and_verified_publication_over_tcp() {
    use spiralcore_transfer::{
        session::SelectionPolicy,
        shared_runtime::{OutputKind, Recipe, State},
        shared_transfer::PreparedPrediction,
    };
    for combined in [false, true] {
        let mut c = context();
        c.capability = format!("test-runtime-{combined}");
        let binding = Binding {
            flow: c.namespace().unwrap(),
            policy_sha256: [4; 32],
            scope_commitment: [9; 32],
        };
        let mut caps = Capabilities::full();
        caps.max_object_bytes = 65536;
        let profile = caps.negotiate(&caps).unwrap();
        let prediction = PreparedPrediction::new(
            &State::seeded(48).unwrap().encode().unwrap(),
            &Recipe {
                steps: 16,
                payload: 1.25,
                output: OutputKind::Trace,
            },
        )
        .unwrap();
        let target = prediction.prediction().to_vec();
        assert!(target.len() <= 16384);
        let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
        let mut sender = if combined {
            ObjectSender::with_combined_policy_and_profile(
                &c,
                prediction,
                &artifact,
                SelectionPolicy::Exhaustive,
                profile,
            )
            .unwrap()
        } else {
            ObjectSender::with_runtime_profile(&c, prediction, profile).unwrap()
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let client = TcpStream::connect(address).unwrap();
        let (server, peer) = listener.accept().unwrap();
        let expected = target.clone();
        let store = Store(std::env::temp_dir().join(format!(
            "spiral-flow-runtime-{}-{combined}",
            std::process::id()
        )));
        std::thread::scope(|scope| {
            let server_context = c.clone();
            let worker = scope.spawn(move || {
                let mut channel = ObjectChannel::from_connected(
                    server,
                    binding,
                    peer,
                    Duration::from_secs(2),
                    caps,
                    caps,
                )
                .unwrap();
                let mut endpoint =
                    ObjectEndpoint::open(&store.0, Limits::default(), Policy, profile).unwrap();
                let mut modes = Vec::new();
                while let ServeEvent::Handled { request, .. } = channel
                    .serve_once(&mut endpoint, &server_context, 0)
                    .unwrap()
                {
                    if request.kind == 1 {
                        modes.push(
                            spiralcore_transfer::protocol::Offer::decode(&request.body)
                                .unwrap()
                                .mode,
                        );
                    }
                }
                assert_eq!(
                    endpoint
                        .read_object(&server_context, object_id(&expected))
                        .unwrap(),
                    expected
                );
                assert!(
                    modes.contains(&spiralcore_transfer::protocol::Mode::RuntimeV1),
                    "actual core runtime offer required: {modes:?}"
                );
                channel.half_close().unwrap();
                channel.counters()
            });
            let mut channel = ObjectChannel::from_connected(
                client,
                binding,
                address,
                Duration::from_secs(2),
                caps,
                caps,
            )
            .unwrap();
            channel.seed_checkpoint(&mut sender, &c, [11; 32]).unwrap();
            if combined {
                channel.seed_schema(&mut sender, &c, [12; 32]).unwrap();
            }
            let receipt = channel
                .send_block(&mut sender, &c, target, [13; 32], false)
                .unwrap();
            assert!(if combined {
                matches!(receipt.selection, ObjectSelection::Combined(_))
            } else {
                matches!(receipt.selection, ObjectSelection::Runtime(_))
            });
            channel.half_close().unwrap();
            assert_eq!(channel.receive_terminal().unwrap(), ServeEvent::HalfClose);
            let server_counts = worker.join().unwrap();
            let client_counts = channel.counters();
            assert_eq!(client_counts.0.wire_bytes, server_counts.1.wire_bytes);
            assert_eq!(client_counts.1.wire_bytes, server_counts.0.wire_bytes);
            println!(
                "FLOWMUX_COUNTERS runtime combined={combined} wire_tx={} wire_rx={} peer_wire_tx={} peer_wire_rx={}",
                client_counts.0.wire_bytes,
                client_counts.1.wire_bytes,
                server_counts.0.wire_bytes,
                server_counts.1.wire_bytes
            );
        });
    }
}

#[test]
fn wrong_published_identity_is_not_a_warm_commit_and_fences_reentry() {
    use spiralcore_transfer::protocol::Frame;
    let c = context();
    let binding = Binding {
        flow: c.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    server
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    std::thread::scope(|scope| {
        let worker = scope.spawn(move || {
            let mut header = [0; spiralcore_mvm_flowmux::HEADER_LEN];
            server.read_exact(&mut header).unwrap();
            let n = u32::from_be_bytes(header[116..120].try_into().unwrap()) as usize;
            assert!(n <= 65536);
            let mut encoded = vec![0; n];
            server.read_exact(&mut encoded).unwrap();
            let request = Frame::decode(&encoded).unwrap();
            let reply = request.reply(103, vec![0; 32]).encode().unwrap();
            header[116..120].copy_from_slice(&(reply.len() as u32).to_be_bytes());
            server.write_all(&header).unwrap();
            server.write_all(&reply).unwrap();
            let mut probe = [0; 1];
            assert_eq!(
                server.read(&mut probe).unwrap(),
                0,
                "no suffix after unverified Published"
            );
        });
        let mut channel = ObjectChannel::from_connected(
            client,
            binding,
            address,
            Duration::from_secs(2),
            caps,
            caps,
        )
        .unwrap();
        let schema = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
        let mut sender =
            ObjectSender::new_with_profile(ObjectMode::Warm, &c, Some(&schema), channel.profile())
                .unwrap();
        assert!(matches!(
            channel.seed_base(&mut sender, &c, vec![3; 1024], [44; 32]),
            Err(spiralcore_mvm_flowmux::Error::Core(
                spiralcore_transfer::Error::Integrity
            ))
        ));
        let before = channel.counters();
        assert!(
            channel
                .send_block(&mut sender, &c, vec![3; 1024], [45; 32], false)
                .is_err()
        );
        assert_eq!(channel.counters(), before);
        worker.join().unwrap();
    });
}

#[test]
fn object_namespace_is_immutable_and_checked_before_read() {
    let c = context();
    let binding = Binding {
        flow: c.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let _client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (server, peer) = listener.accept().unwrap();
    let mut channel =
        ObjectChannel::from_connected(server, binding, peer, Duration::from_millis(50), caps, caps)
            .unwrap();
    let store =
        Store(std::env::temp_dir().join(format!("spiral-flow-scope-{}", std::process::id())));
    let mut endpoint =
        ObjectEndpoint::open(&store.0, Limits::default(), Policy, channel.profile()).unwrap();
    let mut wrong = c.clone();
    wrong.tenant = "other-authorized-tenant".into();
    assert!(matches!(
        channel.serve_once(&mut endpoint, &wrong, 0),
        Err(spiralcore_mvm_flowmux::Error::InvalidBinding)
    ));
    assert_eq!(channel.counters(), Default::default());
}
