use crate::{Error, Result, content_type};
use mvm_contract::protocol::mailbox::{MailboxOperation, TransportBindingId};
use mvm_core::mailbox::OpenContext;
use spiralcore_transfer::endpoint::{Authority, Context};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct Policy {
    pub(crate) state: Arc<Mutex<PolicyState>>,
}
pub(crate) struct PolicyState {
    pub context: OpenContext,
    pub generation: u64,
    revoked_bindings: Vec<TransportBindingId>,
    revocation_fenced: bool,
}

impl PolicyState {
    fn permits(&self, generation: u64) -> bool {
        !self.revocation_fenced && self.generation == generation
    }
}

// Bound policy snapshots and canonical authorization work per endpoint.
const MAX_REVOKED_BINDINGS: usize = 1024;

pub(crate) fn record_binding_revocation(
    revoked: &mut Vec<TransportBindingId>,
    generation: &mut u64,
    id: TransportBindingId,
) -> Result<()> {
    if revoked.contains(&id) {
        return Ok(());
    }
    if revoked.len() >= MAX_REVOKED_BINDINGS {
        return Err(Error::Limit);
    }
    let next = generation.checked_add(1).ok_or(Error::Limit)?;
    revoked.push(id);
    *generation = next;
    Ok(())
}
fn validate(context: &OpenContext) -> Result<()> {
    if !matches!(
        context.required_operation,
        MailboxOperation::Send | MailboxOperation::Reply
    ) || context.acting_principal != context.expected_sender
        || context.accepted_content_types != [content_type()?]
    {
        return Err(Error::Policy);
    }
    Ok(())
}
impl Policy {
    pub fn new(context: OpenContext) -> Result<Self> {
        validate(&context)?;
        Ok(Self {
            state: Arc::new(Mutex::new(PolicyState {
                context,
                generation: 0,
                revoked_bindings: Vec::new(),
                revocation_fenced: false,
            })),
        })
    }
    pub fn snapshot(&self) -> Result<OpenContext> {
        Ok(self.versioned()?.0)
    }
    pub fn replace(&self, context: OpenContext) -> Result<()> {
        validate(&context)?;
        let mut state = self.state.lock().map_err(|_| Error::State)?;
        let generation = state.generation.checked_add(1).ok_or(Error::Limit)?;
        state.context = context;
        state.generation = generation;
        Ok(())
    }
    /// Apply a binding revocation received from trusted control-plane policy.
    /// Ordinary context replacement preserves recorded binding revocations.
    pub fn revoke_transport_binding(&self, id: TransportBindingId) -> Result<()> {
        let mut state = self.state.lock().map_err(|_| Error::State)?;
        if state.revocation_fenced {
            return Err(Error::Policy);
        }
        let PolicyState {
            revoked_bindings,
            generation,
            revocation_fenced,
            ..
        } = &mut *state;
        let result = record_binding_revocation(revoked_bindings, generation, id);
        // A trusted revocation that cannot be represented must stop authority.
        *revocation_fenced = result.is_err();
        result
    }

    pub(crate) fn route_snapshot(&self) -> Result<(OpenContext, u64, Vec<TransportBindingId>)> {
        let state = self.state.lock().map_err(|_| Error::State)?;
        if state.revocation_fenced {
            return Err(Error::Policy);
        }
        Ok((
            state.context.clone(),
            state.generation,
            state.revoked_bindings.clone(),
        ))
    }

    pub(crate) fn versioned(&self) -> Result<(OpenContext, u64)> {
        let state = self.state.lock().map_err(|_| Error::State)?;
        if state.revocation_fenced {
            return Err(Error::Policy);
        }
        Ok((state.context.clone(), state.generation))
    }
    pub(crate) fn check_generation(&self, generation: u64) -> Result<()> {
        let state = self.state.lock().map_err(|_| Error::State)?;
        if !state.permits(generation) {
            return Err(Error::Policy);
        }
        Ok(())
    }
    pub(crate) fn with_send_generations<T>(
        &self,
        generation: u64,
        related: Option<(&Self, u64)>,
        send: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let Some((other, other_generation)) = related else {
            let state = self.state.lock().map_err(|_| Error::State)?;
            if !state.permits(generation) {
                return Err(Error::Policy);
            }
            return send();
        };
        if Arc::ptr_eq(&self.state, &other.state) {
            if generation != other_generation {
                return Err(Error::Policy);
            }
            return self.with_send_generations(generation, None, send);
        }
        // Calls and replies can lock the same policies in opposite logical order.
        let (first, first_generation, second, second_generation) =
            if Arc::as_ptr(&self.state).addr() < Arc::as_ptr(&other.state).addr() {
                (self, generation, other, other_generation)
            } else {
                (other, other_generation, self, generation)
            };
        let first = first.state.lock().map_err(|_| Error::State)?;
        let second = second.state.lock().map_err(|_| Error::State)?;
        if !first.permits(first_generation) || !second.permits(second_generation) {
            return Err(Error::Policy);
        }
        send()
    }

