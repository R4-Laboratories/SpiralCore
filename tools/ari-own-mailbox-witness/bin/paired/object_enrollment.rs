//! Explicit operator issuance for two synthetic participants in a fresh
//! qualification installation. The returned plan nonces must be used when
//! those same original plans are admitted.
use anyhow::{Context, Result, ensure};
use mvm_client::mailbox::{ObjectPairCredentials, PreparedObjectPair};
use mvm_contract::plan::Nonce;
use mvm_contract::protocol::mailbox::{
    CapabilityId, CapabilityOperations, ContentType, MailboxCapabilityGrant, MailboxGrant,
    MailboxMessageKind, MailboxOperation, PrincipalId, RevocationId, SignedMailboxCapability,
};
use mvm_core::{crypto::aead::Key, mailbox::sign_capability};
use mvm_hostd::mailbox_launch::derive_launch_keys;

pub(super) struct ObjectEnrollment {
    pub prepared: PreparedObjectPair,
    pub caller_nonce: Nonce,
    pub callee_nonce: Nonce,
}

/// Publish no credential or key material. The short summary identifies only the
/// prepared (still unlaunched) original signed-grant projection.
impl ObjectEnrollment {
    pub fn issue() -> Result<Self> {
        super::private_home()?;
        let caller = super::enrollment::original_grant("a1", super::A)?;
        let callee = super::enrollment::original_grant("b2", super::B)?;
        let (credentials, caller_nonce, callee_nonce) = issue_credentials(&caller, &callee)?;
        let prepared = PreparedObjectPair::prepare(
            &caller,
            &caller_nonce,
            &callee,
            &callee_nonce,
            credentials,
        )?;
        Ok(Self {
            prepared,
            caller_nonce,
            callee_nonce,
        })
    }

    pub fn summary(&self) -> Result<serde_json::Value> {
        let (a, b) = self.prepared.selected_grants();
        let stamp = a
            .durable_object_pair_sha256_v1
            .context("missing original pair commitment")?;
        ensure!(
            b.durable_object_pair_sha256_v1 == Some(stamp),
            "recipient pair selection differs"
        );
        Ok(
            serde_json::json!({"pair":hex::encode(stamp),"caller":super::A,"callee":super::B,
          "caller_plan_nonce":self.caller_nonce.as_hex(),"callee_plan_nonce":self.callee_nonce.as_hex(),
          "caller_mailbox":hex::encode(a.mailbox.as_bytes()),"callee_mailbox":hex::encode(b.mailbox.as_bytes()),
          "call_max_bytes":4152,"reply_max_bytes":88,"max_outstanding_calls":1,"launched":false}),
        )
    }
}

