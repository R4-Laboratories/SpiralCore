//! One installed parent ledger for managed resources, separate from permission.
use crate::installed_supervisor::InstalledAuthority;
use serde::Serialize;
use std::{
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
const SLOTS: usize = 128;
const TENANTS: usize = 32;
const MIB: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ManagedClaim {
    pub host_plaintext_bytes: u64,
    pub host_codec_bytes: u64,
    pub host_carrier_bytes: u64,
    pub host_metadata_bytes: u64,
    pub host_stack_bytes: u64,
    pub guest_memory_bytes: u64,
    pub cache_disk_bytes: u64,
    pub staging_disk_bytes: u64,
    pub workers: u64,
    pub guests: u64,
}
macro_rules! arithmetic {
    ($method:ident, $op:ident) => {
        fn $method(self, rhs: Self) -> Option<Self> {
            Some(Self {
                host_plaintext_bytes: self.host_plaintext_bytes.$op(rhs.host_plaintext_bytes)?,
                host_codec_bytes: self.host_codec_bytes.$op(rhs.host_codec_bytes)?,
                host_carrier_bytes: self.host_carrier_bytes.$op(rhs.host_carrier_bytes)?,
                host_metadata_bytes: self.host_metadata_bytes.$op(rhs.host_metadata_bytes)?,
                host_stack_bytes: self.host_stack_bytes.$op(rhs.host_stack_bytes)?,
                guest_memory_bytes: self.guest_memory_bytes.$op(rhs.guest_memory_bytes)?,
                cache_disk_bytes: self.cache_disk_bytes.$op(rhs.cache_disk_bytes)?,
                staging_disk_bytes: self.staging_disk_bytes.$op(rhs.staging_disk_bytes)?,
                workers: self.workers.$op(rhs.workers)?,
                guests: self.guests.$op(rhs.guests)?,
            })
        }
    };
}
impl ManagedClaim {
    arithmetic!(add, checked_add);
    arithmetic!(sub, checked_sub);
    pub fn host_resident_bytes(self) -> Result<u64> {
        self.host_plaintext_bytes
            .checked_add(self.host_codec_bytes)
            .and_then(|n| n.checked_add(self.host_carrier_bytes))
            .and_then(|n| n.checked_add(self.host_metadata_bytes))
            .and_then(|n| n.checked_add(self.host_stack_bytes))
            .ok_or(ManagedResourceError::Overflow)
    }
    fn valid(self) -> Result<()> {
        self.host_resident_bytes()?;
        if self == Self::default()
            || self.guests > 1
            || self.guest_memory_bytes > 1024 * MIB
            || (self.guests == 0) != (self.guest_memory_bytes == 0)
        {
            return Err(ManagedResourceError::InvalidClaim);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct ManagedLimits {
    pub host_resident_bytes: u64,
    pub guest_memory_bytes: u64,
    pub cache_disk_bytes: u64,
    pub staging_disk_bytes: u64,
    pub workers: u64,
    pub guests: u64,
}
impl ManagedLimits {
    fn admits(self, usage: ManagedClaim) -> bool {
        usage
            .host_resident_bytes()
            .is_ok_and(|n| n <= self.host_resident_bytes)
            && usage.guest_memory_bytes <= self.guest_memory_bytes
            && usage.cache_disk_bytes <= self.cache_disk_bytes
            && usage.staging_disk_bytes <= self.staging_disk_bytes
            && usage.workers <= self.workers
            && usage.guests <= self.guests
    }
    fn host() -> Self {
        Self {
            host_resident_bytes: 512 * MIB,
            guest_memory_bytes: 2048 * MIB,
            cache_disk_bytes: 1024 * MIB,
            staging_disk_bytes: 512 * MIB,
            workers: 16,
            guests: 2,
        }
    }
    fn tenant() -> Self {
        Self {
            host_resident_bytes: 256 * MIB,
            cache_disk_bytes: 512 * MIB,
            staging_disk_bytes: 256 * MIB,
            ..Self::host()
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedResourceError {
    Full,
    Fenced,
    InvalidClaim,
    Overflow,
    State,
}
impl fmt::Display for ManagedResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "managed resources: {self:?}")
    }
}
impl std::error::Error for ManagedResourceError {}
type Result<T> = std::result::Result<T, ManagedResourceError>;
#[derive(Clone, Debug, Serialize)]
pub struct ManagedSnapshot {
    pub version: u8,
    pub scope: &'static str,
    pub usage: ManagedClaim,
    pub peak_host_resident_bytes: u64,
    pub active_reservations: usize,
    pub registered_tenants: usize,
    pub fenced: bool,
}
#[derive(Clone, Copy)]
struct Tenant {
    id: [u8; 32],
    usage: ManagedClaim,
}
#[derive(Clone, Copy)]
struct Slot {
    serial: u64,
    tenant: usize,
    claim: ManagedClaim,
}
struct Ledger {
    usage: ManagedClaim,
    peak_resident: u64,
    next: u64,
    tenants: [Option<Tenant>; TENANTS],
    slots: [Option<Slot>; SLOTS],
}
impl Default for Ledger {
    fn default() -> Self {
        Self {
            usage: ManagedClaim::default(),
            peak_resident: 0,
            next: 1,
            tenants: std::array::from_fn(|_| None),
            slots: std::array::from_fn(|_| None),
        }
    }
}
struct Shared {
    authority: Arc<InstalledAuthority>,
    ledger: Mutex<Ledger>,
    fenced: AtomicBool,
    quarantined: AtomicBool,
}
impl Shared {
    fn fence(&self) {
        self.fenced.store(true, Ordering::Release);
        self.authority.close_admission();
        // An accounting fault cannot release the installation's exclusion.
        // Exactly one bounded retained owner per pool, until process exit.
        if !self.quarantined.swap(true, Ordering::AcqRel) {
            std::mem::forget(self.authority.clone());
        }
    }
    fn ledger(&self) -> Result<std::sync::MutexGuard<'_, Ledger>> {
        self.ledger.lock().map_err(|_| {
            self.fence();
            ManagedResourceError::State
        })
    }
    fn open(&self) -> bool {
        !self.fenced.load(Ordering::Acquire) && self.authority.admission_open()
    }
}
#[derive(Clone)]
pub struct ManagedResourceBudget {
    shared: Arc<Shared>,
}
impl ManagedResourceBudget {
    pub(crate) fn new_installed(authority: Arc<InstalledAuthority>) -> Self {
        Self {
            shared: Arc::new(Shared {
                authority,
                ledger: Mutex::new(Ledger::default()),
                fenced: AtomicBool::new(false),
                quarantined: AtomicBool::new(false),
            }),
        }
    }
    pub fn limits(&self) -> ManagedLimits {
        ManagedLimits::host()
    }
    pub fn reserve(&self, tenant: [u8; 32], claim: ManagedClaim) -> Result<ManagedReservation> {
        claim.valid()?;
        if tenant == [0; 32] {
            return Err(ManagedResourceError::InvalidClaim);
        }
        if !self.shared.open() {
            return Err(ManagedResourceError::Fenced);
        }
        // File/current-epoch validation precedes the accounting lock. Nothing
        // returned here changes canonical MVM or network authorization.
        if !self.shared.authority.resource_epoch_current() {
            self.shared.authority.close_admission();
            return Err(ManagedResourceError::Fenced);
        }
        let mut ledger = self.shared.ledger()?;
        if !self.shared.open() {
            return Err(ManagedResourceError::Fenced);
        }
        let free = ledger
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(ManagedResourceError::Full)?;
        let tenant_slot = ledger
            .tenants
            .iter()
            .position(|entry| entry.is_some_and(|v| v.id == tenant))
            .or_else(|| ledger.tenants.iter().position(Option::is_none))
            .ok_or(ManagedResourceError::Full)?;
        let previous = ledger.tenants[tenant_slot]
            .map(|v| v.usage)
            .unwrap_or_default();
        let host = ledger
            .usage
            .add(claim)
            .ok_or(ManagedResourceError::Overflow)?;
        let scoped = previous.add(claim).ok_or(ManagedResourceError::Overflow)?;
        if !ManagedLimits::host().admits(host) || !ManagedLimits::tenant().admits(scoped) {
            return Err(ManagedResourceError::Full);
        }
        let serial = ledger.next;
        let next = serial
            .checked_add(1)
            .ok_or(ManagedResourceError::Overflow)?;
        let peak = ledger.peak_resident.max(host.host_resident_bytes()?);
        // All checks finish before any ledger mutation or exposed lease.
        ledger.slots[free] = Some(Slot {
            serial,
            tenant: tenant_slot,
            claim,
        });
        ledger.tenants[tenant_slot] = Some(Tenant {
            id: tenant,
            usage: scoped,
        });
        ledger.usage = host;
        ledger.next = next;
        ledger.peak_resident = peak;
        drop(ledger);
        Ok(ManagedReservation {
            owner: Arc::new(Reservation {
                shared: self.shared.clone(),
                slot: free,
                serial,
            }),
        })
    }
    pub fn snapshot(&self) -> Result<ManagedSnapshot> {
        let ledger = self.shared.ledger()?;
        if self.shared.fenced.load(Ordering::Acquire) {
            return Err(ManagedResourceError::State);
        }
        Ok(ManagedSnapshot {
            version: 1,
            scope: "installation-managed-resources",
            usage: ledger.usage,
            peak_host_resident_bytes: ledger.peak_resident,
            active_reservations: ledger.slots.iter().flatten().count(),
            registered_tenants: ledger.tenants.iter().flatten().count(),
            fenced: !self.shared.open(),
        })
    }
    pub(crate) fn ensure_drained(&self) -> Result<bool> {
        let snapshot = self.snapshot()?;
        Ok(snapshot.active_reservations == 0 && snapshot.usage == ManagedClaim::default())
    }
}
struct Reservation {
    shared: Arc<Shared>,
    slot: usize,
    serial: u64,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let Ok(mut ledger) = self.shared.ledger() else {
            return;
        };
        let Some(slot) = ledger
            .slots
            .get(self.slot)
            .copied()
            .flatten()
            .filter(|s| s.serial == self.serial)
        else {
            self.shared.fence();
            return;
        };
        let Some(tenant) = ledger.tenants.get(slot.tenant).copied().flatten() else {
            self.shared.fence();
            return;
        };
        let (Some(host), Some(scoped)) =
            (ledger.usage.sub(slot.claim), tenant.usage.sub(slot.claim))
        else {
            self.shared.fence();
            return;
        };
        ledger.usage = host;
        ledger.tenants[slot.tenant] = Some(Tenant {
            usage: scoped,
            ..tenant
        });
        ledger.slots[self.slot] = None;
    }
}
#[derive(Clone)]
pub struct ManagedReservation {
    owner: Arc<Reservation>,
}
impl ManagedReservation {
    pub fn check_current(&self) -> Result<()> {
        if !self.owner.shared.open() {
            return Err(ManagedResourceError::Fenced);
        }
        if !self.owner.shared.authority.resource_epoch_current() {
            self.owner.shared.authority.close_admission();
            return Err(ManagedResourceError::Fenced);
        }
        let ledger = self.owner.shared.ledger()?;
        let current_slot = ledger
            .slots
            .get(self.owner.slot)
            .copied()
            .flatten()
            .filter(|s| s.serial == self.owner.serial);
        if current_slot.is_none_or(|s| ledger.tenants.get(s.tenant).is_none_or(Option::is_none)) {
            self.owner.shared.fence();
            return Err(ManagedResourceError::State);
        }
        Ok(())
    }
    pub fn attach<T>(self, value: T) -> ManagedValue<T> {
        ManagedValue {
            value,
            _reservation: self,
        }
    }
}
/// Field order drops the actual value before releasing its reservation.
pub struct ManagedValue<T> {
    value: T,
    _reservation: ManagedReservation,
}
impl<T> AsRef<T> for ManagedValue<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        installed_owner::{InstallationKey, InstallationOwner},
        installed_supervisor::{InstalledConfig, InstalledSupervisor},
    };
    use std::{fs, os::unix::fs::DirBuilderExt, path::PathBuf};
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "spiral-managed-fault-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            Self(path)
        }
        fn owner(&self) -> (InstalledSupervisor, InstallationKey) {
            let config = InstalledConfig::native_profile();
            let key = InstallationKey {
                installation: [92; 32],
                profile: config.profile_hash(),
            };
            let owner = InstallationOwner::initialize(&self.0.join("installation"), key).unwrap();
            (InstalledSupervisor::start(owner, config).unwrap(), key)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn accounting_corruption_never_panics_refunds_or_releases_exclusion() {
        for fault in 0..5 {
            let f = Fixture::new();
            let (mut owner, key) = f.owner();
            let pool = owner.managed_resources().clone();
            let lease = pool
                .reserve(
                    [1; 32],
                    ManagedClaim {
                        host_plaintext_bytes: 1,
                        ..Default::default()
                    },
                )
                .unwrap();
            let slot_index = lease.owner.slot;
            if fault == 4 {
                let shared = pool.shared.clone();
                assert!(
                    std::thread::spawn(move || {
                        let _ledger = shared.ledger.lock().unwrap();
                        panic!("owned accounting poison fixture");
                    })
                    .join()
                    .is_err()
                );
            } else {
                let mut ledger = pool.shared.ledger.lock().unwrap();
                match fault {
                    0 => ledger.slots[slot_index].as_mut().unwrap().serial += 1,
                    1 => ledger.usage.host_plaintext_bytes = 0,
                    2 => ledger.slots[slot_index].as_mut().unwrap().tenant = TENANTS,
                    3 => ledger.tenants[0] = None,
                    _ => unreachable!(),
                }
            }
            if matches!(fault, 0 | 2 | 3 | 4) {
                assert!(matches!(
                    lease.check_current(),
                    Err(ManagedResourceError::State)
                ));
            }
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(lease))).is_ok());
            assert!(pool.snapshot().is_err());
            assert!(
                pool.reserve(
                    [1; 32],
                    ManagedClaim {
                        host_plaintext_bytes: 1,
                        ..Default::default()
                    }
                )
                .is_err()
            );
            owner.close_admission();
            assert!(owner.try_finish().is_err());
            drop(owner);
            drop(pool);
            assert!(InstallationOwner::open(&f.0.join("installation"), key).is_err());
        }
    }
}
