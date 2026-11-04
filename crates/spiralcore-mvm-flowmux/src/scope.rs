//! Complete admitted facts. Construction is not permission: the host's current
//! signed-plan/capability authority must approve these before codec allocation.
use crate::{Binding, Error, Result};
use spiralcore_transfer::{endpoint::Context, hash};
use std::net::SocketAddr;
#[derive(Clone, PartialEq, Eq)]
pub struct ScopeFacts {
    pub object_profile: u16,
    pub policy_sha256: [u8; 32],
    pub plan_id: String,
    pub plan_version: u32,
    pub plan_nonce: [u8; 16],
    pub bundle_sha256: [u8; 32],
    pub tenant: String,
    pub instance: String,
    pub session: String,
    pub guest_key: [u8; 32],
    pub stream_id: u32,
    pub requested_host: String,
    pub requested_port: u16,
    pub actual_peer: SocketAddr,
    pub route: String,
    pub initiator_spki: [u8; 32],
    pub acceptor_spki: [u8; 32],
    pub tls_exporter: [u8; 32],
}
impl std::fmt::Debug for ScopeFacts {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ScopeFacts").finish_non_exhaustive()
    }
}
impl ScopeFacts {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        if !matches!(self.object_profile, 1 | 2)
            || self.plan_version == 0
            || self.stream_id == 0
            || self.requested_port == 0
            || self.actual_peer.port() == 0
            || [
                self.policy_sha256,
                self.bundle_sha256,
                self.guest_key,
                self.initiator_spki,
                self.acceptor_spki,
                self.tls_exporter,
            ]
            .contains(&[0; 32])
        {
            return Err(Error::InvalidBinding);
        }
        let mut bytes = b"spiralcore.flow.scope.v1\0".to_vec();
        for value in [
            &self.plan_id,
            &self.tenant,
            &self.instance,
            &self.session,
            &self.requested_host,
            &self.route,
        ] {
            if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
                return Err(Error::InvalidBinding);
            }
            bytes.extend_from_slice(&(value.len() as u16).to_be_bytes());
            bytes.extend_from_slice(value.as_bytes());
        }
        for value in [
            self.policy_sha256,
            self.bundle_sha256,
            self.guest_key,
            self.initiator_spki,
            self.acceptor_spki,
            self.tls_exporter,
        ] {
            bytes.extend_from_slice(&value);
        }
        bytes.extend_from_slice(&self.plan_version.to_be_bytes());
        bytes.extend_from_slice(&self.plan_nonce);
        bytes.extend_from_slice(&self.stream_id.to_be_bytes());
        bytes.extend_from_slice(&self.requested_port.to_be_bytes());
        bytes.extend_from_slice(&self.object_profile.to_be_bytes());
        match self.actual_peer.ip() {
            std::net::IpAddr::V4(ip) => {
                bytes.push(4);
                bytes.extend_from_slice(&ip.octets());
            }
            std::net::IpAddr::V6(ip) => {
                bytes.push(6);
                bytes.extend_from_slice(&ip.octets());
            }
        }
        bytes.extend_from_slice(&self.actual_peer.port().to_be_bytes());
        if let SocketAddr::V6(address) = self.actual_peer {
            bytes.extend_from_slice(&address.flowinfo().to_be_bytes());
            bytes.extend_from_slice(&address.scope_id().to_be_bytes());
        }
        Ok(bytes)
    }
    pub fn commitment(&self) -> Result<[u8; 32]> {
        Ok(hash(
            b"spiralcore.flow.scope.commit.v1\0",
            &[&self.canonical_bytes()?],
        ))
    }
    pub fn context(&self) -> Result<Context> {
        let commitment = self.commitment()?;
        let hex = |bytes: &[u8]| {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        Ok(Context {
            tenant: self.tenant.clone(),
            domain: "canonical-mvm-flowmux-v1".into(),
            sender: hex(&self.initiator_spki),
            recipient: hex(&self.acceptor_spki),
            mailbox: format!("stream:{}", self.stream_id),
            capability: hex(&self.policy_sha256),
            authorization_scope: hex(&commitment),
            policy_epoch: 0,
            execution_epoch: 0,
        })
    }
    pub fn binding(&self) -> Result<Binding> {
        Ok(Binding {
            flow: self.context()?.namespace()?,
            policy_sha256: self.policy_sha256,
            scope_commitment: self.commitment()?,
        })
    }
}

