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
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
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
}
impl InstalledAuthority {
    pub(crate) fn admission_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }
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
    pub fn close_admission(&mut self) {
        self.authority.open.store(false, Ordering::Release);
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
        // No worker activation exists in this component slice. Once workers
        // are admitted, their owned/reaped ledger must gate this transition.
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
    }
}
