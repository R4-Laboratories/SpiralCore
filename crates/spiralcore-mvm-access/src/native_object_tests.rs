//! Real encrypted TCP and an owned validated application, without a VM claim.
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use mvm_agentd::{
    owner_access::AccessDesk,
    owner_access_object::{CanonicalObjectChannel, ObjectMode, serve_admitted_object},
    vsock::{AuthenticatedSession, GuestRequest, protocol_hello_response},
};
use mvm_contract::{
    grants::owner_access::{
        FieldSchema, OperationGrant, OwnerAccessBinding, OwnerAccessGrant,
        OwnerAccessObjectProfile, ScalarSchema,
    },
    protocol::owner_access::{
        NamedValue, OwnerAccessObjectRequest, OwnerAccessRequest, OwnerAccessStatus, ScalarValue,
        TypedRecord,
    },
};
use std::{
    net::{Shutdown, TcpListener, TcpStream},
    os::unix::fs::{MetadataExt, PermissionsExt},
    sync::Mutex,
};
struct Peer {
    sockets: Vec<TcpStream>,
    worker: Option<std::thread::JoinHandle<PeerOutcome>>,
}
struct PeerOutcome {
    setup_cpu_ns: u128,
    setup_wall_ns: u128,
    exchange_cpu_ns: u128,
    exchange_wall_ns: u128,
    events: u8,
}
struct MeasuredTcp {
    socket: TcpStream,
    counts: Arc<Mutex<(u64, u64)>>,
}
impl std::io::Read for MeasuredTcp {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let count = std::io::Read::read(&mut self.socket, bytes)?;
        self.counts.lock().unwrap().1 += count as u64;
        Ok(count)
    }
}
impl std::io::Write for MeasuredTcp {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = std::io::Write::write(&mut self.socket, bytes)?;
        self.counts.lock().unwrap().0 += count as u64;
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Write::flush(&mut self.socket)
    }
}
impl std::os::fd::AsRawFd for MeasuredTcp {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        std::os::fd::AsRawFd::as_raw_fd(&self.socket)
    }
}
impl std::os::fd::AsFd for MeasuredTcp {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        std::os::fd::AsFd::as_fd(&self.socket)
    }
}
impl AccessSocket for MeasuredTcp {
    fn read_timeout(&self, timeout: Duration) -> std::io::Result<()> {
        self.socket.set_read_timeout(Some(timeout))
    }
    fn write_timeout(&self, timeout: Duration) -> std::io::Result<()> {
        self.socket.set_write_timeout(Some(timeout))
    }
    fn close(&self) {
        let _ = self.socket.shutdown(Shutdown::Both);
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        for socket in &self.sockets {
            let _ = socket.shutdown(Shutdown::Both);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
fn paired_object_whole_keeps_exact_field_and_separate_publication_application_acks() {
    paired_object_case(ObjectMode::Whole, false);
}
#[test]
fn paired_raw_warm_runtime_and_combined_keep_one_canonical_application() {
    for mode in [
        ObjectMode::Raw,
        ObjectMode::Warm,
        ObjectMode::Runtime,
        ObjectMode::Combined,
    ] {
        paired_object_case(mode, false);
    }
}
#[test]
fn configured_public_connector_starts_admission_before_crypto_and_object_discovery() {
    paired_object_case(ObjectMode::Whole, true);
}
fn paired_object_case(mode: ObjectMode, guarded: bool) {
    // Explicit trusted object profile, qualified after the retained default16
    // semantic refusal. The general access default remains sixteen frames.
    let frame_limit = 32;
    let now = Utc::now();
    let host = SigningKey::from_bytes(&[31; 32]);
    let guest = SigningKey::from_bytes(&[32; 32]);
    let signer = SigningKey::from_bytes(&[17; 32]);
    let mut plan = mvm_core::plan::signing::test_support::sample_plan();
    plan.valid_from = now - chrono::Duration::seconds(1);
    plan.valid_until = now + chrono::Duration::seconds(60);
    plan.bundle = Some(mvm_contract::plan::bundle::PlanArtifact::new(
        "b".repeat(64),
        &[7; 64],
        mvm_contract::plan::bundle::KeyId("fixture-key".into()),
    ));
    plan.grants = Some(mvm_contract::grants::Grants {
        owner_access: Some(OwnerAccessGrant {
            object_v1: Some(OwnerAccessObjectProfile {
                major: 1,
                codec_mask: 891,
                cache_reuse_within_execution: false,
            }),
            operations: vec![OperationGrant {
                name: "state".into(),
                owners: vec!["local-owner".into()],
                request: vec![FieldSchema {
                    name: "checkpoint".into(),
                    schema: ScalarSchema::Bytes { max_bytes: 4096 },
                }],
                reply: vec![FieldSchema {
                    name: "checkpoint".into(),
                    schema: ScalarSchema::Bytes { max_bytes: 4096 },
                }],
                max_request_bytes: 16384,
                max_reply_bytes: 16384,
                max_in_flight: 1,
                max_calls_per_second: 32,
                max_deadline_ms: 3000,
            }],
        }),
        ..Default::default()
    });
    let signed = mvm_core::plan::sign_plan(&plan, &signer, "fixture-signer");
    let plan = mvm_core::plan::verify_plan(&signed, &[("fixture-signer", &signer.verifying_key())])
        .unwrap();
    let binding = OwnerAccessBinding::from_plan(&plan, "fixture-object")
        .unwrap()
        .unwrap();
    let mut grant = VerbGrant {
        session_id: "fixture-object".into(),
        plan_nonce: plan.nonce.clone(),
        not_after: now + chrono::Duration::seconds(30),
        verbs: vec![],
        drive: None,
        tool_mediation: None,
        application_terminal: None,
        owner_access: Some(binding.clone()),
        sig: vec![],
    };
    grant.sig = host.sign(&grant.signing_bytes()).to_bytes().to_vec();
    let budget = SharedBudget::new(Limits {
        frames_per_request: frame_limit,
        codec_bytes: spiralcore_transfer::resource_profile::ResourceProfile::SmallV1
            .operation_charge()
            .unwrap() as usize,
        wire_bytes: native::ObjectOptions::wire_charge(),
        ..Default::default()
    })
    .unwrap();
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let emitter = Arc::new(
        mvm_hostd::audit::emitter::AuditEmitter::with_dir(
            signer.clone(),
            &root.path().join("audit"),
        )
        .unwrap(),
    );
    let mut access = native::NativeAccess::component_fixture(
        Admission {
            signed: &signed,
            keys: &[("fixture-signer", &signer.verifying_key())],
            grant: grant.clone(),
            host: &host.verifying_key(),
            control: AccessControl::new(LifecycleBinding {
                instance: "fixture-object".into(),
                tenant: plan.tenant.0.clone(),
                image_sha256: plan.image.sha256.clone(),
                revocation_generation: 1,
                profile: AgentProfile::SealedProd,
            }),
            policy: LocalPolicy::new(["local-owner"], ["state"]),
            budget: budget.clone(),
            clock: Arc::new(|| Time {
                unix: Utc::now(),
                monotonic: Instant::now(),
            }),
        },
        emitter,
    )
    .unwrap();
    let checkpoint = spiralcore_transfer::shared_runtime::State::seeded(32)
        .unwrap()
        .encode()
        .unwrap();
    let recipe = spiralcore_transfer::shared_runtime::Recipe {
        steps: 4,
        payload: 1.25,
        output: spiralcore_transfer::shared_runtime::OutputKind::Trace,
    };
    let artifact =
        spiralcore_transfer::schema::fit(&[b"{\"value\":0}\n", b"{\"value\":1}\n"]).unwrap();
    let body = if matches!(mode, ObjectMode::Runtime | ObjectMode::Combined) {
        spiralcore_transfer::shared_transfer::PreparedPrediction::new_with_resource_profile(
            &checkpoint,
            &recipe,
            spiralcore_transfer::resource_profile::ResourceProfile::SmallV1,
        )
        .unwrap()
        .prediction()
        .to_vec()
    } else {
        vec![42; 4096]
    };
    assert!(body.len() <= 4096);
    let payload = TypedRecord {
        fields: vec![NamedValue {
            name: "checkpoint".into(),
            value: ScalarValue::Bytes(body.clone()),
        }],
    };
    let program = root.path().join("fixed-operation");
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\ncat >/dev/null\nprintf x >> invocations\nprintf '%s' '{}'\n",
            serde_json::to_string(&payload).unwrap()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o555)).unwrap();
    let marker = root.path().join("entrypoint");
    std::fs::write(&marker, program.to_str().unwrap()).unwrap();
    let entrypoint = mvm_agentd::entrypoint::EntrypointPolicy {
        marker_path: marker,
        allowed_prefix: root.path().to_path_buf(),
        same_fs_as: None,
        required_mode: 0o555,
        required_uid: std::fs::metadata(&program).unwrap().uid(),
        required_gid: std::fs::metadata(&program).unwrap().gid(),
        allow_shell_shebang: true,
    }
    .validate()
    .unwrap();
    let cwd = root.path().to_path_buf();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let socket =
        TcpStream::connect_timeout(&listener.local_addr().unwrap(), Duration::from_secs(2))
            .unwrap();
    let (peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    peer.set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut guard = Peer {
        sockets: vec![socket.try_clone().unwrap(), peer.try_clone().unwrap()],
        worker: None,
    };
    let anchor = host.verifying_key();
    let peer_counts = Arc::new(Mutex::new((0u64, 0u64)));
    let peer_observed = peer_counts.clone();
    guard.worker = Some(std::thread::spawn(move || {
        let setup_started = Instant::now();
        let setup_cpu = cpu_time::ThreadTime::now();
        let control = peer.try_clone().unwrap();
        let mut peer = MeasuredTcp {
            socket: peer,
            counts: peer_observed,
        };
        let mut session = AuthenticatedSession::guest(&mut peer, guest, &anchor).unwrap();
        for _ in 0..2 {
            let GuestRequest::ProtocolHello {
                host_protocol_version,
                min_supported_version,
                host_version,
                requested_capabilities,
            } = session.read(&mut peer).unwrap()
            else {
                panic!("expected bounded hello")
            };
            session
                .write(
                    &mut peer,
                    &protocol_hello_response(
                        host_protocol_version,
                        min_supported_version,
                        &host_version,
                        &requested_capabilities,
                    ),
                )
                .unwrap();
        }
        let req: GuestRequest = session.read(&mut peer).unwrap();
        assert!(req.allowed_in(AgentProfile::SealedProd));
        assert!(mvm_agentd::vsock::enforce_verb_grant(&req, Some(&grant)).is_none());
        assert!(
            mvm_agentd::vsock::enforce_owner_access_grant(&req, Some(&grant), Utc::now()).is_none()
        );
        let GuestRequest::OwnerAccessObject(OwnerAccessObjectRequest::Begin(begin)) = req else {
            panic!("expected canonical object header")
        };
        let setup_cpu_ns = setup_cpu.elapsed().as_nanos();
        let setup_wall_ns = setup_started.elapsed().as_nanos();
        let exchange_started = Instant::now();
        let exchange_cpu = cpu_time::ThreadTime::now();
        let mut desk = AccessDesk::default();
        desk.install_verified(
            &grant,
            &anchor,
            "fixture-object",
            &grant.plan_nonce,
            Utc::now(),
        )
        .unwrap();
        let ticket = desk
            .admit_object(&begin, Utc::now(), Instant::now())
            .unwrap();
        let desk = Mutex::new(desk);
        let mut channel = CanonicalObjectChannel::new(&mut peer, &mut session);
        let store = tempfile::Builder::new()
            .prefix("guest-call-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir_in(&cwd)
            .unwrap();
        let report = serve_admitted_object(
            &desk,
            ticket,
            &entrypoint,
            &cwd,
            &store.path().join("objects"),
            &control,
            &mut channel,
        );
        eprintln!(
            "object peer result={:?} actual_wire={:?}",
            report.as_ref().err(),
            channel.counters()
        );
        assert!(report.is_ok(), "canonical object peer: {report:?}");
        let report = report.unwrap();
        assert_eq!(report.application_invocations, 1);
        assert!(report.events_received <= 16);
        drop(store);
        PeerOutcome {
            setup_cpu_ns,
            setup_wall_ns,
            exchange_cpu_ns: exchange_cpu.elapsed().as_nanos(),
            exchange_wall_ns: exchange_started.elapsed().as_nanos(),
            events: report.events_received,
        }
    }));
    if guarded {
        // Physical TCP setup is fixture preparation. This exercises the real
        // connector callback/crypto/discovery after original Call admission.
        access = access.with_object_store(root.path().to_path_buf()).unwrap();
        let host_counts = Arc::new(Mutex::new((0u64, 0u64)));
        let observed = host_counts.clone();
        let before = Instant::now();
        let reply = access
            .call_typed_connect(
                "local-owner",
                OwnerAccessRequest {
                    policy: binding.policy,
                    execution: binding.execution,
                    request_id: [44; 32],
                    sequence: 1,
                    operation: "state".into(),
                    owner: "local-owner".into(),
                    deadline_unix_ms: Utc::now().timestamp_millis() as u64 + 2500,
                    payload: payload.clone(),
                },
                |deadline| {
                    assert_eq!(budget.usage().unwrap().0, 1);
                    assert!(
                        budget.object_usage().unwrap().0 > 0
                            && budget.object_usage().unwrap().1 > 0
                    );
                    assert!(deadline <= before + Duration::from_secs(3));
                    Ok((
                        MeasuredTcp {
                            socket,
                            counts: observed,
                        },
                        host,
                        SigningKey::from_bytes(&[32; 32]).verifying_key(),
                    ))
                },
            )
            .unwrap();
        assert_eq!(reply.status, OwnerAccessStatus::Ok);
        assert_eq!(reply.payload, payload);
        guard.worker.take().unwrap().join().unwrap();
        assert_eq!(*host_counts.lock().unwrap(), {
            let (sent, received) = *peer_counts.lock().unwrap();
            (received, sent)
        });
        assert_eq!(
            std::fs::read(root.path().join("invocations")).unwrap(),
            b"x"
        );
        assert_eq!(access.counters().forwarded_frames, 18);
        assert_eq!(access.counters().completed, 1);
        assert_eq!(budget.usage().unwrap(), (0, 0));
        assert_eq!(budget.object_usage().unwrap(), (0, 0));
        assert!(
            !std::fs::read_dir(root.path()).unwrap().any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("access-call-")),
            "fresh host stores must be removed before returning"
        );
        return;
    }
    let setup_started = Instant::now();
    let setup_cpu = cpu_time::ThreadTime::now();
    let mut client = OwnerClient::connect_object(
        socket,
        "fixture-object",
        host,
        &SigningKey::from_bytes(&[32; 32]).verifying_key(),
        Instant::now() + Duration::from_secs(3),
    )
    .unwrap();
    let setup = client.wire_counters();
    let host_setup_cpu_ns = setup_cpu.elapsed().as_nanos();
    let host_setup_wall_ns = setup_started.elapsed().as_nanos();
    let exchange_started = Instant::now();
    let exchange_cpu = cpu_time::ThreadTime::now();
    eprintln!(
        "owned object root mode={:o}, canonical_same={}, is_symlink={}",
        std::fs::symlink_metadata(root.path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        root.path().canonicalize().unwrap() == root.path(),
        std::fs::symlink_metadata(root.path())
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let result = access.call_typed_object(
        &mut client,
        "local-owner",
        OwnerAccessRequest {
            policy: binding.policy,
            execution: binding.execution,
            request_id: [44; 32],
            sequence: 1,
            operation: "state".into(),
            owner: "local-owner".into(),
            deadline_unix_ms: Utc::now().timestamp_millis() as u64 + 2500,
            payload: payload.clone(),
        },
        root.path(),
        native::ObjectOptions {
            mode,
            schema: matches!(mode, ObjectMode::Warm | ObjectMode::Combined)
                .then_some(artifact.as_slice()),
            prediction: matches!(mode, ObjectMode::Runtime | ObjectMode::Combined)
                .then_some((checkpoint.as_slice(), &recipe)),
        },
    );
    let host_exchange_cpu_ns = exchange_cpu.elapsed().as_nanos();
    let host_exchange_wall_ns = exchange_started.elapsed().as_nanos();
    eprintln!(
        "object host error={:?} actual_wire={:?} setup={:?} admitted_counters={:?} shared_codec_wire={:?}",
        result.as_ref().err(),
        client.wire_counters(),
        setup,
        access.counters(),
        budget.object_usage().unwrap()
    );
    let reply = result.unwrap();
    assert_eq!(reply.reply.status, OwnerAccessStatus::Ok);
    assert_eq!(reply.reply.payload, payload);
    assert!(reply.wire.sent > 0 && reply.wire.received > 0);
    assert_eq!(access.counters().completed, 1);
    assert_eq!(
        std::fs::read(root.path().join("invocations")).unwrap(),
        b"x"
    );
    assert_eq!(budget.usage().unwrap(), (0, 0));
    assert_eq!(budget.object_usage().unwrap(), (0, 0));
    let peer = guard.worker.take().unwrap().join().unwrap();
    let (sent, received) = *peer_counts.lock().unwrap();
    assert_eq!(
        client.wire_counters().sent,
        received,
        "all actual host writes reached the peer"
    );
    assert_eq!(
        client.wire_counters().received,
        sent,
        "all actual peer writes reached the host"
    );
    assert!(reply.wire.frames_sent + reply.wire.frames_received <= 32);
    if matches!(mode, ObjectMode::Whole | ObjectMode::Raw) {
        assert_eq!(reply.wire.frames_sent + reply.wire.frames_received, 18);
    }
    if mode == ObjectMode::Runtime {
        assert_eq!(
            reply.input_receipt.transfer.mode,
            spiralcore_transfer::protocol::Mode::RuntimeV1
        );
    }
    if let Some(directory) = std::env::var_os("SPIRALCORE_ACCESS_EVIDENCE_DIR") {
        let mut record = serde_json::json!({
            "scope":"encrypted TCP component with owned application; no VM/WAN claim",
            "requested_mode":format!("{mode:?}"),"input_codec":format!("{:?}",reply.input_receipt.transfer.mode),
            "frame_limit":frame_limit,"frames_sent":reply.wire.frames_sent,"frames_received":reply.wire.frames_received,"forwarded_frames":access.counters().forwarded_frames,
            "all_wire_sent":client.wire_counters().sent,"all_wire_received":client.wire_counters().received,"peer_all_sent":sent,"peer_all_received":received,"all_two_direction_bytes_match":true,
            "setup_wire_sent":setup.sent,"setup_wire_received":setup.received,"object_wire_sent":reply.wire.sent,"object_wire_received":reply.wire.received,
            "shared_core_target_batch_request_model_excludes_seeds":reply.input_receipt.transfer.request_bytes,"shared_core_target_batch_response_model_excludes_seeds":reply.input_receipt.transfer.response_bytes,
            "actual_input_core_sent_includes_seeds":reply.input_core_sent,"actual_input_core_received_includes_seeds":reply.input_core_received,
            "exact_payload_bytes":body.len(),"exact_payload_id":spiralcore_transfer::hex(&spiralcore_transfer::object_id(&body)),"exact_post_policy_readback":true,"application_invocations":1,"completed":access.counters().completed,
            "codec_reservation":reply.codec_reservation,"wire_reservation":reply.wire_reservation,"codec_retained_after":budget.object_usage().unwrap().0,"wire_retained_after":budget.object_usage().unwrap().1,
        });
        let timings = serde_json::json!({
            "host_setup_cpu_ns":host_setup_cpu_ns,"host_setup_wall_ns":host_setup_wall_ns,"host_exchange_cpu_ns":host_exchange_cpu_ns,"host_exchange_wall_ns":host_exchange_wall_ns,
            "peer_setup_cpu_ns":peer.setup_cpu_ns,"peer_setup_wall_ns":peer.setup_wall_ns,"peer_exchange_cpu_ns":peer.exchange_cpu_ns,"peer_exchange_wall_ns":peer.exchange_wall_ns,
            "verified_lifecycle_wall_ns":exchange_started.elapsed().as_nanos(),"peer_events":peer.events,
            "cpu_domain":"separate host and peer current-thread self CPU; helper threads and application child CPU excluded/NOT_MEASURED",
            "setup_domain":"host crypto handshake/two hellos; peer handshake/two hellos/wait for Begin",
            "lifecycle_domain":"public call including host fresh-store cleanup plus peer join and guest fresh-store cleanup"
        });
        record.as_object_mut().unwrap().extend(
            timings
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        );
        let output = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(
                std::path::PathBuf::from(directory)
                    .join(format!("object-{mode:?}-frame{frame_limit}.json")),
            )
            .unwrap();
        serde_json::to_writer_pretty(output, &record).unwrap();
    }
    drop(client);
}
