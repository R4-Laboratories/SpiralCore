//! Owner-scoped reservations for serialized ciphertext transport capacity.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportLimits {
    pub links: u64,
    pub frame_slots: u64,
    pub serialized_buffer_bytes: u64,
}
impl TransportLimits {
    fn add(self, rhs: Self) -> Option<Self> {
        Some(Self {
            links: self.links.checked_add(rhs.links)?,
            frame_slots: self.frame_slots.checked_add(rhs.frame_slots)?,
            serialized_buffer_bytes: self
                .serialized_buffer_bytes
                .checked_add(rhs.serialized_buffer_bytes)?,
        })
    }
    fn sub(self, rhs: Self) -> Option<Self> {
        Some(Self {
            links: self.links.checked_sub(rhs.links)?,
            frame_slots: self.frame_slots.checked_sub(rhs.frame_slots)?,
            serialized_buffer_bytes: self
                .serialized_buffer_bytes
                .checked_sub(rhs.serialized_buffer_bytes)?,
        })
    }
    fn within(self, limit: Self) -> bool {
        self.links <= limit.links
            && self.frame_slots <= limit.frame_slots
            && self.serialized_buffer_bytes <= limit.serialized_buffer_bytes
    }
    fn peak(self, rhs: Self) -> Self {
        Self {
            links: self.links.max(rhs.links),
            frame_slots: self.frame_slots.max(rhs.frame_slots),
            serialized_buffer_bytes: self
                .serialized_buffer_bytes
                .max(rhs.serialized_buffer_bytes),
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportBudgetConfig {
    pub host: TransportLimits,
    pub tenant: TransportLimits,
    pub max_tenants: u32,
    pub max_frame_bytes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportBudgetError {
    InvalidConfig,
    UnknownScope,
    Full,
    Overflow,
    Fenced,
    State,
}
impl fmt::Display for TransportBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "transport budget: {self:?}")
    }
}
impl std::error::Error for TransportBudgetError {}
type Result<T> = std::result::Result<T, TransportBudgetError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, try_from = "Report")]
pub struct TransportBudgetSnapshot {
    pub version: u8,
    pub scope: String,
    pub resource_domain: String,
    pub limits: TransportLimits,
    pub usage: TransportLimits,
    pub high_water: TransportLimits,
    pub registered_tenants: u32,
    pub refusals: u64,
    pub fenced: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    version: u8,
    scope: String,
    resource_domain: String,
    limits: TransportLimits,
    usage: TransportLimits,
    high_water: TransportLimits,
    registered_tenants: u32,
    refusals: u64,
    fenced: bool,
}
impl TryFrom<Report> for TransportBudgetSnapshot {
    type Error = &'static str;
    fn try_from(r: Report) -> std::result::Result<Self, Self::Error> {
        if r.version != 1
            || r.scope != "orchestrator"
            || r.resource_domain != "serialized-ciphertext-transport"
            || !r.usage.within(r.high_water)
            || !r.high_water.within(r.limits)
        {
            return Err("invalid transport budget report");
        }
        Ok(Self {
            version: r.version,
            scope: r.scope,
            resource_domain: r.resource_domain,
            limits: r.limits,
            usage: r.usage,
            high_water: r.high_water,
            registered_tenants: r.registered_tenants,
            refusals: r.refusals,
            fenced: r.fenced,
        })
    }
}
#[derive(Default)]
struct Ledger {
    usage: TransportLimits,
    high_water: TransportLimits,
    tenants: BTreeMap<u32, TransportLimits>,
    refusals: u64,
}
struct Shared {
    config: TransportBudgetConfig,
    ledger: Mutex<Ledger>,
    fenced: AtomicBool,
}
impl Shared {
    fn lock(&self) -> Result<MutexGuard<'_, Ledger>> {
        self.ledger.lock().map_err(|_| {
            self.fenced.store(true, Ordering::Release);
            TransportBudgetError::State
        })
    }
    fn refuse<T>(ledger: &mut Ledger, error: TransportBudgetError) -> Result<T> {
        ledger.refusals = ledger.refusals.saturating_add(1);
        Err(error)
    }
    fn reject<T>(&self, error: TransportBudgetError) -> Result<T> {
        let mut ledger = self.lock()?;
        let error = if self.fenced.load(Ordering::Acquire) {
            TransportBudgetError::Fenced
        } else {
            error
        };
        Self::refuse(&mut ledger, error)
    }
    fn charge(&self, slot: u32, delta: TransportLimits) -> Result<()> {
        let mut ledger = self.lock()?;
        if self.fenced.load(Ordering::Acquire) {
            return Self::refuse(&mut ledger, TransportBudgetError::Fenced);
        }
        let Some(tenant) = ledger.tenants.get(&slot).copied() else {
            return Self::refuse(&mut ledger, TransportBudgetError::UnknownScope);
        };
        let (Some(host), Some(tenant)) = (ledger.usage.add(delta), tenant.add(delta)) else {
            return Self::refuse(&mut ledger, TransportBudgetError::Overflow);
        };
        if !host.within(self.config.host) || !tenant.within(self.config.tenant) {
            return Self::refuse(&mut ledger, TransportBudgetError::Full);
        }
        ledger.usage = host;
        ledger.high_water = ledger.high_water.peak(host);
        ledger.tenants.insert(slot, tenant);
        Ok(())
    }
    fn release(&self, slot: u32, delta: TransportLimits) {
        let Ok(mut ledger) = self.lock() else {
            return;
        };
        let host = ledger.usage.sub(delta);
        let tenant = ledger.tenants.get(&slot).and_then(|usage| usage.sub(delta));
        if let (Some(host), Some(tenant)) = (host, tenant) {
            ledger.usage = host;
            ledger.tenants.insert(slot, tenant);
        } else {
            self.fenced.store(true, Ordering::Release);
        }
    }
}
#[derive(Clone)]
pub struct HostTransportBudget {
    shared: Arc<Shared>,
}
pub struct TransportScope {
    owner: Arc<Shared>,
    slot: u32,
}
pub struct LinkReservation {
    link: Arc<LinkCharge>,
}
struct LinkCharge {
    owner: Arc<Shared>,
    slot: u32,
}
pub struct FrameReservation {
    link: Arc<LinkCharge>,
    capacity: u64,
}
impl HostTransportBudget {
    pub fn new(config: TransportBudgetConfig) -> Result<Self> {
        if config.max_tenants == 0
            || config.max_frame_bytes == 0
            || [config.host, config.tenant].iter().any(|l| {
                l.links == 0
                    || l.frame_slots == 0
                    || config.max_frame_bytes > l.serialized_buffer_bytes
            })
        {
            return Err(TransportBudgetError::InvalidConfig);
        }
        Ok(Self {
            shared: Arc::new(Shared {
                config,
                ledger: Mutex::new(Ledger::default()),
                fenced: AtomicBool::new(false),
            }),
        })
    }
    pub fn scope(&self, slot: u32) -> Result<TransportScope> {
        let mut ledger = self.shared.lock()?;
        if self.shared.fenced.load(Ordering::Acquire) {
            return Shared::refuse(&mut ledger, TransportBudgetError::Fenced);
        }
        if slot >= self.shared.config.max_tenants {
            return Shared::refuse(&mut ledger, TransportBudgetError::UnknownScope);
        }
        ledger.tenants.entry(slot).or_default();
        Ok(TransportScope {
            owner: self.shared.clone(),
            slot,
        })
    }
    pub fn reserve_link(&self, scope: &TransportScope) -> Result<LinkReservation> {
        if !Arc::ptr_eq(&self.shared, &scope.owner) {
            return self.shared.reject(TransportBudgetError::UnknownScope);
        }
        self.shared.charge(
            scope.slot,
            TransportLimits {
                links: 1,
                ..TransportLimits::default()
            },
        )?;
        Ok(LinkReservation {
            link: Arc::new(LinkCharge {
                owner: self.shared.clone(),
                slot: scope.slot,
            }),
        })
    }
    pub fn snapshot(&self) -> Result<TransportBudgetSnapshot> {
        let ledger = self.shared.lock()?;
        Ok(TransportBudgetSnapshot {
            version: 1,
            scope: "orchestrator".into(),
            resource_domain: "serialized-ciphertext-transport".into(),
            limits: self.shared.config.host,
            usage: ledger.usage,
            high_water: ledger.high_water,
            registered_tenants: u32::try_from(ledger.tenants.len())
                .map_err(|_| TransportBudgetError::Overflow)?,
            refusals: ledger.refusals,
            fenced: self.shared.fenced.load(Ordering::Acquire),
        })
    }
}
impl LinkReservation {
    pub fn reserve_frame(&self, capacity: u64) -> Result<FrameReservation> {
        if capacity == 0 || capacity > self.link.owner.config.max_frame_bytes {
            return self.link.owner.reject(TransportBudgetError::Full);
        }
        self.link.owner.charge(
            self.link.slot,
            TransportLimits {
                frame_slots: 1,
                serialized_buffer_bytes: capacity,
                ..TransportLimits::default()
            },
        )?;
        Ok(FrameReservation {
            link: self.link.clone(),
            capacity,
        })
    }
}
impl FrameReservation {
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}
impl Drop for FrameReservation {
    fn drop(&mut self) {
        self.link.owner.release(
            self.link.slot,
            TransportLimits {
                frame_slots: 1,
                serialized_buffer_bytes: self.capacity,
                ..TransportLimits::default()
            },
        );
    }
}
impl Drop for LinkCharge {
    fn drop(&mut self) {
        self.owner.release(
            self.slot,
            TransportLimits {
                links: 1,
                ..TransportLimits::default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owner() -> HostTransportBudget {
        let limits = TransportLimits {
            links: 1,
            frame_slots: 1,
            serialized_buffer_bytes: 8,
        };
        HostTransportBudget::new(TransportBudgetConfig {
            host: limits,
            tenant: limits,
            max_tenants: 1,
            max_frame_bytes: 8,
        })
        .unwrap()
    }
    #[test]
    fn underflow_fences_without_clamping_and_refusals_saturate() {
        let owner = owner();
        let scope = owner.scope(0).unwrap();
        let link = owner.reserve_link(&scope).unwrap();
        owner.shared.ledger.lock().unwrap().usage.links = 0;
        drop(link);
        assert!(owner.snapshot().unwrap().fenced);
        assert!(matches!(
            owner.reserve_link(&scope),
            Err(TransportBudgetError::Fenced)
        ));
        owner.shared.ledger.lock().unwrap().refusals = u64::MAX;
        assert!(owner.reserve_link(&scope).is_err());
        assert_eq!(owner.snapshot().unwrap().refusals, u64::MAX);
    }
    #[test]
    fn poisoned_ledger_refuses_snapshot_and_admission() {
        let owner = owner();
        let scope = owner.scope(0).unwrap();
        let copy = owner.clone();
        assert!(
            std::thread::spawn(move || {
                let _guard = copy.shared.ledger.lock().unwrap();
                panic!("poison");
            })
            .join()
            .is_err()
        );
        assert!(matches!(owner.snapshot(), Err(TransportBudgetError::State)));
        assert!(matches!(
            owner.reserve_link(&scope),
            Err(TransportBudgetError::State)
        ));
        assert!(owner.shared.fenced.load(Ordering::Acquire));
    }
}
