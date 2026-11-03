//! Bounded FIFO metadata for already authenticated canonical mailbox headers.

use super::{Result, ServiceError};
use mvm_contract::protocol::mailbox::{
    ExecutionEpoch, MailboxAddress, MailboxEnvelopeHeader, MessageId, OrderingScope, PrincipalId,
};
use std::collections::{BTreeMap, HashMap};

/// Boot IDs do not reset one principal's FIFO within the same execution epoch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ScopeKey {
    address: MailboxAddress,
    sender: PrincipalId,
    execution_epoch: ExecutionEpoch,
    ordering_scope: OrderingScope,
}

pub(super) fn scope_key(header: &MailboxEnvelopeHeader) -> ScopeKey {
    ScopeKey {
        address: MailboxAddress {
            tenant: header.tenant,
            authorization_domain: header.authorization_domain,
            mailbox: header.mailbox,
            recipient: header.recipient,
        },
        sender: header.sender,
        execution_epoch: header.execution_epoch,
        ordering_scope: header.ordering_scope.clone(),
    }
}

struct Entry {
    message: MessageId,
    expires_at_unix_ms: i64,
    terminal: bool,
}
impl Entry {
    fn matches(&self, header: &MailboxEnvelopeHeader) -> bool {
        self.message == header.message_id && self.expires_at_unix_ms == header.expires_at_unix_ms
    }
}
struct Scope {
    /// None is an exhausted u64 sequence space, never a wrapped sequence zero.
    next: Option<u64>,
    entries: BTreeMap<u64, Entry>,
}

/// Metadata only: the service authenticates headers, validates deadlines and
/// charges payloads, retained identities and scope checkpoints independently.
/// Cursors persist until an authorized owner/epoch reset; evicting an empty
/// scope here would reset FIFO and admit old sequences. Full is explicit.
/// A terminal held item beyond an unknown gap remains charged and bounded.
/// Metadata or payload overload can still backpressure arrival of a gap repair.
/// No queue-capacity exemption or inferred expiry is provided by this index.
pub(super) struct OrderingIndex {
    scopes: HashMap<ScopeKey, Scope>,
    entries: usize,
    max_scopes: usize,
    max_entries: usize,
}
impl OrderingIndex {
    pub(super) fn new(max_scopes: usize, max_entries: usize) -> Result<Self> {
        if max_scopes == 0 || max_entries == 0 {
            return Err(ServiceError::Limit);
        }
        Ok(Self {
            scopes: HashMap::new(),
            entries: 0,
            max_scopes,
            max_entries,
        })
    }

    pub(super) fn contains_scope(&self, header: &MailboxEnvelopeHeader) -> bool {
        self.scopes.contains_key(&scope_key(header))
    }

    /// Includes known terminal entries held behind a missing lower sequence.
    pub(super) fn contains_entry(&self, header: &MailboxEnvelopeHeader) -> bool {
        self.scopes.get(&scope_key(header)).is_some_and(|scope| {
            scope
                .entries
                .get(&header.sequence.get())
                .is_some_and(|entry| entry.matches(header))
        })
    }

    /// Pure admission preflight; duplicates cannot consume another entry.
    pub(super) fn check(&self, header: &MailboxEnvelopeHeader) -> Result<()> {
        let sequence = header.sequence.get();
        if let Some(scope) = self.scopes.get(&scope_key(header)) {
            if let Some(entry) = scope.entries.get(&sequence) {
                return if entry.matches(header) && !entry.terminal {
                    Ok(())
                } else {
                    Err(ServiceError::Conflict)
                };
            }
            if scope.next.is_none_or(|next| sequence < next) {
                return Err(ServiceError::Conflict);
            }
        } else if self.scopes.len() >= self.max_scopes {
            return Err(ServiceError::Full);
        }
        if self.entries >= self.max_entries {
            return Err(ServiceError::Full);
        }
        Ok(())
    }

    /// Call only after the service's global resource reservation succeeds.
    pub(super) fn insert(&mut self, header: &MailboxEnvelopeHeader) -> Result<()> {
        self.check(header)?;
        let scope = self
            .scopes
            .entry(scope_key(header))
            .or_insert_with(|| Scope {
                next: Some(1),
                entries: BTreeMap::new(),
            });
        let sequence = header.sequence.get();
        if scope.entries.contains_key(&sequence) {
            return Ok(());
        }
        let count = self.entries.checked_add(1).ok_or(ServiceError::Limit)?;
        scope.entries.insert(
            sequence,
            Entry {
                message: header.message_id,
                expires_at_unix_ms: header.expires_at_unix_ms,
                terminal: false,
            },
        );
        self.entries = count;
        Ok(())
    }

    pub(super) fn is_ready(&self, header: &MailboxEnvelopeHeader) -> bool {
        self.scopes.get(&scope_key(header)).is_some_and(|scope| {
            scope.next == Some(header.sequence.get())
                && scope
                    .entries
                    .get(&header.sequence.get())
                    .is_some_and(|entry| entry.matches(header) && !entry.terminal)
        })
    }

