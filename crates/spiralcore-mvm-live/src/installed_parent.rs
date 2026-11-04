//! Original installed reservations bound to canonical opaque MVM admission.
use crate::{
    installed_resources::{ManagedClaim, ManagedReservation, ManagedResourceBudget},
    installed_supervisor::{InstalledNativeIssuer, InstalledSupervisor},
};
use anyhow::{Result, anyhow, ensure};
use mvm_client::{HostLifetimeIntent, LifecycleMode};
use mvm_core::{
    host_lifetime::{
        CoreCurrentParentPermit, HostLaunchBinding, HostLeaseExpectation, HostLifetimeInputs,
        PreparedHostLifetime,
    },
    vm_backend::BackendKind,
};
use mvm_hostd::{host_lifetime_parent::ParentAdmissionProvider, plan_admission::AdmittedPlan};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex, Weak};

const BOOKKEEPING_BYTES: u64 = 4096;
const MIB: u64 = 1024 * 1024;

struct Registry {
    closed: bool,
    slots: [Weak<VmGrant>; 2],
}
struct Shared {
    registry: Mutex<Registry>,
    pool: ManagedResourceBudget,
    issuer: InstalledNativeIssuer,
    _bookkeeping: ManagedReservation,
}

/// Local parent admission, never a replacement for signed MVM authorization.
pub struct InstalledParentProvider {
    shared: Arc<Shared>,
}
struct VmGrant {
    binding: HostLaunchBinding,
    tenant: String,
    reservation: ManagedReservation,
    parent: Weak<Shared>,
    admitted_plan: Mutex<Option<String>>,
}

/// Unused preparation refunds on Drop; launched custody keeps this same owner.
pub struct PreparedParentVm {
    grant: Arc<VmGrant>,
    handoff: crate::installed_owner::HolderHandoff,
}

/// Armed before canonical invocation; uncertainty cannot refund the guest.
/// Final completion requires the separate original retirement/drain integration.
pub struct ParentLaunchOwner {
    grant: Arc<VmGrant>,
}
impl ParentLaunchOwner {
    pub fn binding(&self) -> &HostLaunchBinding {
        &self.grant.binding
    }
}
impl Drop for ParentLaunchOwner {
    fn drop(&mut self) {
        // Each retained owner consumes one of the SAME two guest reservations.
        // No record eviction, PID query or caller bool can release uncertainty.
        // Keep this original owner until the process exits; a future positive
        // retirement path must explicitly disarm this conservative disposition.
        std::mem::forget(self.grant.clone());
    }
}
impl PreparedParentVm {
    pub fn binding(&self) -> &HostLaunchBinding {
        &self.grant.binding
    }
    pub fn begin_launch(self) -> Result<(PreparedHostLifetime, ParentLaunchOwner)> {
        let grant = self.grant.clone();
        let lifetime = self.into_lifetime()?;
        Ok((lifetime, ParentLaunchOwner { grant }))
    }
    pub fn into_lifetime(self) -> Result<PreparedHostLifetime> {
        self.grant.check_open()?;
        let inputs = HostLifetimeInputs {
            directory: self.handoff.directory,
            holder: self.handoff.holder,
            expected: HostLeaseExpectation {
                state_name: "state.bin".into(),
                state_bytes: self.handoff.state_bytes,
                state_sha256: self.handoff.state_sha256,
                absent_names: [".state.next".into(), ".state.intent".into()],
            },
            binding: self.grant.binding.clone(),
        };
        PreparedHostLifetime::bind(inputs, self.grant)
    }
}

fn tenant_tag(tenant: &str) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"SpiralCore/installed-parent/tenant/v1\0");
    hash.update((tenant.len() as u64).to_le_bytes());
    hash.update(tenant.as_bytes());
    hash.finalize().into()
}

impl InstalledParentProvider {
    pub fn new(supervisor: &InstalledSupervisor) -> Result<Self> {
        supervisor.require_canonical_profile()?;
        let pool = supervisor.managed_resources().clone();
        let bookkeeping = pool.reserve(
            tenant_tag("installation-parent-registry"),
            ManagedClaim {
                host_metadata_bytes: BOOKKEEPING_BYTES,
                ..Default::default()
            },
        )?;
        Ok(Self {
            shared: Arc::new(Shared {
                registry: Mutex::new(Registry {
                    closed: false,
                    slots: std::array::from_fn(|_| Weak::new()),
                }),
                pool,
                issuer: supervisor.native_issuer(),
                _bookkeeping: bookkeeping,
            }),
        })
    }

    pub fn close_admission(&self) {
        match self.shared.registry.lock() {
            Ok(mut registry) => registry.closed = true,
            // Poisoned registries refuse every subsequent lock operation.
            Err(poisoned) => poisoned.into_inner().closed = true,
        }
    }

