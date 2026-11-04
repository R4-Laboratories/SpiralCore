//! Genuine signed component fixture. The cfg(test)-only constructor does not
//! claim the full opaque MVM admission or a booted guest/remote owner.
use super::*;
use chrono::TimeZone;
use ed25519_dalek::{Signer, SigningKey};
use mvm_agentd::vsock::{
    AuthenticatedSession, GuestRequest, GuestResponse, protocol_hello_response,
};
use mvm_contract::{
    grants::owner_access::{
        FieldSchema, OperationGrant, OwnerAccessBinding, OwnerAccessGrant, ScalarSchema,
    },
    protocol::owner_access::{
        NamedValue, OwnerAccessReply, OwnerAccessRequest, OwnerAccessStatus, ScalarValue,
        TypedRecord,
    },
};
use std::{
    net::{Shutdown, TcpListener, TcpStream},
    thread::JoinHandle,
};

struct PeerGuard {
    sockets: Vec<TcpStream>,
    worker: Option<JoinHandle<()>>,
}
impl Drop for PeerGuard {
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
fn paired_admitted_dev_exec_uses_canonical_frames_and_owned_bounded_child() {
    use mvm_contract::plan::VerbId;
    let now = Utc::now();
    let host = SigningKey::from_bytes(&[31; 32]);
    let guest = SigningKey::from_bytes(&[32; 32]);
    let signer = SigningKey::from_bytes(&[17; 32]);
    let mut plan = mvm_core::plan::signing::test_support::sample_plan();
    plan.valid_from = now - chrono::Duration::seconds(1);
    plan.valid_until = now + chrono::Duration::seconds(60);
    plan.agent_verbs = Some(vec![VerbId::new("exec").unwrap()]);
    let signed = mvm_core::plan::sign_plan(&plan, &signer, "fixture-plan-signer");
    let plan =
        mvm_core::plan::verify_plan(&signed, &[("fixture-plan-signer", &signer.verifying_key())])
            .unwrap();
    let mut grant = VerbGrant {
        session_id: "fixture-dev".into(),
        plan_nonce: plan.nonce.clone(),
        not_after: now + chrono::Duration::seconds(30),
        verbs: vec![VerbId::new("exec").unwrap()],
        drive: None,
        tool_mediation: None,
        application_terminal: None,
        owner_access: None,
        sig: vec![],
    };
    grant.sig = host.sign(&grant.signing_bytes()).to_bytes().to_vec();
    let control = AccessControl::new(LifecycleBinding {
        instance: grant.session_id.clone(),
        tenant: plan.tenant.0.clone(),
        image_sha256: plan.image.sha256.clone(),
        revocation_generation: 1,
        profile: AgentProfile::Dev,
    });
    let budget = SharedBudget::new(Default::default()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let emitter = Arc::new(
        mvm_hostd::audit::emitter::AuditEmitter::with_dir(signer.clone(), directory.path())
            .unwrap(),
    );
    let mut access = native::NativeAccess::component_fixture(
        Admission {
            signed: &signed,
            keys: &[("fixture-plan-signer", &signer.verifying_key())],
            grant: grant.clone(),
            host: &host.verifying_key(),
            control,
            policy: LocalPolicy::new(["local-owner"], ["exec"]),
            budget: budget.clone(),
            clock: Arc::new(|| Time {
                unix: Utc::now(),
                monotonic: Instant::now(),
            }),
        },
        emitter,
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let socket =
        TcpStream::connect_timeout(&listener.local_addr().unwrap(), Duration::from_secs(2))
            .unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    peer.set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut guard = PeerGuard {
        sockets: vec![socket.try_clone().unwrap(), peer.try_clone().unwrap()],
        worker: None,
    };
    let anchor = host.verifying_key();
    guard.worker = Some(std::thread::spawn(move || {
        let mut session = AuthenticatedSession::guest(&mut peer, guest, &anchor).unwrap();
        for _ in 0..2 {
            let req: GuestRequest = session.read(&mut peer).unwrap();
            let GuestRequest::ProtocolHello {
                host_protocol_version,
                min_supported_version,
                host_version,
                requested_capabilities,
            } = req
            else {
                panic!("expected canonical hello")
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
        assert!(req.allowed_in(AgentProfile::Dev));
        assert!(mvm_agentd::vsock::enforce_verb_grant(&req, Some(&grant)).is_none());
        let GuestRequest::Exec {
            command,
            stdin,
            deadline_unix_ms: Some(deadline),
            ..
        } = req
        else {
            panic!("expected bounded canonical Exec")
        };
        let deadline =
            mvm_agentd::owner_access::received_deadline(deadline, Utc::now(), Instant::now())
                .unwrap();
        let terminal = mvm_agentd::exec_stream::stream_exec_bounded(
            &command,
            stdin.as_deref(),
            deadline,
            &mvm_agentd::entrypoint::CancellationToken::default(),
            |event| {
                session
                    .write(&mut peer, &GuestResponse::ExecEvent(event))
                    .unwrap()
            },
        );
        session
            .write(&mut peer, &GuestResponse::ExecEvent(terminal))
            .unwrap();
    }));
    let mut client = OwnerClient::connect(
        socket,
        "fixture-dev",
        host,
        &SigningKey::from_bytes(&[32; 32]).verifying_key(),
        Instant::now() + Duration::from_secs(3),
    )
    .unwrap();
    let reply = access
        .call_dev(
            &mut client,
            "local-owner",
            [44; 32],
            "printf 'bounded-dev-ok'",
            None,
        )
        .unwrap();
    assert_eq!(reply.stdout, b"bounded-dev-ok");
    assert!(reply.stderr.is_empty());
    assert_eq!(reply.code, 0);
    assert_eq!(budget.usage().unwrap(), (0, 0));
    assert_eq!(access.counters().completed, 1);
    assert!(client.wire_counters().sent > 0 && client.wire_counters().received > 0);
    drop(client);
    drop(guard);
}

#[test]
fn paired_canonical_typed_status_preserves_policy_audit_replay_and_exact_bytes() {
    let now = Utc.with_ymd_and_hms(2026, 11, 3, 12, 0, 0).unwrap();
    let plan_key = SigningKey::from_bytes(&[17; 32]);
    let host = SigningKey::from_bytes(&[31; 32]);
    let guest = SigningKey::from_bytes(&[32; 32]);
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
            object_v1: None,
            operations: vec![OperationGrant {
                name: "status".into(),
                owners: vec!["local-owner".into()],
                request: vec![FieldSchema {
                    name: "verbose".into(),
                    schema: ScalarSchema::Bool,
                }],
                reply: vec![FieldSchema {
                    name: "ready".into(),
                    schema: ScalarSchema::Bool,
                }],
                max_request_bytes: 1024,
                max_reply_bytes: 1024,
                max_in_flight: 1,
                max_calls_per_second: 4,
                max_deadline_ms: 1000,
            }],
        }),
        ..Default::default()
    });
    let signed = mvm_core::plan::sign_plan(&plan, &plan_key, "fixture-plan-signer");
    let admitted = mvm_core::plan::verify_plan(
        &signed,
        &[("fixture-plan-signer", &plan_key.verifying_key())],
    )
    .unwrap();
    let binding = OwnerAccessBinding::from_plan(&admitted, "fixture-instance")
        .unwrap()
        .unwrap();
    let mut grant = VerbGrant {
        session_id: "fixture-instance".into(),
        plan_nonce: admitted.nonce.clone(),
        not_after: now + chrono::Duration::seconds(30),
        verbs: vec![],
        drive: None,
        tool_mediation: None,
        application_terminal: None,
        owner_access: Some(binding.clone()),
        sig: vec![],
    };
    grant.sig = host.sign(&grant.signing_bytes()).to_bytes().to_vec();
    let lifecycle = LifecycleBinding {
        instance: grant.session_id.clone(),
        tenant: admitted.tenant.0.clone(),
        image_sha256: admitted.image.sha256.clone(),
        revocation_generation: 1,
        profile: AgentProfile::SealedProd,
    };
    let control = AccessControl::new(lifecycle);
    let budget = SharedBudget::new(Default::default()).unwrap();
    let audit_root = tempfile::tempdir().unwrap();
    let emitter = Arc::new(
        mvm_hostd::audit::emitter::AuditEmitter::with_dir(plan_key.clone(), audit_root.path())
            .unwrap(),
    );
    let mut access = native::NativeAccess::component_fixture(
        Admission {
            signed: &signed,
            keys: &[("fixture-plan-signer", &plan_key.verifying_key())],
            grant: grant.clone(),
            host: &host.verifying_key(),
            control,
            policy: LocalPolicy::new(["local-owner"], ["status"]),
            budget: budget.clone(),
            clock: Arc::new(move || Time {
                unix: now,
                monotonic: Instant::now(),
            }),
        },
        emitter.clone(),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let socket =
        TcpStream::connect_timeout(&listener.local_addr().unwrap(), Duration::from_secs(2))
            .unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    peer.set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut guard = PeerGuard {
        sockets: vec![socket.try_clone().unwrap(), peer.try_clone().unwrap()],
        worker: None,
    };
    let host_anchor = host.verifying_key();
    guard.worker = Some(std::thread::spawn(move || {
        let mut session = AuthenticatedSession::guest(&mut peer, guest, &host_anchor).unwrap();
        for _ in 0..2 {
            let req: GuestRequest = session.read(&mut peer).unwrap();
            let GuestRequest::ProtocolHello {
                host_protocol_version,
                min_supported_version,
                host_version,
                requested_capabilities,
            } = req
            else {
                panic!("expected bounded discovery")
            };
            let response = protocol_hello_response(
                host_protocol_version,
                min_supported_version,
                &host_version,
                &requested_capabilities,
            );
            session.write(&mut peer, &response).unwrap();
        }
        let req: GuestRequest = session.read(&mut peer).unwrap();
        let GuestRequest::DeliverDeclaredOperation(req) = req else {
            panic!("expected typed operation")
        };
        assert!(
            mvm_agentd::vsock::enforce_owner_access_grant(
                &GuestRequest::DeliverDeclaredOperation(req.clone()),
                Some(&grant),
                now
            )
            .is_none()
        );
        let mut desk = mvm_agentd::owner_access::AccessDesk::default();
        desk.install_verified(
            &grant,
            &host_anchor,
            "fixture-instance",
            &grant.plan_nonce,
            now,
        )
        .unwrap();
        let ticket = desk.admit(&req, now, Instant::now()).unwrap();
        let reply = OwnerAccessReply {
            request_id: req.request_id,
            sequence: req.sequence,
            status: OwnerAccessStatus::Ok,
            payload: TypedRecord {
                fields: vec![NamedValue {
                    name: "ready".into(),
                    value: ScalarValue::Bool(true),
                }],
            },
        };
        desk.finish(ticket, &reply, now, Instant::now()).unwrap();
        session
            .write(&mut peer, &GuestResponse::OwnerAccessReply(reply))
            .unwrap();
    }));
    let mut client = OwnerClient::connect(
        socket,
        "fixture-instance",
        host,
        &SigningKey::from_bytes(&[32; 32]).verifying_key(),
        Instant::now() + Duration::from_secs(5),
    )
    .unwrap();
    let request = OwnerAccessRequest {
        policy: binding.policy,
        execution: binding.execution,
        request_id: [33; 32],
        sequence: 1,
        operation: "status".into(),
        owner: "local-owner".into(),
        deadline_unix_ms: now.timestamp_millis() as u64 + 1000,
        payload: TypedRecord {
            fields: vec![NamedValue {
                name: "verbose".into(),
                value: ScalarValue::Bool(false),
            }],
        },
    };
    let response = access
        .call_typed(&mut client, "local-owner", request.clone())
        .unwrap();
    assert_eq!(response.payload.fields[0].value, ScalarValue::Bool(true));
    let before = client.wire_counters();
    assert!(before.sent > 0 && before.received > 0);
    assert!(matches!(
        access.call_typed(&mut client, "local-owner", request),
        Err(Error::Replay)
    ));
    assert_eq!(client.wire_counters(), before);
    assert_eq!(access.counters().completed, 1);
    assert_eq!(budget.usage().unwrap(), (0, 0));
    let root = emitter.publish_root(&admitted.tenant.0).unwrap();
    mvm_contract::merkle::verify_signed_root(&root, &plan_key.verifying_key()).unwrap();
    drop(client);
    drop(guard);
}
