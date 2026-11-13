//! One installation owner and one transitively retained transport ledger.
//! Worker activation is a separate boundary; this module grants no boot API.
use crate::{
    host_budget::{
        HostTransportBudget, TransportBudgetConfig, TransportBudgetError, TransportLimits,
    },
    installed_owner::{Epoch, HolderHandoff, InstallationBinding, InstallationOwner, OwnerError},
    installed_resources::{ManagedResourceBudget, ManagedResourceError},
};
use sha2::{Digest, Sha256};
use std::{
    fmt, io,
    path::PathBuf,
    process::{Child, Command, ExitStatus},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub struct InstalledConfig {
    canonical: bool,
}
impl InstalledConfig {
    pub fn native_profile() -> Self {
        Self { canonical: false }
    }
    /// Fresh provisioning for the canonical resource/disk contract. The stamp
    /// identifies included model definitions, not an executed child or RSS.
    pub fn canonical_native_profile() -> Self {
        Self { canonical: true }
    }
    pub fn profile_hash(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(b"spiralcore/installed-profile/v1\0");
        for value in [2_u64, 2, 524_288, 1, 262_144, 2, 1, 1024] {
            digest.update(value.to_be_bytes());
        }
        if self.canonical {
            digest.update(b"spiralcore/canonical-installed-model-definition/v2\0");
            for tag in [
                b"endpoint-bootstrap-v1".as_slice(),
                b"owned-helper-model-v1",
                b"disposable-inventory-v1",
                b"retained-telemetry-catalog-v1",
                b"native-fc-hvf-role-v1",
            ] {
                digest.update((tag.len() as u64).to_be_bytes());
                digest.update(tag);
            }
            digest.update(usize::BITS.to_be_bytes());
            digest.update(std::env::consts::OS.as_bytes());
            digest.update(std::env::consts::ARCH.as_bytes());
            // These are compiled-in definition stamps. Final activation also
            // requires the actual one-Core/MVM build closure and child checks.
            for definition in [
                include_bytes!("../Cargo.toml").as_slice(),
                include_bytes!("installed_resources.rs").as_slice(),
                include_bytes!("installed_owner/endpoint_disks.rs").as_slice(),
                include_bytes!("installed_owner/retained_telemetry.rs").as_slice(),
                include_bytes!("installed_retained.rs").as_slice(),
                include_bytes!("../../spiralcore-transfer/Cargo.toml").as_slice(),
                include_bytes!("../../spiralcore-transfer/Cargo.lock").as_slice(),
                include_bytes!("../../spiralcore-transfer/src/resource_profile.rs").as_slice(),
                include_bytes!("../../spiralcore-transfer/src/schema.rs").as_slice(),
                include_bytes!("../../spiralcore-transfer/src/schema_zstd.rs").as_slice(),
                include_bytes!("../../spiralcore-transfer/src/session.rs").as_slice(),
                include_bytes!("../../spiralcore-transfer/src/endpoint.rs").as_slice(),
                include_bytes!("../../spiralcore-transfer/src/v1/ordered.rs").as_slice(),
            ] {
                digest.update((definition.len() as u64).to_be_bytes());
                digest.update(definition);
            }
        }
        digest.finalize().into()
    }
    fn transport(&self) -> TransportBudgetConfig {
        let limits = TransportLimits {
            links: 2,
            frame_slots: 2,
            serialized_buffer_bytes: 524_288,
        };
        TransportBudgetConfig {
            host: limits,
            tenant: limits,
            max_tenants: 1,
            max_frame_bytes: 262_144,
        }
    }
}

#[derive(Debug)]
pub enum SupervisorError {
    Busy,
    State,
    Profile,
    Owner(OwnerError),
    Budget(TransportBudgetError),
    Resources(ManagedResourceError),
    Io(io::Error),
}
impl fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "installed supervisor: {self:?}")
    }
}
impl std::error::Error for SupervisorError {}
impl From<io::Error> for SupervisorError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<OwnerError> for SupervisorError {
    fn from(error: OwnerError) -> Self {
        Self::Owner(error)
    }
}
impl From<TransportBudgetError> for SupervisorError {
    fn from(error: TransportBudgetError) -> Self {
        Self::Budget(error)
    }
}
impl From<ManagedResourceError> for SupervisorError {
    fn from(error: ManagedResourceError) -> Self {
        Self::Resources(error)
    }
}
type Result<T> = std::result::Result<T, SupervisorError>;