    pub fn prepare_vm(
        &self,
        intent: &HostLifetimeIntent,
        tenant: &str,
        backend: BackendKind,
    ) -> Result<PreparedParentVm> {
        ensure!(
            !tenant.is_empty() && tenant.len() <= 128,
            "bounded nonempty parent tenant required"
        );
        let name = intent
            .name
            .as_deref()
            .ok_or_else(|| anyhow!("explicit VM name required"))?;
        mvm_core::naming::validate_vm_name(name)?;
        ensure!(
            intent.mode == LifecycleMode::Transient
                && intent.cpus == 1
                && (1..=1024).contains(&intent.memory_mib),
            "installed parent native profile exceeded"
        );
        ensure!(
            (cfg!(target_os = "linux") && backend == BackendKind::Firecracker)
                || (cfg!(all(target_os = "macos", target_arch = "aarch64"))
                    && backend == BackendKind::Hvf),
            "unsupported installed native role"
        );
        let backend_name = match backend {
            BackendKind::Firecracker => "firecracker",
            BackendKind::Hvf => "hvf",
            _ => return Err(anyhow!("unsupported installed native backend")),
        };
        ensure!(
            intent.backend.as_deref().is_none_or(|b| b == backend_name),
            "requested backend differs from original backend"
        );
        // Fresh filesystem checks occur before taking the registry lock.
        let reservation = self.shared.pool.reserve(
            tenant_tag(tenant),
            ManagedClaim {
                host_metadata_bytes: BOOKKEEPING_BYTES,
                guest_memory_bytes: u64::from(intent.memory_mib) * MIB,
                guests: 1,
                ..Default::default()
            },
        )?;
        let (installation, handoff) = self.shared.issuer.prepare()?;
        let mut registry = self
            .shared
            .registry
            .lock()
            .map_err(|_| anyhow!("parent registry poisoned"))?;
        ensure!(!registry.closed, "parent admission closed");
        let slot = registry
            .slots
            .iter()
            .position(|slot| slot.upgrade().is_none())
            .ok_or_else(|| anyhow!("original VM grant slots full"))?;
        ensure!(
            registry
                .slots
                .iter()
                .filter_map(Weak::upgrade)
                .all(|g| g.binding.vm_name != name),
            "original VM name already reserved"
        );
        let serial = reservation.serial();
        let grant = Arc::new(VmGrant {
            binding: HostLaunchBinding {
                installation: installation.key.installation,
                profile: installation.key.profile,
                generation: installation.generation,
                grant_serial: serial,
                vm_name: name.into(),
                backend,
                cpus: intent.cpus,
                memory_mib: intent.memory_mib,
            },
            tenant: tenant.into(),
            reservation,
            parent: Arc::downgrade(&self.shared),
            admitted_plan: Mutex::new(None),
        });
        registry.slots[slot] = Arc::downgrade(&grant);
        Ok(PreparedParentVm { grant, handoff })
    }
}

impl VmGrant {
    fn check_open(&self) -> Result<()> {
        self.reservation.check_current()?;
        let parent = self
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original parent unavailable"))?;
        let registry = parent
            .registry
            .lock()
            .map_err(|_| anyhow!("parent registry poisoned"))?;
        ensure!(!registry.closed, "parent admission closed");
        Ok(())
    }
}

impl CoreCurrentParentPermit for VmGrant {
    fn check_current(&self, binding: &HostLaunchBinding) -> Result<()> {
        ensure!(*binding == self.binding, "parent binding substituted");
        self.check_open()?;
        ensure!(
            self.admitted_plan
                .lock()
                .map_err(|_| anyhow!("parent admission poisoned"))?
                .is_some(),
            "parent grant has no canonical admission"
        );
        Ok(())
    }
}

impl ParentAdmissionProvider for InstalledParentProvider {
    fn bind(
        &self,
        admitted: &AdmittedPlan,
        binding: &HostLaunchBinding,
    ) -> Result<Arc<dyn CoreCurrentParentPermit>> {
        let grant = {
            let registry = self
                .shared
                .registry
                .lock()
                .map_err(|_| anyhow!("parent registry poisoned"))?;
            ensure!(!registry.closed, "parent admission closed");
            registry
                .slots
                .iter()
                .filter_map(Weak::upgrade)
                .find(|grant| grant.binding == *binding)
                .ok_or_else(|| anyhow!("original parent grant unavailable"))?
        };
        grant.reservation.check_current()?;
        let plan = admitted.plan();
        let runtime = match binding.backend {
            BackendKind::Firecracker => "firecracker",
            BackendKind::Hvf => "hvf",
            _ => return Err(anyhow!("unsupported original parent backend")),
        };
        ensure!(
            plan.tenant.0 == grant.tenant
                && plan.workload.0 == binding.vm_name
                && plan.runtime_profile.0 == runtime
                && plan.resources.cpus == binding.cpus
                && plan.resources.mem_mib == u64::from(binding.memory_mib),
            "canonical admission differs from original parent grant"
        );
        let mut bound = grant
            .admitted_plan
            .lock()
            .map_err(|_| anyhow!("parent admission poisoned"))?;
        ensure!(bound.is_none(), "original parent grant already bound");
        *bound = Some(admitted.plan_id().0.clone());
        drop(bound);
        grant.check_current(binding)?;
        Ok(grant)
    }
}