    pub(super) fn mark_delivered(&mut self, header: &MailboxEnvelopeHeader) -> Result<()> {
        let scope = self
            .scopes
            .get_mut(&scope_key(header))
            .ok_or(ServiceError::UnknownHandle)?;
        let entry = scope
            .entries
            .get_mut(&header.sequence.get())
            .ok_or(ServiceError::UnknownHandle)?;
        if !entry.matches(header) {
            return Err(ServiceError::Conflict);
        }
        if entry.terminal {
            return Err(ServiceError::Expired);
        }
        if scope.next != Some(header.sequence.get()) {
            return Err(ServiceError::Backpressured);
        }
        entry.terminal = true;
        advance_known_terminal(scope, &mut self.entries)
    }

    /// The service must establish this held item's expiry/terminal disposition.
    /// A missing sequence has no stored fact and must never be expired here.
    pub(super) fn expire(&mut self, header: &MailboxEnvelopeHeader) -> Result<()> {
        let scope = self
            .scopes
            .get_mut(&scope_key(header))
            .ok_or(ServiceError::UnknownHandle)?;
        let entry = scope
            .entries
            .get_mut(&header.sequence.get())
            .ok_or(ServiceError::UnknownHandle)?;
        if !entry.matches(header) {
            return Err(ServiceError::Conflict);
        }
        entry.terminal = true;
        advance_known_terminal(scope, &mut self.entries)
    }

    #[cfg(test)]
    fn entry_count(&self) -> usize {
        self.entries
    }
    #[cfg(test)]
    fn scope_count(&self) -> usize {
        self.scopes.len()
    }
}

