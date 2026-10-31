use crate::{Error, Result, content_type};
use mvm_contract::protocol::mailbox::MailboxOperation;
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
}
fn validate(context: &OpenContext) -> Result<()> {
    if context.required_operation != MailboxOperation::Send
        || context.acting_principal != context.expected_sender
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
    pub(crate) fn versioned(&self) -> Result<(OpenContext, u64)> {
        let state = self.state.lock().map_err(|_| Error::State)?;
        Ok((state.context.clone(), state.generation))
    }
    pub(crate) fn check_generation(&self, generation: u64) -> Result<()> {
        let state = self.state.lock().map_err(|_| Error::State)?;
        if state.generation != generation {
            return Err(Error::Policy);
        }
        Ok(())
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
        if policy.generation != generation {
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
            Some(lease) if lease.context == *context && lease.generation == policy.generation => {
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
