//! Private, ephemeral launch provisioning for the development witness.
use crate::Result;
use ed25519_dalek::{SigningKey, VerifyingKey};
use mvm_contract::protocol::mailbox::*;
use mvm_core::{
    crypto::aead::Key,
    mailbox::{
        OpenContext, sign_capability,
        transport::{
            RouteId, RouteLocator, SignedTransportBinding, TransportAssurance,
            TransportBindingClaims, TransportIdentity, TransportPath, sign_transport_binding,
        },
    },
};
use serde::{Deserialize, Serialize};
use spiralcore_mvm_adapter::{CONTENT_TYPE, Direction, DirectionConfig, Policy};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> Result<i64> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Arm {
    Warm,
    Whole,
    Raw,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    One,
    Two,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub stage: Stage,
    pub arm: Arm,
    pub size: usize,
    pub trial: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Sender,
    Receiver,
    Verify,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Material {
    pub capability: SignedMailboxCapability,
    pub binding: SignedTransportBinding,
    pub signing: Option<[u8; 32]>,
    pub verification: [u8; 32],
    pub encryption: [u8; 32],
    pub authority: [u8; 32],
    pub boot: [u8; 32],
    pub key_id: [u8; 32],
    pub kind: MailboxMessageKind,
    pub reply_to: Option<ReplyRoute>,
}
impl Material {
    pub fn direction(&self) -> Result<Direction> {
        let grant = &self.capability.grant;
        let policy = Policy::new(OpenContext {
            expected_tenant: grant.address.tenant,
            expected_authorization_domain: grant.address.authorization_domain,
            expected_mailbox: grant.address.mailbox,
            expected_sender: grant.subject,
            expected_recipient: grant.address.recipient,
            acting_principal: grant.subject,
            expected_capability_issuer: grant.issuer,
            current_execution_epoch: grant.minimum_execution_epoch,
            current_policy_epoch: grant.policy_epoch,
            required_operation: MailboxOperation::Send,
            now_unix_ms: now_ms()?,
            accepted_content_types: vec![ContentType::parse(CONTENT_TYPE, 1)?],
            revoked_capabilities: Vec::new(),
        })?;
        Ok(Direction::new(DirectionConfig {
            policy,
            capability: self.capability.clone(),
            binding: self.binding.clone(),
            signing_key: self.signing.as_ref().map(SigningKey::from_bytes),
            sender_verifying_key: VerifyingKey::from_bytes(&self.verification)?,
            encryption_key: Key::from_bytes(self.encryption),
            authority_key: VerifyingKey::from_bytes(&self.authority)?,
            boot: BootId::from_bytes(self.boot),
            key_id: self.key_id,
            kind: self.kind,
            reply_to: self.reply_to,
        })?)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Published {
    pub namespace: [u8; 32],
    pub object: [u8; 32],
    pub length: usize,
    pub kind: String,
}
#[derive(Serialize, Deserialize)]
pub struct Bootstrap {
    pub role: Role,
    pub plan: Plan,
    pub utc_seconds: u64,
    pub incoming: Option<Material>,
    pub outgoing: Option<Material>,
    pub published: Vec<Published>,
}
pub struct Pair {
    pub sender: Bootstrap,
    pub receiver: Bootstrap,
}
impl Pair {
    pub fn provision(plan: Plan) -> Result<Self> {
        if !(64..=(16 << 20)).contains(&plan.size) {
            return Err("object size outside live profile".into());
        }
        let now = now_ms()?;
        let authority = SigningKey::from_bytes(&rand::random());
        let a = PrincipalId::from_bytes(rand::random());
        let b = PrincipalId::from_bytes(rand::random());
        let tenant = TenantId::from_bytes(rand::random());
        let domain = AuthorizationDomainId::from_bytes(rand::random());
        let forward_address = MailboxAddress {
            tenant,
            authorization_domain: domain,
            mailbox: MailboxId::from_bytes(rand::random()),
            recipient: b,
        };
        let reverse_address = MailboxAddress {
            mailbox: MailboxId::from_bytes(rand::random()),
            recipient: a,
            ..forward_address
        };
        let reverse_capability = CapabilityId::from_bytes(rand::random());
        let recipe = Provision {
            now,
            authority: &authority,
            issuer: PrincipalId::from_bytes(rand::random()),
        };
        let forward = recipe.material(DirectionSpec {
            sender: a,
            address: forward_address,
            capability: CapabilityId::from_bytes(rand::random()),
            kind: MailboxMessageKind::Call,
            reply: Some(ReplyRoute {
                address: reverse_address,
                capability_id: reverse_capability,
            }),
        })?;
        let reverse = recipe.material(DirectionSpec {
            sender: b,
            address: reverse_address,
            capability: reverse_capability,
            kind: MailboxMessageKind::Reply,
            reply: None,
        })?;
        let mut receive_call = forward.clone();
        receive_call.signing = None;
        let mut receive_reply = reverse.clone();
        receive_reply.signing = None;
        let utc_seconds = u64::try_from(now / 1000)?;
        Ok(Self {
            sender: Bootstrap {
                role: Role::Sender,
                plan: plan.clone(),
                utc_seconds,
                incoming: Some(receive_reply),
                outgoing: Some(forward),
                published: Vec::new(),
            },
            receiver: Bootstrap {
                role: Role::Receiver,
                plan,
                utc_seconds,
                incoming: Some(receive_call),
                outgoing: Some(reverse),
                published: Vec::new(),
            },
        })
    }
}

struct Provision<'a> {
    now: i64,
    authority: &'a SigningKey,
    issuer: PrincipalId,
}
struct DirectionSpec {
    sender: PrincipalId,
    address: MailboxAddress,
    capability: CapabilityId,
    kind: MailboxMessageKind,
    reply: Option<ReplyRoute>,
}
impl Provision<'_> {
    fn material(&self, spec: DirectionSpec) -> Result<Material> {
        let signing = SigningKey::from_bytes(&rand::random());
        let epoch = ExecutionEpoch::new(1)?;
        let capability = sign_capability(
            MailboxCapabilityGrant {
                protocol_version: ProtocolVersion::V1,
                signature_algorithm: SignatureAlgorithm::ED25519,
                capability_id: spec.capability,
                issuer: self.issuer,
                subject: spec.sender,
                address: spec.address,
                operations: CapabilityOperations::from_operations(&[MailboxOperation::Send]),
                message_kinds: vec![spec.kind],
                content_types: vec![ContentType::parse(CONTENT_TYPE, 1)?],
                max_payload_bytes: 65536,
                max_messages_per_minute: 4096,
                max_outstanding_calls: 1,
                max_queue_contribution_bytes: 262144,
                minimum_execution_epoch: epoch,
                maximum_execution_epoch: epoch,
                policy_epoch: 1,
                not_before_unix_ms: self.now.checked_sub(10000).ok_or("time underflow")?,
                expires_at_unix_ms: self.now.checked_add(1_800_000).ok_or("time overflow")?,
                revocation_id: RevocationId::from_bytes(rand::random()),
                delegation_depth: 0,
            },
            self.authority,
        )?;
        let binding = sign_transport_binding(
            TransportBindingClaims {
                protocol_version: ProtocolVersion::V1,
                signature_algorithm: SignatureAlgorithm::ED25519,
                binding_id: TransportBindingId::from_bytes(rand::random()),
                issuer: self.issuer,
                address: spec.address,
                execution_epoch: epoch,
                transport_identity: TransportIdentity::from_bytes(rand::random()),
                route_id: RouteId::from_bytes(rand::random()),
                authorization_domain_route_label: AuthorizationDomainRouteLabel::from_bytes(
                    rand::random(),
                ),
                destination_route_token: DestinationRouteToken::from_bytes(rand::random()),
                path: TransportPath::Direct,
                locator: RouteLocator::parse("vsock:5257")?,
                assurance: TransportAssurance::Development,
                policy_epoch: 1,
                not_before_unix_ms: capability.grant.not_before_unix_ms,
                expires_at_unix_ms: capability.grant.expires_at_unix_ms,
            },
            self.authority,
        )?;
        Ok(Material {
            capability,
            binding,
            signing: Some(signing.to_bytes()),
            verification: signing.verifying_key().to_bytes(),
            encryption: rand::random(),
            authority: self.authority.verifying_key().to_bytes(),
            boot: rand::random(),
            key_id: rand::random(),
            kind: spec.kind,
            reply_to: spec.reply,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provisioning_separates_signers_and_roundtrips_only_private_bootstrap() {
        let plan = Plan {
            stage: Stage::Two,
            arm: Arm::Warm,
            size: 4096,
            trial: 0,
        };
        let pair = Pair::provision(plan.clone()).unwrap();
        assert!(pair.sender.outgoing.as_ref().unwrap().signing.is_some());
        assert!(pair.sender.incoming.as_ref().unwrap().signing.is_none());
        assert!(pair.receiver.outgoing.as_ref().unwrap().signing.is_some());
        assert!(pair.receiver.incoming.as_ref().unwrap().signing.is_none());
        assert_eq!(
            pair.sender.outgoing.as_ref().unwrap().verification,
            pair.receiver.incoming.as_ref().unwrap().verification
        );
        assert_eq!(
            pair.receiver.outgoing.as_ref().unwrap().verification,
            pair.sender.incoming.as_ref().unwrap().verification
        );
        let private = serde_json::to_vec(&pair.sender).unwrap();
        assert!(private.len() < 65536);
        let decoded: Bootstrap = serde_json::from_slice(&private).unwrap();
        decoded.outgoing.unwrap().direction().unwrap();
        let other = Pair::provision(plan).unwrap();
        assert_ne!(
            pair.sender.outgoing.unwrap().boot,
            other.sender.outgoing.unwrap().boot
        );
        assert!(
            Pair::provision(Plan {
                stage: Stage::One,
                arm: Arm::Raw,
                size: 0,
                trial: 0
            })
            .is_err()
        );
    }
}