fn issue_credentials(
    caller: &MailboxGrant,
    callee: &MailboxGrant,
) -> Result<(ObjectPairCredentials, Nonce, Nonce)> {
    let keys = mvm_core::config::mvm_keys_dir();
    let (_, a) = derive_launch_keys(&keys, caller, &Key::random())?.trust();
    let (_, b) = derive_launch_keys(&keys, callee, &Key::random())?.trust();
    let start = a
        .capability
        .not_before_unix_ms
        .max(b.capability.not_before_unix_ms);
    let end = a
        .capability
        .expires_at_unix_ms
        .min(b.capability.expires_at_unix_ms)
        .min(
            start
                .checked_add(120_000)
                .context("credential validity overflow")?,
        );
    ensure!(
        end > start,
        "original authority validity windows do not overlap"
    );
    // The qualification operator explicitly selects the caller's tenant and
    // domain for these four credentials. The ordinary templates stay untouched.
    let service_scope = (a.header.tenant, a.header.authorization_domain);
    let issue = |template: &MailboxCapabilityGrant,
                 authority: &ed25519_dalek::SigningKey,
                 recipient: PrincipalId,
                 sender: PrincipalId,
                 kind: MailboxMessageKind,
                 op: MailboxOperation,
                 limit: u32|
     -> Result<SignedMailboxCapability> {
        let mut g = template.clone();
        g.address.tenant = service_scope.0;
        g.address.authorization_domain = service_scope.1;
        g.address.recipient = recipient;
        g.subject = sender;
        g.operations = CapabilityOperations::from_operations(&[op]);
        g.message_kinds = vec![kind];
        g.content_types = vec![ContentType::parse("application/spiralcore-object", 1)?];
        ensure!(
            template.max_outstanding_calls >= 1 && template.max_payload_bytes >= limit,
            "original capacity below selected profile"
        );
        g.max_payload_bytes = limit;
        g.max_messages_per_minute = g.max_messages_per_minute.min(60);
        g.max_outstanding_calls = 1;
        g.max_queue_contribution_bytes = g.max_queue_contribution_bytes.min(8192);
        g.not_before_unix_ms = start;
        g.expires_at_unix_ms = end;
        g.capability_id = CapabilityId::from_bytes(rand::random());
        g.revocation_id = RevocationId::from_bytes(rand::random());
        g.delegation_depth = 0;
        Ok(sign_capability(g, authority)?)
    };
    use MailboxMessageKind::{Call, Reply};
    let cs = a.header.sender;
    let ds = b.header.sender;
    Ok((
        ObjectPairCredentials {
            call: issue(
                &b.capability,
                &b.authority_key,
                ds,
                cs,
                Call,
                MailboxOperation::Send,
                4152,
            )?,
            reply: issue(
                &a.capability,
                &a.authority_key,
                cs,
                ds,
                Reply,
                MailboxOperation::Reply,
                88,
            )?,
            receive_at_callee: issue(
                &b.capability,
                &b.authority_key,
                ds,
                ds,
                Call,
                MailboxOperation::Receive,
                4152,
            )?,
            receive_at_caller: issue(
                &a.capability,
                &a.authority_key,
                cs,
                cs,
                Reply,
                MailboxOperation::Receive,
                88,
            )?,
        },
        Nonce::from_bytes(rand::random()),
        Nonce::from_bytes(rand::random()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn four_original_grants_match_only_their_measured_direction_and_original_nonces() -> Result<()>
    {
        let _home = mvm_core::util::test_env::TestEnv::new();
        let a = super::super::enrollment::original_grant("a1", super::super::A)?;
        let b = super::super::enrollment::original_grant("b2", super::super::B)?;
        let (g, an, bn) = issue_credentials(&a, &b)?;
        let ready = PreparedObjectPair::prepare(&a, &an, &b, &bn, g.clone())?;
        let (ag, bg) = ready.selected_grants();
        ensure!(an != bn && ag.durable_object_pair_sha256_v1 == bg.durable_object_pair_sha256_v1);
        for (auth, cred, op, kind, limit) in [
            (
                &b,
                &g.call,
                MailboxOperation::Send,
                MailboxMessageKind::Call,
                4152,
            ),
            (
                &b,
                &g.receive_at_callee,
                MailboxOperation::Receive,
                MailboxMessageKind::Call,
                4152,
            ),
            (
                &a,
                &g.reply,
                MailboxOperation::Reply,
                MailboxMessageKind::Reply,
                88,
            ),
            (
                &a,
                &g.receive_at_caller,
                MailboxOperation::Receive,
                MailboxMessageKind::Reply,
                88,
            ),
        ] {
            mvm_core::mailbox::verify_capability_signature(
                cred,
                &ed25519_dalek::VerifyingKey::from_bytes(&auth.authority_key)?,
            )?;
            ensure!(
                cred.grant.operations == CapabilityOperations::from_operations(&[op])
                    && cred.grant.message_kinds == [kind]
                    && cred.grant.max_payload_bytes == limit
                    && cred.grant.address.mailbox == auth.mailbox
            );
        }
        let mut wrong = g.clone();
        std::mem::swap(&mut wrong.receive_at_callee, &mut wrong.receive_at_caller);
        ensure!(PreparedObjectPair::prepare(&a, &an, &b, &bn, wrong).is_err());
        ensure!(PreparedObjectPair::prepare(&ag, &bn, &bg, &an, g).is_err());
        Ok(())
    }
}