    #[cfg(feature = "live")]
    pub(crate) fn refresh_time(&self, now_unix_ms: i64) -> Result<()> {
        self.state
            .lock()
            .map_err(|_| Error::State)?
            .context
            .now_unix_ms = now_unix_ms;
        Ok(())
    }
}

/// An authority lease minted only by canonical open; construction authorizes nothing.
#[derive(Clone)]
pub struct MailboxAuthority {
    policy: Policy,
    lease: Arc<Mutex<Option<Lease>>>,
}
struct Lease {
    context: Context,
    generation: u64,
    expiry: Option<(i64, Instant)>,
}
impl MailboxAuthority {
    #[cfg(feature = "live")]
    pub(crate) fn clear(&self) -> Result<()> {
        *self.lease.lock().map_err(|_| Error::State)? = None;
        Ok(())
    }
    pub(crate) fn new(policy: Policy) -> Self {
        Self {
            policy,
            lease: Arc::new(Mutex::new(None)),
        }
    }
    pub(crate) fn install(&self, context: Context, generation: u64) -> Result<()> {
        self.install_until(context, generation, None)
    }
    pub(crate) fn install_until(
        &self,
        context: Context,
        generation: u64,
        expiry: Option<(i64, Instant)>,
    ) -> Result<()> {
        // Keep lock order policy -> lease, matching authorize. Replacement cannot
        // race between the generation check and installation.
        let policy = self.policy.state.lock().map_err(|_| Error::State)?;
        if !policy.permits(generation) {
            return Err(Error::Policy);
        }
        let mut lease = self.lease.lock().map_err(|_| Error::State)?;
        *lease = Some(Lease {
            context,
            generation,
            expiry,
        });
        Ok(())
    }
}
impl Authority for MailboxAuthority {
    fn authorize(&self, context: &Context) -> spiralcore_transfer::Result<()> {
        use spiralcore_transfer::Error::Unauthorized;
        let policy = self.policy.state.lock().map_err(|_| Unauthorized)?;
        let lease = self.lease.lock().map_err(|_| Unauthorized)?;
        match lease.as_ref() {
            Some(lease) if lease.context == *context && policy.permits(lease.generation) => {
                if let Some((expiry, deadline)) = lease.expiry {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|_| Unauthorized)?;
                    let now = i64::try_from(now.as_millis()).map_err(|_| Unauthorized)?;
                    if now >= expiry || Instant::now() >= deadline {
                        return Err(Unauthorized);
                    }
                }
                Ok(())
            }
            _ => Err(Unauthorized),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mvm_contract::protocol::mailbox::{
        AuthorizationDomainId, ExecutionEpoch, MailboxId, PrincipalId, TenantId,
    };

    fn policy() -> Policy {
        let sender = PrincipalId::from_bytes([3; 32]);
        Policy::new(OpenContext {
            expected_tenant: TenantId::from_bytes([1; 32]),
            expected_authorization_domain: AuthorizationDomainId::from_bytes([2; 32]),
            expected_mailbox: MailboxId::from_bytes([5; 32]),
            expected_sender: sender,
            expected_recipient: PrincipalId::from_bytes([4; 32]),
            acting_principal: sender,
            expected_capability_issuer: PrincipalId::from_bytes([6; 32]),
            current_execution_epoch: ExecutionEpoch::new(1).unwrap(),
            current_policy_epoch: 1,
            required_operation: MailboxOperation::Send,
            now_unix_ms: 100,
            accepted_content_types: vec![content_type().unwrap()],
            revoked_capabilities: vec![],
        })
        .unwrap()
    }

    #[test]
    fn opposite_policy_revocation_between_snapshot_and_send_admission_emits_nothing() {
        for reverse in [false, true] {
            let request = policy();
            let reply = policy();
            let request_generation = request.versioned().unwrap().1;
            let reply_generation = reply.versioned().unwrap().1;
            let (outbound, generation, other, old) = if reverse {
                (&reply, reply_generation, &request, request_generation)
            } else {
                (&request, request_generation, &reply, reply_generation)
            };
            other
                .revoke_transport_binding(TransportBindingId::from_bytes([7; 32]))
                .unwrap();
            let mut sends = 0;
            let result = outbound.with_send_generations(generation, Some((other, old)), || {
                sends += 1;
                Ok(())
            });
            assert!(matches!(result, Err(Error::Policy)));
            assert_eq!(sends, 0);
        }
    }
}