pub(crate) struct InstalledAuthority {
    owner: Mutex<InstallationOwner>,
    epoch: Epoch,
    open: AtomicBool,
    workers: Mutex<WorkerLedger>,
    canonical: bool,
}
impl InstalledAuthority {
    pub(crate) fn with_disk_owner<T>(
        &self,
        operation: impl FnOnce(&mut InstallationOwner, Epoch) -> std::result::Result<T, OwnerError>,
    ) -> Result<T> {
        let mut owner = self.owner.lock().map_err(|_| {
            self.close_admission();
            SupervisorError::State
        })?;
        operation(&mut owner, self.epoch).map_err(Into::into)
    }
    pub(crate) fn admission_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }
    pub(crate) fn close_admission(&self) {
        self.open.store(false, Ordering::Release);
    }
    pub(crate) fn resource_epoch_current(&self) -> bool {
        self.owner
            .lock()
            .is_ok_and(|owner| owner.active_binding(self.epoch).is_ok())
    }
    fn workers(&self) -> Result<std::sync::MutexGuard<'_, WorkerLedger>> {
        self.workers.lock().map_err(|_| {
            self.close_admission();
            SupervisorError::State
        })
    }
}

/// Original descriptor custody for the installed parent's native bridge.
/// This grants no VM or service permission and retains no idle SH descriptor.
pub struct InstalledNativeIssuer {
    authority: Arc<InstalledAuthority>,
}
impl InstalledNativeIssuer {
    pub fn prepare(&self) -> Result<(InstallationBinding, HolderHandoff)> {
        if !self.authority.admission_open() {
            return Err(SupervisorError::State);
        }
        let result = (|| {
            let owner = self
                .authority
                .owner
                .lock()
                .map_err(|_| SupervisorError::State)?;
            let binding = owner.active_binding(self.authority.epoch)?;
            let handoff = owner.prepare_original_handoff(self.authority.epoch)?;
            if !self.authority.admission_open() {
                return Err(SupervisorError::State);
            }
            Ok((binding, handoff))
        })();
        if result.is_err() {
            self.authority.close_admission();
        }
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkerRole {
    Sender,
    Receiver,
    Verify,
}

/// Local accounting token, never a wire credential or proof of backend exit.
/// Drop cannot refund its slot. A stale token still retains owner exclusion.
pub(crate) struct WorkerGrant {
    authority: Arc<InstalledAuthority>,
    slot: usize,
    serial: u64,
    role: WorkerRole,
}
impl WorkerGrant {
    pub(crate) fn close_admission(&self) {
        self.authority.close_admission();
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WorkerBinding {
    pub(crate) root: PathBuf,
    pub(crate) installation: [u8; 32],
    pub(crate) profile: [u8; 32],
    pub(crate) generation: u64,
    pub(crate) role: u8,
    pub(crate) slot: u8,
    pub(crate) serial: u64,
}

struct WorkerSlot {
    serial: u64,
    role: WorkerRole,
    child: Option<Child>,
    reaped: Option<ExitStatus>,
    launching: bool,
}
#[derive(Default)]
struct WorkerLedger {
    serial: u64,
    slots: [Option<WorkerSlot>; 2],
}
#[derive(PartialEq, Eq)]
enum Phase {
    Open,
    Closing,
    Finished,
    Fenced,
}

pub struct InstalledSupervisor {
    authority: Arc<InstalledAuthority>,
    budget: HostTransportBudget,
    resources: ManagedResourceBudget,
    #[cfg(feature = "canonical-host")]
    retained: Mutex<Option<Arc<crate::installed_retained::RetainedTelemetryInventory>>>,
    phase: Phase,
    session_started: bool,
}
impl InstalledSupervisor {
    pub(crate) fn require_canonical_profile(&self) -> Result<()> {
        if self.authority.canonical {
            Ok(())
        } else {
            Err(SupervisorError::Profile)
        }
    }
    pub(crate) fn disk_authority(&self) -> Arc<InstalledAuthority> {
        self.authority.clone()
    }
    pub fn endpoint_disk_inventory(&self) -> Result<crate::installed_disk::EndpointDiskInventory> {
        crate::installed_disk::EndpointDiskInventory::new(self)
    }
    #[cfg(feature = "canonical-host")]
    pub(crate) fn retained_telemetry_inventory(
        &self,
    ) -> Result<Arc<crate::installed_retained::RetainedTelemetryInventory>> {
        let mut original = self.retained.lock().map_err(|_| SupervisorError::State)?;
        if original.is_none() {
            *original = Some(crate::installed_retained::RetainedTelemetryInventory::new(self)?);
        }
        original.as_ref().cloned().ok_or(SupervisorError::State)
    }
    pub fn start(mut owner: InstallationOwner, config: InstalledConfig) -> Result<Self> {
        if owner.profile_hash()? != config.profile_hash() {
            return Err(SupervisorError::Profile);
        }
        let epoch = owner.begin()?;
        let authority = Arc::new(InstalledAuthority {
            owner: Mutex::new(owner),
            epoch,
            open: AtomicBool::new(true),
            workers: Mutex::new(WorkerLedger::default()),
            canonical: config.canonical,
        });
        let budget = HostTransportBudget::new_installed(config.transport(), authority.clone())?;
        let resources = ManagedResourceBudget::new_installed(authority.clone());
        Ok(Self {
            authority,
            budget,
            resources,
            #[cfg(feature = "canonical-host")]
            retained: Mutex::new(None),
            phase: Phase::Open,
            session_started: false,
        })
    }
    pub fn budget(&self) -> &HostTransportBudget {
        &self.budget
    }
    pub fn managed_resources(&self) -> &ManagedResourceBudget {
        &self.resources
    }
    pub fn native_issuer(&self) -> InstalledNativeIssuer {
        InstalledNativeIssuer {
            authority: self.authority.clone(),
        }
    }
    pub(crate) fn claim_session(&mut self) -> Result<()> {
        if self.phase != Phase::Open || !self.authority.admission_open() || self.session_started {
            return Err(SupervisorError::State);
        }
        if self.authority.workers()?.slots.iter().any(Option::is_some) {
            return Err(SupervisorError::Busy);
        }
        self.session_started = true;
        Ok(())
    }

    pub(crate) fn worker_binding(&mut self, grant: &WorkerGrant) -> Result<WorkerBinding> {
        // Token identity is checked before any filesystem query. A positively
        // reaped worker cannot receive a new activation answer.
        if self.try_reap_worker(grant)?.is_some() {
            self.authority.close_admission();
            return Err(SupervisorError::State);
        }
        if self.phase != Phase::Open || !self.authority.admission_open() {
            return Err(SupervisorError::State);
        }
        let installation = match self.authority.owner.lock() {
            Ok(owner) => owner.active_binding(self.authority.epoch),
            Err(_) => {
                self.authority.close_admission();
                return Err(SupervisorError::State);
            }
        };
        let installation = installation.map_err(|error| {
            self.authority.close_admission();
            SupervisorError::Owner(error)
        })?;
        // The storage read held no worker/transport ledger. Check again after
        // it, immediately before returning facts for the private channel write.
        if !self.authority.admission_open() || self.try_reap_worker(grant)?.is_some() {
            self.authority.close_admission();
            return Err(SupervisorError::State);
        }
        Ok(WorkerBinding {
            root: installation.root,
            installation: installation.key.installation,
            profile: installation.key.profile,
            generation: installation.generation,
            role: match grant.role {
                WorkerRole::Sender => 1,
                WorkerRole::Receiver => 2,
                WorkerRole::Verify => 3,
            },
            slot: u8::try_from(grant.slot).map_err(|_| SupervisorError::State)?,
            serial: grant.serial,
        })
    }
    pub(crate) fn reserve_worker(&mut self, role: WorkerRole) -> Result<WorkerGrant> {
        let mut workers = self.authority.workers()?;
        if self.phase != Phase::Open || !self.authority.admission_open() {
            return Err(SupervisorError::State);
        }
        let slot = workers
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(SupervisorError::Busy)?;
        let Some(serial) = workers.serial.checked_add(1) else {
            self.authority.close_admission();
            return Err(SupervisorError::State);
        };
        workers.serial = serial;
        workers.slots[slot] = Some(WorkerSlot {
            serial,
            role,
            child: None,
            reaped: None,
            launching: false,
        });
        Ok(WorkerGrant {
            authority: self.authority.clone(),
            slot,
            serial,
            role,
        })
    }
    /// The actual launcher must use this transaction, not reserve/spawn/bind
    /// itself. Only a definite spawn error can prove this reservation empty.
    pub(crate) fn spawn_worker(
        &mut self,
        role: WorkerRole,
        command: &mut Command,
    ) -> Result<WorkerGrant> {
        let grant = self.reserve_worker(role)?;
        {
            let mut workers = self.authority.workers()?;
            self.worker_slot(&mut workers, &grant)?.launching = true;
        }
        // No process creation or pre-exec work under the worker ledger.
        match command.spawn() {
            Ok(child) => {
                let mut pending = Some(child);
                let error = match self.bind_worker(&grant, &mut pending) {
                    Ok(()) => return Ok(grant),
                    Err(error) => error,
                };
                // An accounting fault may have closed admission during spawn.
                // Take custody even then: never drop a newly created Child
                // without retaining it for exact cleanup by this controller.
                self.authority.close_admission();
                let mut workers = match self.authority.workers.lock() {
                    Ok(workers) => workers,
                    Err(poisoned) => {
                        self.authority.close_admission();
                        poisoned.into_inner()
                    }
                };
                let slot = workers.slots[grant.slot].as_mut().unwrap();
                slot.child = pending.take();
                slot.launching = false;
                Err(error)
            }
            Err(error) => {
                self.authority.close_admission();
                let mut workers = self.authority.workers()?;
                let slot = self.worker_slot(&mut workers, &grant)?;
                if slot.launching && slot.child.is_none() && slot.reaped.is_none() {
                    workers.slots[grant.slot] = None;
                }
                Err(SupervisorError::Io(error))
            }
        }
    }
    fn worker_slot<'a>(
        &self,
        workers: &'a mut WorkerLedger,
        grant: &WorkerGrant,
    ) -> Result<&'a mut WorkerSlot> {
        if !Arc::ptr_eq(&self.authority, &grant.authority) {
            return Err(SupervisorError::State);
        }
        workers
            .slots
            .get_mut(grant.slot)
            .and_then(Option::as_mut)
            .filter(|slot| slot.serial == grant.serial && slot.role == grant.role)
            .ok_or(SupervisorError::State)
    }
    /// Preserve the caller's exact Child on every refusal. Before real launch
    /// integration, spawning must be encapsulated so no cancellation can cover
    /// the interval between spawn and this custody transfer.
    pub(crate) fn bind_worker(
        &mut self,
        grant: &WorkerGrant,
        child: &mut Option<Child>,
    ) -> Result<()> {
        let mut workers = self.authority.workers()?;
        let slot = self.worker_slot(&mut workers, grant)?;
        if self.phase != Phase::Open
            || !self.authority.admission_open()
            || slot.child.is_some()
            || slot.reaped.is_some()
            || child.is_none()
        {
            return Err(SupervisorError::State);
        }
        slot.child = child.take();
        slot.launching = false;
        Ok(())
    }
    pub(crate) fn try_reap_worker(&mut self, grant: &WorkerGrant) -> Result<Option<ExitStatus>> {
        let mut workers = self.authority.workers()?;
        let slot = self.worker_slot(&mut workers, grant)?;
        if let Some(status) = slot.reaped {
            return Ok(Some(status));
        }
        let child = slot.child.as_mut().ok_or(SupervisorError::State)?;
        let status = child.try_wait().map_err(|error| {
            // Keep the owned Child and its slot charged on an unknown outcome.
            self.authority.close_admission();
            SupervisorError::Io(error)
        })?;
        if let Some(status) = status {
            slot.reaped = Some(status);
            slot.child.take();
            if !status.success() {
                self.authority.close_admission();
            }
        }
        Ok(status)
    }
    /// Trusted launcher calls this only after validating its backend shutdown
    /// contract. A successful process exit alone cannot establish that fact.
    pub(crate) fn confirm_worker_exit(&mut self, grant: &WorkerGrant) -> Result<()> {
        let mut workers = self.authority.workers()?;
        let slot = self.worker_slot(&mut workers, grant)?;
        if slot.child.is_some() || !slot.reaped.is_some_and(|status| status.success()) {
            return Err(SupervisorError::State);
        }
        workers.slots[grant.slot] = None;
        Ok(())
    }
    pub fn close_admission(&mut self) {
        self.authority.close_admission();
        if self.budget.admission_barrier().is_err() {
            self.phase = Phase::Fenced;
        } else if self.phase == Phase::Open {
            self.phase = Phase::Closing;
        }
    }
    pub fn try_finish(&mut self) -> Result<()> {
        self.close_admission();
        if self.phase == Phase::Finished {
            return Ok(());
        }
        if self.phase == Phase::Fenced {
            return Err(SupervisorError::State);
        }
        let snapshot = self.budget.snapshot()?;
        if snapshot.fenced {
            self.phase = Phase::Fenced;
            return Err(SupervisorError::State);
        }
        if snapshot.usage != TransportLimits::default() {
            return Err(SupervisorError::Busy);
        }
        #[cfg(feature = "canonical-host")]
        {
            let retained = self.retained.get_mut().map_err(|_| SupervisorError::State)?;
            if retained.as_ref().is_some_and(|inventory| !inventory.is_empty().unwrap_or(false)) {
                return Err(SupervisorError::Busy);
            }
            // Other actual aliases keep the original bookkeeping charged.
            retained.take();
        }
        if !self.resources.ensure_drained()? {
            return Err(SupervisorError::Busy);
        }
        {
            let workers = self.authority.workers()?;
            if workers
                .slots
                .iter()
                .flatten()
                .any(|slot| slot.launching || slot.child.is_some() || slot.reaped.is_none())
            {
                return Err(SupervisorError::Busy);
            }
        }
        // All owned Children are positively reaped. Failed/unconfirmed backend
        // slots never enabled reuse, but final retirement may now attempt the
        // original kernel holder-exclusion proof, outside both ledger locks.
        let result = match self.authority.owner.lock() {
            Ok(mut owner) => owner.finish(self.authority.epoch),
            Err(_) => {
                self.phase = Phase::Fenced;
                return Err(SupervisorError::State);
            }
        };
        match result {
            Ok(()) => {
                self.phase = Phase::Finished;
                Ok(())
            }
            Err(OwnerError::Busy) => Err(SupervisorError::Busy),
            Err(error) => {
                self.phase = Phase::Fenced;
                Err(error.into())
            }
        }
    }
    pub(crate) fn close_and_drain_workers(&mut self) -> Result<()> {
        self.close_admission();
        // No kernel process operations under the worker or transport ledger.
        // The controller's authority stays owned throughout this drain.
        let mut pending: [Option<Child>; 2] = {
            let mut workers = self
                .authority
                .workers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::array::from_fn(|index| {
                workers.slots[index]
                    .as_mut()
                    .and_then(|slot| slot.child.take())
            })
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut reaped = [None; 2];
        for child in pending.iter_mut().flatten() {
            // Signal only the original owned Child. Even a failed kill must
            // still be followed by positive reaping or retained quarantine.
            let _ = child.kill();
        }
        for (index, child) in pending.iter_mut().enumerate() {
            while let Some(owned) = child.as_mut() {
                match owned.try_wait() {
                    Ok(Some(status)) => {
                        reaped[index] = Some(status);
                        child.take();
                        break;
                    }
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::park_timeout(Duration::from_millis(5));
                    }
                    _ => break,
                }
            }
        }
        let unknown = pending.iter().any(Option::is_some);
        {
            let mut workers = self
                .authority
                .workers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for (index, child) in pending.into_iter().enumerate() {
                if let Some(status) = reaped[index] {
                    workers.slots[index].as_mut().unwrap().reaped = Some(status);
                }
                if let Some(child) = child {
                    // These slots cannot be removed/reused after close, and
                    // the controller's exclusive borrow prevents concurrent
                    // lifecycle calls during this transfer back to custody.
                    workers.slots[index].as_mut().unwrap().child = Some(child);
                }
            }
        }
        if unknown {
            Err(SupervisorError::Busy)
        } else {
            Ok(())
        }
    }
}
impl Drop for InstalledSupervisor {
    fn drop(&mut self) {
        if self.close_and_drain_workers().is_err() {
            // Bounded failure quarantine: retain exactly one authority Arc,
            // hence owner EX and at most two exact Child handles, until this
            // host process exits. Never publish Clean or refund uncertain
            // workers. This installation cannot be reopened in this process.
            std::mem::forget(self.authority.clone());
            eprintln!(
                "installed worker termination unknown; installation exclusion retained until process exit"
            );
        }
    }
}

#[cfg(test)]
#[path = "installed_supervisor/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "installed_supervisor/spawn_tests.rs"]
mod spawn_tests;

#[cfg(test)]
#[path = "installed_supervisor/drain_tests.rs"]
mod drain_tests;

#[cfg(all(test, feature = "canonical-host"))]
#[path = "installed_supervisor/retained_tests.rs"]
mod retained_tests;
