//! Synthetic operator enrollment for exactly two named MVMs in a fresh private installation.
use anyhow::{Context, Result, ensure};
use mvm_client::admission::mailbox_grant::{at_workload_epoch, resolve};
use mvm_contract::protocol::mailbox::{
    CapabilityId, MailboxGrant, MailboxJournalDirectionV1, MailboxMessageKind, RevocationId,
};
use mvm_core::{
    crypto::aead::Key,
    mailbox::{
        outbound::journal_capability_digest, reader::MailboxReaderBootBinding, sign_capability,
    },
};
use mvm_hostd::{
    mailbox_launch::{MailboxTemplate, derive_launch_keys},
    mailbox_process::{
        original_journal::{
            OriginalJournalDestination,
            authorization::{JournalCredentials, PairedJournalAdmission},
        },
        paired_incoming::{IncomingJournalAdmission, IncomingJournalPolicy},
    },
    plan_admission::AdmittedPlan,
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

pub struct Enrollment {
    pub source: MailboxGrant,
    pub destination: MailboxGrant,
    pub policy: IncomingJournalPolicy,
    pub incoming: IncomingJournalAdmission,
}

pub(super) fn original_grant(id: &str, name: &str) -> Result<MailboxGrant> {
    let grant =
        resolve(Some(&id.repeat(32)), Some(30_000), None)?.context("fixture mailbox absent")?;
    at_workload_epoch(grant, name)
}

impl Enrollment {
    pub fn new() -> Result<Self> {
        let source = original_grant("a1", super::A)?;
        let mut destination = original_grant("b2", super::B)?;
        let keys = mvm_core::config::mvm_keys_dir();
        let (_, a) = derive_launch_keys(&keys, &source, &Key::random())?.trust();
        let (_, b) = derive_launch_keys(&keys, &destination, &Key::random())?.trust();
        let mut local = a.capability;
        local.message_kinds = vec![MailboxMessageKind::Cast];
        let mut template = MailboxTemplate {
            header: b.header,
            routing: b.routing,
            capability: b.capability,
        };
        template.header.tenant = local.address.tenant;
        template.header.authorization_domain = local.address.authorization_domain;
        template.capability.address.tenant = local.address.tenant;
        template.capability.address.authorization_domain = local.address.authorization_domain;
        template.capability.message_kinds = vec![MailboxMessageKind::Cast];
        template.routing.hop_limit = 1;
        if cfg!(feature = "large-object-fixture") {
            local.max_payload_bytes = super::fixture::FRAME_LEN as u32;
            template.capability.max_payload_bytes = super::fixture::FRAME_LEN as u32;
        }
        let mut external = template.capability.clone();
        external.subject = a.header.sender;
        external.capability_id = CapabilityId::from_bytes(rand::random());
        external.revocation_id = RevocationId::from_bytes(rand::random());
        let policy = IncomingJournalPolicy {
            source_mailbox: source.mailbox,
            source_epoch: source.epoch,
            source_scope: sign_capability(local, &a.authority_key)?,
            destination_send: sign_capability(external, &b.authority_key)?,
            destination_template: template,
        };
        destination.incoming_journal_policy_sha256_v1 = Some(policy.commitment()?);
        let incoming = IncomingJournalAdmission::install(&destination, &policy)?;
        Ok(Self {
            source,
            destination,
            policy,
            incoming,
        })
    }

    pub fn bind_source(
        &self,
        plan: &AdmittedPlan,
        b: Arc<Mutex<OriginalJournalDestination>>,
        deadline: Instant,
    ) -> Result<(MailboxGrant, PairedJournalAdmission)> {
        let plan = plan.plan();
        ensure!(
            plan.mailbox == Some(self.destination) && plan.image.name == super::B,
            "unexpected destination plan"
        );
        let boot = MailboxReaderBootBinding::load(&mvm_core::config::vm_state_dir(super::B))?;
        boot.check_plan(plan)?;
        let receive = b
            .lock()
            .map_err(|_| anyhow::anyhow!("destination owner unavailable"))?
            .original_receive()?
            .clone();
        let grants = Arc::new(JournalCredentials {
            source_scope: self.policy.source_scope.clone(),
            destination_send: self.policy.destination_send.clone(),
            destination_receive: receive,
        });
        let signed = [
            &grants.source_scope,
            &grants.destination_send,
            &grants.destination_receive,
        ];
        let start = signed
            .iter()
            .map(|g| g.grant.not_before_unix_ms)
            .max()
            .unwrap();
        let end = signed
            .iter()
            .map(|g| g.grant.expires_at_unix_ms)
            .min()
            .unwrap();
        ensure!(end > start, "empty signed credential intersection");
        let direction = MailboxJournalDirectionV1 {
            source: grants.destination_send.grant.subject,
            destination_plan_sha256: hex::decode(
                plan.plan_id
                    .0
                    .strip_prefix("sha256:")
                    .context("bad plan digest")?,
            )?
            .try_into()
            .map_err(|_| anyhow::anyhow!("bad digest length"))?,
            destination_plan_nonce: hex::decode(plan.nonce.as_hex())?
                .try_into()
                .map_err(|_| anyhow::anyhow!("bad nonce length"))?,
            destination_boot: boot.boot,
            address: grants.destination_send.grant.address,
            execution_epoch: self.destination.epoch,
            policy_epoch: grants.destination_send.grant.policy_epoch,
            receipt_key: self.destination.authority_key,
            source_scope_grant_sha256: journal_capability_digest(&grants.source_scope)?,
            send_grant_sha256: journal_capability_digest(&grants.destination_send)?,
            receive_grant_sha256: journal_capability_digest(&grants.destination_receive)?,
            not_before_unix_ms: start,
            expires_at_unix_ms: end,
            max_payload_bytes: super::fixture::FRAME_LEN as u32,
        };
        let mut a = self.source;
        a.outbound_journal_v1 = Some(direction);
        Ok((a, PairedJournalAdmission::new(b, grants, deadline)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mvm_contract::protocol::mailbox::{CapabilityOperations, ContentType, MailboxOperation};
    use mvm_core::mailbox::verify_capability_signature;

    #[test]
    fn installed_pair_issues_forward_cast_without_an_object_return_route() -> Result<()> {
        let _home = mvm_core::util::test_env::TestEnv::new();
        let enrollment = Enrollment::new()?;
        let incoming = &enrollment.policy;
        let source = &incoming.source_scope.grant;
        let guest_message = ContentType::parse("application/vnd.mvm.guest-message", 1)?;
        for (signed, owner) in [
            (&incoming.source_scope, enrollment.source),
            (&incoming.destination_send, enrollment.destination),
        ] {
            verify_capability_signature(
                signed,
                &ed25519_dalek::VerifyingKey::from_bytes(&owner.authority_key)?,
            )?;
            assert_eq!(signed.grant.address.mailbox, owner.mailbox);
            assert_eq!(signed.grant.subject, source.subject);
            assert_eq!(signed.grant.message_kinds, [MailboxMessageKind::Cast]);
            assert_eq!(signed.grant.content_types, [guest_message.clone()]);
            assert_eq!(
                signed.grant.operations,
                CapabilityOperations::from_operations(&[MailboxOperation::Send])
            );
        }
        assert_ne!(incoming.destination_template.header.sender, source.subject);
        assert_ne!(enrollment.source.mailbox, enrollment.destination.mailbox);
        assert_eq!(enrollment.destination.outbound_journal_v1, None);
        assert_eq!(
            enrollment.destination.incoming_journal_policy_sha256_v1,
            Some(incoming.commitment()?)
        );
        Ok(())
    }
}