fn advance_known_terminal(scope: &mut Scope, entries: &mut usize) -> Result<()> {
    while let Some(next) = scope.next {
        if !scope.entries.get(&next).is_some_and(|entry| entry.terminal) {
            break;
        }
        let count = entries.checked_sub(1).ok_or(ServiceError::State)?;
        scope.entries.remove(&next);
        *entries = count;
        scope.next = next.checked_add(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ServiceError;
    use mvm_contract::protocol::mailbox::*;

    fn header(scope: &str, sequence: u64, id: u8) -> MailboxEnvelopeHeader {
        MailboxEnvelopeHeader {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            tenant: TenantId::from_bytes([1; 32]),
            authorization_domain: AuthorizationDomainId::from_bytes([2; 32]),
            sender: PrincipalId::from_bytes([3; 32]),
            recipient: PrincipalId::from_bytes([4; 32]),
            mailbox: MailboxId::from_bytes([5; 32]),
            sender_boot: BootId::from_bytes([6; 32]),
            execution_epoch: ExecutionEpoch::new(1).unwrap(),
            message_id: MessageId::from_bytes([id; 32]),
            correlation_id: CorrelationId::from_bytes([7; 32]),
            causation_id: CausationId::from_bytes([8; 32]),
            kind: MailboxMessageKind::Cast,
            reply_to: None,
            ordering_scope: OrderingScope::parse(scope).unwrap(),
            sequence: SequenceNumber::new(sequence).unwrap(),
            issued_at_unix_ms: 1,
            expires_at_unix_ms: 100,
            nonce: ReplayNonce::from_bytes([9; 16]),
            content_type: ContentType::parse("application/test", 1).unwrap(),
            capability_id: CapabilityId::from_bytes([10; 32]),
            policy_epoch: 1,
            encryption: EncryptionContext {
                algorithm: EncryptionAlgorithm::AES_256_GCM,
                key_id: [11; 32],
            },
            plaintext_len: 1,
        }
    }

    #[test]
    fn gaps_block_only_own_scope_and_delivery_advances_known_sequence() {
        let mut index = OrderingIndex::new(4, 8).unwrap();
        let second = header("a", 2, 2);
        let first = header("a", 1, 1);
        let independent = header("b", 1, 3);
        index.insert(&second).unwrap();
        assert!(!index.is_ready(&second));
        index.insert(&independent).unwrap();
        assert!(index.is_ready(&independent));
        assert_eq!(
            index.mark_delivered(&second),
            Err(ServiceError::Backpressured)
        );
        index.insert(&first).unwrap();
        assert!(index.is_ready(&first));
        index.mark_delivered(&first).unwrap();
        assert!(index.is_ready(&second));
        index.mark_delivered(&second).unwrap();
        assert_eq!(index.entry_count(), 1);
        assert_eq!(index.scope_count(), 2);
    }

    #[test]
    fn held_expiry_never_fabricates_missing_sender_terminal_fact() {
        let mut index = OrderingIndex::new(4, 8).unwrap();
        let first = header("a", 1, 1);
        let second = header("a", 2, 2);
        let third = header("a", 3, 3);
        index.insert(&second).unwrap();
        index.expire(&second).unwrap();
        assert!(index.contains_entry(&second));
        assert_eq!(index.check(&second), Err(ServiceError::Conflict));
        assert_eq!(index.insert(&second), Err(ServiceError::Conflict));
        assert!(!index.contains_entry(&first));
        assert!(!index.contains_entry(&header("a", 2, 99)));
        index.insert(&third).unwrap();
        assert!(!index.is_ready(&third));
        assert_eq!(index.expire(&first), Err(ServiceError::UnknownHandle));
        assert!(!index.is_ready(&third));
        assert_eq!(index.entry_count(), 2);
        index.insert(&first).unwrap();
        index.mark_delivered(&first).unwrap();
        assert!(index.is_ready(&third));
        assert!(!index.contains_entry(&second));
        assert_eq!(index.entry_count(), 1);
    }

    #[test]
    fn expired_known_head_can_release_next_without_delivering_expired_payload() {
        let mut index = OrderingIndex::new(2, 4).unwrap();
        let first = header("a", 1, 1);
        let second = header("a", 2, 2);
        index.insert(&first).unwrap();
        index.insert(&second).unwrap();
        index.expire(&first).unwrap();
        assert!(!index.is_ready(&first));
        assert!(index.is_ready(&second));
        assert_eq!(index.entry_count(), 1);
    }

    #[test]
    fn preflight_is_pure_and_sequence_identity_conflicts_do_not_mutate() {
        let mut index = OrderingIndex::new(1, 4).unwrap();
        let first = header("a", 1, 1);
        index.check(&first).unwrap();
        assert!(!index.contains_scope(&first));
        assert_eq!(index.scope_count(), 0);
        index.insert(&first).unwrap();
        index.insert(&first).unwrap();
        assert_eq!(index.entry_count(), 1);
        let conflict = header("a", 1, 2);
        assert_eq!(index.check(&conflict), Err(ServiceError::Conflict));
        assert_eq!(index.insert(&conflict), Err(ServiceError::Conflict));
        assert_eq!(index.expire(&conflict), Err(ServiceError::Conflict));
        assert!(index.is_ready(&first));
        index.mark_delivered(&first).unwrap();
        assert_eq!(index.insert(&first), Err(ServiceError::Conflict));
        assert_eq!(index.scope_count(), 1); // Consumed cursor cannot silently reset.
    }

    #[test]
    fn capacities_are_shared_across_scopes_and_preflight_is_atomic() {
        assert!(OrderingIndex::new(0, 1).is_err());
        assert!(OrderingIndex::new(1, 0).is_err());
        let mut index = OrderingIndex::new(1, 2).unwrap();
        let first = header("a", 1, 1);
        let second = header("a", 2, 2);
        index.insert(&first).unwrap();
        index.insert(&second).unwrap();
        assert_eq!(index.insert(&header("a", 3, 3)), Err(ServiceError::Full));
        assert_eq!(index.insert(&header("b", 1, 4)), Err(ServiceError::Full));
        assert_eq!(index.scope_count(), 1);
        assert_eq!(index.entry_count(), 2);
        index.mark_delivered(&first).unwrap();
        index.insert(&header("a", 3, 3)).unwrap();
    }

    #[test]
    fn canonical_key_partitions_all_authority_fields_but_not_sender_boot() {
        let mut index = OrderingIndex::new(9, 24).unwrap();
        let blocked = header("a", 2, 1);
        index.insert(&blocked).unwrap();
        let mut variants = Vec::new();
        let base = header("a", 1, 2);
        let mut h = base.clone();
        h.tenant = TenantId::from_bytes([20; 32]);
        variants.push(h);
        let mut h = base.clone();
        h.authorization_domain = AuthorizationDomainId::from_bytes([20; 32]);
        variants.push(h);
        let mut h = base.clone();
        h.mailbox = MailboxId::from_bytes([20; 32]);
        variants.push(h);
        let mut h = base.clone();
        h.recipient = PrincipalId::from_bytes([20; 32]);
        variants.push(h);
        let mut h = base.clone();
        h.sender = PrincipalId::from_bytes([20; 32]);
        variants.push(h);
        let mut h = base.clone();
        h.execution_epoch = ExecutionEpoch::new(2).unwrap();
        variants.push(h);
        let mut h = base.clone();
        h.ordering_scope = OrderingScope::parse("b").unwrap();
        variants.push(h);
        for h in variants {
            index.insert(&h).unwrap();
            assert!(index.is_ready(&h));
        }
        let mut restarted = blocked.clone();
        restarted.sender_boot = BootId::from_bytes([99; 32]);
        assert_eq!(scope_key(&restarted), scope_key(&blocked));
        assert!(!index.is_ready(&restarted));
    }

    #[test]
    fn maximum_sequence_exhausts_without_wraparound() {
        let mut index = OrderingIndex::new(1, 2).unwrap();
        let last = header("a", u64::MAX, 1);
        index.insert(&last).unwrap();
        index.scopes.get_mut(&scope_key(&last)).unwrap().next = Some(u64::MAX);
        assert!(index.is_ready(&last));
        index.mark_delivered(&last).unwrap();
        assert!(!index.is_ready(&last));
        assert_eq!(index.insert(&last), Err(ServiceError::Conflict));
        assert_eq!(index.entry_count(), 0);
    }
}
