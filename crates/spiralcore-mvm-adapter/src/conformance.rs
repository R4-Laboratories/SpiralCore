//! Host-only synthetic fixture using MVM's real canonical in-memory transport.
//!
//! Enabled explicitly for tests and measurements. This is not key provisioning,
//! a durable mailbox inbox, a vsock service or a network transport benchmark.
pub use crate::Counters;
use crate::{Direction, DirectionConfig, Error, Policy, Result, content_type};
use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::{
    crypto::aead::Key,
    mailbox::{
        OpenContext, sign_capability,
        transport::{
            CancellationToken, InMemoryPeerTransport, RouteId, RouteLocator, TransportAssurance,
            TransportBindingClaims, TransportIdentity, TransportPath, sign_transport_binding,
        },
    },
};
use spiralcore_transfer::{
    Id,
    endpoint::{ColdGraphPolicy, Context, Endpoint, Limits},
    protocol::Frame,
};
use std::path::{Path, PathBuf};

pub struct Harness {
    root: PathBuf,
    context: Context,
    forward: Direction,
    reverse: Direction,
    endpoint: Option<Endpoint<crate::MailboxAuthority>>,
    transport: InMemoryPeerTransport,
    cancel: CancellationToken,
    counters: Counters,
}
impl Harness {
    pub fn new(root: &Path) -> Result<Self> {
        let mut forward = Direction::new(config(MailboxMessageKind::Call)?)?;
        let reverse = Direction::new(config(MailboxMessageKind::Reply)?)?;
        let transport = InMemoryPeerTransport::default();
        transport
            .register_route(forward.route_id())
            .map_err(Error::Transport)?;
        transport
            .register_route(reverse.route_id())
            .map_err(Error::Transport)?;
        let cancel = CancellationToken::default();
        // An actual authenticated probe establishes the namespace before a
        // WarmSession is constructed. It is not dispatched or acknowledged as
        // mailbox acceptance. All its bytes are included in setup counters.
        let probe = Frame {
            kind: 5,
            transfer: rand::random(),
            seq: 0,
            body: Vec::new(),
        };
        forward.send(&probe, &transport, &cancel)?;
        let wire = one(&transport, forward.route_id())?;
        let context = forward.open(&wire)?.context().clone();
        let endpoint = endpoint(root, forward.authority())?;
        let counters = forward.counters().checked_add(reverse.counters())?;
        Ok(Self {
            root: root.to_path_buf(),
            context,
            forward,
            reverse,
            endpoint: Some(endpoint),
            transport,
            cancel,
            counters,
        })
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn counters(&self) -> Counters {
        self.counters
    }
    pub fn forward_policy(&self) -> Policy {
        self.forward.policy()
    }
    pub fn reverse_policy(&self) -> Policy {
        self.reverse.policy()
    }

    pub fn exchange(&mut self, frames: &[Frame]) -> spiralcore_transfer::Result<Vec<Frame>> {
        let result = self.exchange_inner(frames);
        // Receive counters include rejected input; send counters include only
        // frames actually queued. Preserve both even if an exchange fails.
        self.counters = self
            .forward
            .counters()
            .checked_add(self.reverse.counters())
            .map_err(application_error)?;
        result.map_err(application_error)
    }
    fn exchange_inner(&mut self, frames: &[Frame]) -> Result<Vec<Frame>> {
        if frames.is_empty() || frames.len() > 4096 {
            return Err(Error::Limit);
        }
        let mut replies = Vec::with_capacity(frames.len());
        for request in frames {
            self.forward.send(request, &self.transport, &self.cancel)?;
            let metadata = self.forward.last_sent.ok_or(Error::State)?;
            let wire = one(&self.transport, self.forward.route_id())?;
            let verified = self.forward.open(&wire)?;
            if verified.context() != &self.context {
                return Err(Error::Policy);
            }
            let reply = match verified.dispatch(self.endpoint.as_mut().ok_or(Error::State)?, 0) {
                Ok(reply) => reply,
                Err(Error::Frame(error)) => {
                    request.reply(127, (error as u16).to_le_bytes().to_vec())
                }
                Err(error) => return Err(error),
            };
            self.reverse
                .send_correlated(&reply, &self.transport, &self.cancel, Some(metadata))?;
            let wire = one(&self.transport, self.reverse.route_id())?;
            let opened = self.reverse.open(&wire)?;
            let received = opened.frame();
            let kind_matches = received.kind == 127
                || match request.kind {
                    1 | 4 => matches!(received.kind, 101 | 103),
                    2 => received.kind == 102,
                    3 => matches!(received.kind, 103 | 104),
                    5 | 6 => received.kind == 105,
                    _ => false,
                };
            if received.transfer != request.transfer || received.seq != request.seq || !kind_matches
            {
                return Err(Error::Frame(spiralcore_transfer::Error::Sequence));
            }
            let terminal = matches!(received.kind, 103 | 127);
            replies.push(received.clone());
            if terminal {
                break;
            }
        }
        Ok(replies)
    }
    pub fn read_object(&mut self, id: Id) -> Result<Vec<u8>> {
        self.endpoint
            .as_mut()
            .ok_or(Error::State)?
            .read_object(&self.context, id)
            .map_err(Error::Frame)
    }
    /// Restarts only the object store, keeping process-local crypto/replay state.
    /// It does not establish durable accepted-message semantics across a crash.
    pub fn reopen(&mut self) -> Result<()> {
        drop(self.endpoint.take());
        self.endpoint = Some(endpoint(&self.root, self.forward.authority())?);
        Ok(())
    }
}
fn endpoint(
    root: &Path,
    authority: crate::MailboxAuthority,
) -> Result<Endpoint<crate::MailboxAuthority>> {
    let mut endpoint = Endpoint::open_with_policy(
        root,
        Limits::default(),
        authority,
        ColdGraphPolicy::Disabled,
    )?;
    endpoint.set_codec_cache_enabled(true)?;
    Ok(endpoint)
}
fn one(transport: &InMemoryPeerTransport, route: RouteId) -> Result<Vec<u8>> {
    let mut frames = transport.drain(route).map_err(Error::Transport)?;
    if frames.len() != 1 {
        return Err(Error::State);
    }
    frames.pop().ok_or(Error::State)
}
fn application_error(error: Error) -> spiralcore_transfer::Error {
    match error {
        Error::Frame(error) => error,
        Error::Limit | Error::ReplayCapacity => spiralcore_transfer::Error::Limit,
        Error::Policy | Error::Mailbox(_) | Error::Route(_) | Error::Outbound(_) => {
            spiralcore_transfer::Error::Unauthorized
        }
        _ => spiralcore_transfer::Error::Storage,
    }
}

// Public fixture identities and fresh ephemeral keys. No test signing key is
// compiled into the default library; these keys have no external authority.
fn config(kind: MailboxMessageKind) -> Result<DirectionConfig> {
    let reply = kind == MailboxMessageKind::Reply;
    let sender = PrincipalId::from_bytes([if reply { 3 } else { 2 }; 32]);
    let recipient = PrincipalId::from_bytes([if reply { 2 } else { 3 }; 32]);
    let address = MailboxAddress {
        tenant: TenantId::from_bytes([19; 32]),
        authorization_domain: AuthorizationDomainId::from_bytes([1; 32]),
        mailbox: MailboxId::from_bytes([if reply { 14 } else { 4 }; 32]),
        recipient,
    };
    let content = content_type()?;
    let epoch = ExecutionEpoch::new(7).map_err(|_| Error::Policy)?;
    let issuer = PrincipalId::from_bytes([12; 32]);
    let authority = SigningKey::from_bytes(&rand::random());
    let signing_key = SigningKey::from_bytes(&rand::random());
    let capability = sign_capability(
        MailboxCapabilityGrant {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            capability_id: CapabilityId::from_bytes([if reply { 11 } else { 10 }; 32]),
            issuer,
            subject: sender,
            address,
            operations: CapabilityOperations::from_operations(&[MailboxOperation::Send]),
            message_kinds: vec![kind],
            content_types: vec![content.clone()],
            max_payload_bytes: 65_536,
            max_messages_per_minute: 4096,
            max_outstanding_calls: 4,
            max_queue_contribution_bytes: 262_144,
            minimum_execution_epoch: epoch,
            maximum_execution_epoch: epoch,
            policy_epoch: 4,
            not_before_unix_ms: 900,
            expires_at_unix_ms: 3_000,
            revocation_id: RevocationId::from_bytes([if reply { 14 } else { 13 }; 32]),
            delegation_depth: 0,
        },
        &authority,
    )
    .map_err(Error::Mailbox)?;
    let policy = Policy::new(OpenContext {
        expected_tenant: address.tenant,
        expected_authorization_domain: address.authorization_domain,
        expected_mailbox: address.mailbox,
        expected_sender: sender,
        expected_recipient: recipient,
        acting_principal: sender,
        expected_capability_issuer: issuer,
        current_execution_epoch: epoch,
        current_policy_epoch: 4,
        required_operation: MailboxOperation::Send,
        now_unix_ms: 1_500,
        accepted_content_types: vec![content],
        revoked_capabilities: vec![],
    })?;
    let binding = sign_transport_binding(
        TransportBindingClaims {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            binding_id: TransportBindingId::from_bytes([if reply { 28 } else { 18 }; 32]),
            issuer,
            address,
            execution_epoch: epoch,
            transport_identity: TransportIdentity::from_bytes([if reply { 29 } else { 19 }; 32]),
            route_id: RouteId::from_bytes([if reply { 30 } else { 20 }; 32]),
            authorization_domain_route_label: AuthorizationDomainRouteLabel::from_bytes([15; 32]),
            destination_route_token: DestinationRouteToken::from_bytes(
                [if reply { 26 } else { 16 }; 32],
            ),
            path: TransportPath::Direct,
            locator: RouteLocator::parse(if reply { "memory:reply" } else { "memory:call" })
                .map_err(Error::Route)?,
            assurance: TransportAssurance::Development,
            policy_epoch: 4,
            not_before_unix_ms: 900,
            expires_at_unix_ms: 3_000,
        },
        &authority,
    )
    .map_err(Error::Route)?;
    Ok(DirectionConfig {
        policy,
        capability,
        binding,
        sender_verifying_key: signing_key.verifying_key(),
        signing_key: Some(signing_key),
        encryption_key: Key::from_bytes(rand::random()),
        authority_key: authority.verifying_key(),
        boot: BootId::from_bytes(rand::random()),
        key_id: rand::random(),
        kind,
        reply_to: if reply {
            None
        } else {
            Some(ReplyRoute {
                address: MailboxAddress {
                    mailbox: MailboxId::from_bytes([14; 32]),
                    recipient: sender,
                    ..address
                },
                capability_id: CapabilityId::from_bytes([11; 32]),
            })
        },
    })
}
