//! One installation owner and one transitively retained transport ledger.
//! Worker activation is a separate boundary; this module grants no boot API.
use crate::{
    host_budget::{
        HostTransportBudget, TransportBudgetConfig, TransportBudgetError, TransportLimits,
    },
    installed_owner::{Epoch, InstallationOwner, OwnerError},
};
use sha2::{Digest, Sha256};
use std::{
    fmt, io,
    process::{Child, ExitStatus},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub struct InstalledConfig {
    _private: (),
}
impl InstalledConfig {
    pub fn native_profile() -> Self {
        Self { _private: () }
    }
    pub fn profile_hash(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(b"spiralcore/installed-profile/v1\0");
        for value in [2_u64, 2, 524_288, 1, 262_144, 2, 1, 1024] {
            digest.update(value.to_be_bytes());
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
    Io(io::Error),
}
impl fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "installed supervisor: {self:?}")
    }
}
impl std::error::Error for SupervisorError {}
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
type Result<T> = std::result::Result<T, SupervisorError>;

pub(crate) struct InstalledAuthority {
    owner: Mutex<InstallationOwner>,
    epoch: Epoch,
    open: AtomicBool,
    workers: Mutex<WorkerLedger>,
}
impl InstalledAuthority {
    pub(crate) fn admission_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }
    pub(crate) fn close_admission(&self) {
        self.open.store(false, Ordering::Release);
    }
    fn workers(&self) -> Result<std::sync::MutexGuard<'_, WorkerLedger>> {
        self.workers.lock().map_err(|_| {
            self.close_admission();
            SupervisorError::State
        })
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

struct WorkerSlot {
    serial: u64,
    role: WorkerRole,
    child: Option<Child>,
    reaped: Option<ExitStatus>,
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
    phase: Phase,
}
impl InstalledSupervisor {
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
        });
        let budget = HostTransportBudget::new_installed(config.transport(), authority.clone())?;
        Ok(Self {
            authority,
            budget,
            phase: Phase::Open,
        })
    }
    pub fn budget(&self) -> &HostTransportBudget {
        &self.budget
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
        });
        Ok(WorkerGrant {
            authority: self.authority.clone(),
            slot,
            serial,
            role,
        })
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
        {
            let workers = self.authority.workers()?;
            if workers
                .slots
                .iter()
                .flatten()
                .any(|slot| slot.child.is_some() || slot.reaped.is_none())
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
}
impl Drop for InstalledSupervisor {
    fn drop(&mut self) {
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
        for child in pending.iter_mut().flatten() {
            // Signal only the original owned Child. Even a failed kill must
            // still be followed by positive reaping or retained quarantine.
            let _ = child.kill();
        }
        for child in &mut pending {
            while let Some(owned) = child.as_mut() {
                match owned.try_wait() {
                    Ok(Some(_)) => {
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
        if pending.iter().any(Option::is_some) {
            let mut workers = self
                .authority
                .workers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for (index, child) in pending.into_iter().enumerate() {
                if let Some(child) = child {
                    // These slots cannot be removed/reused after close, and
                    // the controller's exclusive borrow prevents concurrent
                    // lifecycle calls during this transfer back to custody.
                    workers.slots[index].as_mut().unwrap().child = Some(child);
                }
            }
            drop(workers);
            // Bounded failure quarantine: retain exactly one authority Arc,
            // hence owner EX and at most two exact Child handles, until this
            // host process exits. Never publish Clean or refund uncertain
            // workers. This installation cannot be reopened in this process.
            std::mem::forget(self.authority.clone());
            eprintln!("installed worker termination unknown; installation exclusion retained until process exit");
        }
    }
}

#[cfg(test)]
#[path = "installed_supervisor/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "installed_supervisor/spawn_tests.rs"]
mod spawn_tests;