use crate::Role;
use spiralcore_transfer::endpoint::Authority;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone)]
pub struct PinnedScopePolicy {
    inner: Arc<Mutex<PolicyState>>,
}
struct PolicyState {
    max: usize,
    generation: u64,
    next: u64,
    entries: Vec<Enrollment>,
}
struct Enrollment {
    commitment: [u8; 32],
    peer: [u8; 32],
    role: Role,
    until: Instant,
    id: u64,
}
#[derive(Clone)]
pub struct ScopeAuthority {
    policy: PinnedScopePolicy,
    facts: ScopeFacts,
    context: Context,
    generation: u64,
    id: u64,
    role: Role,
    peer: [u8; 32],
}
fn denied() -> Error {
    Error::Core(spiralcore_transfer::Error::Unauthorized)
}
impl PinnedScopePolicy {
    /// An empty enrollment set refuses every peer and scope.
    pub fn new(max: usize) -> Result<Self> {
        if !(1..=32).contains(&max) {
            return Err(Error::Limit);
        }
        Ok(Self {
            inner: Arc::new(Mutex::new(PolicyState {
                max,
                generation: 1,
                next: 1,
                entries: Vec::new(),
            })),
        })
    }
    /// Trusted controller enrollment, never a peer's received assertion.
    pub fn enroll(
        &self,
        facts: &ScopeFacts,
        peer: [u8; 32],
        role: Role,
        until: Instant,
    ) -> Result<()> {
        let commitment = facts.commitment()?;
        if peer
            != match role {
                Role::Initiator => facts.acceptor_spki,
                Role::Acceptor => facts.initiator_spki,
            }
        {
            return Err(denied());
        }
        let now = Instant::now();
        if until <= now || until.saturating_duration_since(now) > Duration::from_secs(300) {
            return Err(denied());
        }
        let mut state = self.inner.lock().map_err(|_| Error::Worker)?;
        state.entries.retain(|entry| entry.until > now);
        let replace = state.entries.iter().position(|entry| {
            entry.commitment == commitment && entry.peer == peer && entry.role == role
        });
        if replace.is_none() && state.entries.len() >= state.max {
            return Err(Error::Limit);
        }
        let id = state.next;
        state.next = id.checked_add(1).ok_or(Error::Limit)?;
        let entry = Enrollment {
            commitment,
            peer,
            role,
            until,
            id,
        };
        if let Some(index) = replace {
            state.entries[index] = entry;
        } else {
            state.entries.push(entry);
        }
        Ok(())
    }
    pub fn revoke_all(&self) -> Result<()> {
        let mut state = self.inner.lock().map_err(|_| Error::Worker)?;
        state.generation = state.generation.checked_add(1).ok_or(Error::Limit)?;
        state.entries.clear();
        Ok(())
    }
    pub fn authorize_scope(
        &self,
        facts: &ScopeFacts,
        peer: [u8; 32],
        role: Role,
    ) -> Result<ScopeAuthority> {
        let commitment = facts.commitment()?;
        let context = facts.context()?;
        let state = self.inner.lock().map_err(|_| Error::Worker)?;
        let entry = state
            .entries
            .iter()
            .find(|entry| {
                entry.commitment == commitment
                    && entry.peer == peer
                    && entry.role == role
                    && entry.until > Instant::now()
            })
            .ok_or_else(denied)?;
        Ok(ScopeAuthority {
            policy: self.clone(),
            facts: facts.clone(),
            context,
            generation: state.generation,
            id: entry.id,
            role,
            peer,
        })
    }
}
impl ScopeAuthority {
    pub fn object_profile(&self) -> u16 {
        self.facts.object_profile
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn binding(&self) -> Result<Binding> {
        self.facts.binding()
    }
    pub fn check(&self) -> Result<()> {
        self.authorize(&self.context).map_err(Error::from)
    }
    pub(crate) fn check_carrier(&self, carrier: &crate::tls::TlsCarrier, role: Role) -> Result<()> {
        self.check()?;
        let local = match role {
            Role::Initiator => self.facts.initiator_spki,
            Role::Acceptor => self.facts.acceptor_spki,
        };
        if role != self.role
            || carrier.role() != role
            || carrier.exporter() != self.facts.tls_exporter
            || carrier.peer_spki_sha256() != self.peer
            || carrier.local_spki_sha256() != local
            || role == Role::Initiator && carrier.peer_addr()? != self.facts.actual_peer
        {
            return Err(denied());
        }
        Ok(())
    }
}
impl Authority for ScopeAuthority {
    fn authorize(&self, context: &Context) -> spiralcore_transfer::Result<()> {
        let deny = || spiralcore_transfer::Error::Unauthorized;
        if context != &self.context {
            return Err(deny());
        }
        let state = self.policy.inner.lock().map_err(|_| deny())?;
        if state.generation != self.generation
            || !state.entries.iter().any(|entry| {
                entry.id == self.id
                    && entry.role == self.role
                    && entry.peer == self.peer
                    && entry.until > Instant::now()
            })
        {
            return Err(deny());
        }
        Ok(())
    }
}
