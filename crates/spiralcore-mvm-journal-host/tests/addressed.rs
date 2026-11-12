#[path = "../../spiralcore-mvm-adapter/tests/common/mod.rs"]
mod common;

use cpu_time::{ProcessTime, ThreadTime};
use ed25519_dalek::SigningKey;
use journal_contract::protocol::mailbox as dst;
use journal_core::{
    crypto::aead::Key,
    mailbox::{
        OpenContext,
        journal::{
            FileJournalAnchor, HistoryKeyCustody, HistoryKeyRef, JournalError, JournalLimits,
            MailboxJournal,
        },
    },
};
use mvm_contract::protocol::mailbox as src;
use mvm_core::mailbox::{
    encode_envelope, sign_capability,
    transport::{CancellationToken, RouteLocator, sign_transport_binding},
};
use mvm_hostd::mailbox_service::{MailboxLimits, MailboxPolicy, durable::DurableMailboxService};
use sha2::{Digest, Sha256};
use spiralcore_mvm_adapter::{
    issuance::IssuancePolicy,
    service::{MailboxBinding, ServiceBindingConfig, ServiceError, ServiceTime},
    service_sender::{MessageOptions, prepare},
};
use spiralcore_mvm_journal_host::{Error, send_cast};
use spiralcore_transfer::protocol::Frame;
use std::{
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn now() -> ServiceTime {
    ServiceTime {
        unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
        monotonic_ms: 0,
    }
}

struct Custody(HistoryKeyRef);
impl HistoryKeyCustody for Custody {
    fn load_existing(&self, reference: &HistoryKeyRef) -> Result<Key, JournalError> {
        if reference != &self.0 {
            return Err(JournalError::HistoryKeyMismatch);
        }
        Ok(Key::from_bytes([90; 32]))
    }
}

struct Pair {
    source: ServiceBindingConfig,
    destination: OpenContext,
    address: dst::MailboxAddress,
    receive: dst::SignedMailboxCapability,
    authority: SigningKey,
}
fn pair(path: &Path, instant: ServiceTime) -> Pair {
    let authority = SigningKey::from_bytes(&[22; 32]);
    let mut source =
        ServiceBindingConfig::from_direction(common::config(src::MailboxMessageKind::Cast))
            .unwrap();
    let content = src::ContentType::parse("application/vnd.mvm.guest-message", 1).unwrap();
    source.context.now_unix_ms = instant.unix_ms;
    source.context.accepted_content_types = vec![content.clone()];
    let mut grant = source.capability.grant;
    grant.content_types = vec![content];
    grant.not_before_unix_ms = instant.unix_ms - 1_000;
    grant.expires_at_unix_ms = instant.unix_ms + 60_000;
    source.capability = sign_capability(grant, &authority).unwrap();
    let mut route = source.binding.claims;
    route.not_before_unix_ms = instant.unix_ms - 1_000;
    route.expires_at_unix_ms = instant.unix_ms + 60_000;
    route.locator = RouteLocator::parse(format!("unix:{}", path.display())).unwrap();
    source.binding = sign_transport_binding(route, &authority).unwrap();

    let send: dst::SignedMailboxCapability =
        serde_json::from_value(serde_json::to_value(&source.capability).unwrap()).unwrap();
    let grant = send.grant;
    let address = grant.address;
    let destination = OpenContext {
        expected_tenant: address.tenant,
        expected_authorization_domain: address.authorization_domain,
        expected_mailbox: address.mailbox,
        expected_sender: grant.subject,
        expected_recipient: address.recipient,
        acting_principal: grant.subject,
        expected_capability_issuer: grant.issuer,
        current_execution_epoch: grant.minimum_execution_epoch,
        current_policy_epoch: grant.policy_epoch,
        required_operation: dst::MailboxOperation::Send,
        now_unix_ms: instant.unix_ms,
        accepted_content_types: grant.content_types.clone(),
        revoked_capabilities: vec![],
    };
    let receive = journal_core::mailbox::sign_capability(
        dst::MailboxCapabilityGrant {
            capability_id: dst::CapabilityId::from_bytes([71; 32]),
            revocation_id: dst::RevocationId::from_bytes([72; 32]),
            operations: dst::CapabilityOperations::from_operations(&[
                dst::MailboxOperation::Receive,
            ]),
            subject: address.recipient,
            ..grant
        },
        &authority,
    )
    .unwrap();
    Pair {
        source,
        destination,
        address,
        receive,
        authority,
    }
}

#[test]
fn source_grant_to_b_journal_restart_and_same_receipt() {
    let dir = tempfile::Builder::new()
        .prefix("scjr")
        .tempdir_in("/tmp")
        .unwrap();
    let socket = dir.path().join("b.sock");
    let pair = pair(&socket, now());
    let history = HistoryKeyRef::new(
        pair.address,
        pair.destination.current_execution_epoch,
        [30; 32],
        1,
    )
    .unwrap();
    let key = pair.authority.verifying_key();
    let signer = pair.source.sender_verifying_key;
    let start = |destination: &OpenContext| {
        let anchor =
            FileJournalAnchor::open(&dir.path().join("anchor"), pair.address, [91; 32]).unwrap();
        let journal = MailboxJournal::open_local_with_custody(
            &dir.path().join("journal"),
            history,
            key,
            JournalLimits {
                max_records: 4,
                max_bytes: 65_536,
            },
            &Custody(history),
            &anchor,
            dst::MailboxRetentionPolicy::new(60_000).unwrap(),
        )
        .unwrap();
        DurableMailboxService::bind_local(
            &socket,
            MailboxLimits {
                max_depth: 4,
                max_frame_bytes: 65_536,
            },
            MailboxPolicy {
                context: destination.clone(),
                encryption_key: Key::from_bytes([23; 32]),
                sender_key: signer,
                authority_key: key,
            },
            pair.authority.clone(),
            journal,
            anchor,
        )
        .unwrap()
    };
    let source_context = pair.source.context.clone();
    let binding_id = pair.source.binding.claims.binding_id;
    let binding = MailboxBinding::new(pair.source, None).unwrap();
    let frame = Frame {
        kind: 2,
        transfer: [67; 32],
        seq: 2,
        body: (0..257).map(|x| x as u8).collect(),
    };
    let begin_setup = Instant::now();
    let setup_cpu = ThreadTime::now();
    let encoded = frame.encode().unwrap();
    let options = MessageOptions::fresh_for_binding(
        &binding,
        source_context.accepted_content_types[0].clone(),
        now(),
        now().unix_ms + 30_000,
        IssuancePolicy::Exact,
    )
    .unwrap();
    let prepared = prepare(&binding, &encoded, &options, now()).unwrap();
    let setup_wall = begin_setup.elapsed();
    let setup_cpu = setup_cpu.elapsed();
    let cancel = CancellationToken::default();
    let attempt = || {
        send_cast(
            &binding,
            &prepared,
            &cancel,
            now(),
            Instant::now() + Duration::from_secs(2),
        )
    };
    let sealed = encode_envelope(prepared.envelope()).unwrap();

    // Both source and destination keys are held by their original test endpoints.
    // A differently admitted B policy refuses the authentic, but out-of-scope, Send.
    let mut stale = pair.destination.clone();
    stale.current_policy_epoch += 1;
    let refused = start(&stale);
    assert!(matches!(attempt(), Err(Error::Receipt(_))));
    drop(refused);

    let service = start(&pair.destination);
    let measure = |label| {
        let start = Instant::now();
        let process = ProcessTime::now();
        let sender = ThreadTime::now();
        let result = attempt().unwrap();
        println!(
            "turn={label} body={} frame={} envelope={} sent={} received={} wall_us={} sender_thread_cpu_us={} process_cpu_us={}",
            frame.body.len(),
            encoded.len(),
            sealed.len(),
            result.bytes_to_journal,
            result.bytes_from_journal,
            start.elapsed().as_micros(),
            sender.elapsed().as_micros(),
            process.elapsed().as_micros()
        );
        result
    };
    let first = measure("admit");
    assert_eq!(
        first.receipt.receipt.protocol_version,
        dst::ProtocolVersion::V2
    );
    assert_eq!(
        first.receipt.receipt.outcome,
        dst::DeliveryOutcome::AcceptedLocalDurable
    );
    assert_eq!(first.receipt.receipt.address, pair.address);
    drop(service);
    assert!(matches!(attempt(), Err(Error::Unavailable)));
    let service = start(&pair.destination);
    let retry = measure("retry");
    assert_eq!(
        serde_json::to_vec(&retry.receipt).unwrap(),
        serde_json::to_vec(&first.receipt).unwrap()
    );
    assert_eq!(encode_envelope(prepared.envelope()).unwrap(), sealed);
    assert_eq!(first.bytes_to_journal, retry.bytes_to_journal);
    assert_eq!(first.bytes_from_journal, retry.bytes_from_journal);

    let mut reader = pair.destination.clone();
    reader.acting_principal = pair.address.recipient;
    reader.required_operation = dst::MailboxOperation::Receive;
    let mut foreign = reader.clone();
    foreign.expected_recipient = dst::PrincipalId::from_bytes([47; 32]);
    assert!(service.receive_next(&pair.receive, &foreign).is_err());
    let message = service
        .receive_next(&pair.receive, &reader)
        .unwrap()
        .unwrap();
    assert_eq!(message.payload(), encoded);
    assert_eq!(Frame::decode(message.payload()).unwrap(), frame);
    service
        .acknowledge(message.identity(), &pair.receive, &reader)
        .unwrap();
    assert!(
        service
            .receive_next(&pair.receive, &reader)
            .unwrap()
            .is_none()
    );
    println!(
        "setup_wall_us={} setup_sender_thread_cpu_us={} body_sha256={:x} frame_sha256={:x}",
        setup_wall.as_micros(),
        setup_cpu.as_micros(),
        Sha256::digest(&frame.body),
        Sha256::digest(&encoded)
    );

    let mut foreign = source_context.clone();
    foreign.expected_recipient = src::PrincipalId::from_bytes([47; 32]);
    binding.replace_policy(foreign).unwrap();
    assert!(matches!(
        attempt(),
        Err(Error::Service(ServiceError::Unauthorized))
    ));
    binding.replace_policy(source_context).unwrap();
    binding.revoke_transport_binding(binding_id).unwrap();
    assert!(matches!(
        attempt(),
        Err(Error::Service(ServiceError::Unauthorized))
    ));
}
