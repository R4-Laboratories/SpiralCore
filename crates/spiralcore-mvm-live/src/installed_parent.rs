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

mod resident;
pub use resident::{InstalledResident, ResidentId, ResidentProgress, ResidentStatus};

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
    lifecycle: Mutex<GrantLifecycle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum EndpointIssuance {
    #[default]
    NeverIssued,
    Preparing,
    Issued,
    Uncertain,
    CleanupComplete,
}
#[derive(Default)]
struct GrantLifecycle {
    admitted_plan: Option<String>,
    endpoint: EndpointIssuance,
    retired: bool,
}
impl GrantLifecycle {
    fn check_launch_open(&self) -> Result<()> {
        ensure!(
            !self.retired
                && matches!(
                    self.endpoint,
                    EndpointIssuance::NeverIssued
                        | EndpointIssuance::Preparing
                        | EndpointIssuance::Issued
                ),
            "original grant retired or endpoint unavailable"
        );
        Ok(())
    }
    fn seal_retired(&mut self) -> Result<()> {
        ensure!(
            self.admitted_plan.is_some(),
            "original canonical admission unavailable"
        );
        ensure!(
            matches!(
                self.endpoint,
                EndpointIssuance::NeverIssued | EndpointIssuance::CleanupComplete
            ),
            "original endpoint cleanup incomplete"
        );
        self.retired = true;
        Ok(())
    }

    fn seal_unused(&mut self) -> Result<()> {
        ensure!(
            self.admitted_plan.is_none() && self.endpoint == EndpointIssuance::NeverIssued,
            "original preparation has prior admission or endpoint effects"
        );
        self.retired = true;
        Ok(())
    }
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
    completed: bool,
}
impl ParentLaunchOwner {
    pub fn binding(&self) -> &HostLaunchBinding {
        &self.grant.binding
    }

    /// Cancel only the same unconverted preparation returned by canonical pure
    /// preflight. Borrowing keeps that original owner live through every check.
    pub fn cancel_unused(
        &mut self,
        unused: &mvm_client::UnusedHostLifetimePreparation,
    ) -> Result<()> {
        ensure!(
            unused.binding() == &self.grant.binding,
            "unused preparation binding differs"
        );
        ensure!(
            unused.retains_parent_owner(&self.grant),
            "unused preparation owner differs"
        );
        self.grant.reservation.check_retained_identity()?;
        self.grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .seal_unused()?;
        self.completed = true;
        Ok(())
    }

    /// Complete only this original native owner after its sealed retirement
    /// and already-drained endpoint state. This performs no endpoint/disk I/O.
    pub fn finish_retired(&mut self, outcome: &mvm_client::LaunchOutcome) -> Result<()> {
        outcome.with_owned_retirement(|receipt| self.require_original_retirement(receipt))??;
        self.complete_already_drained()
    }

    /// Failed startup retains the linear receipt returned by the SAME original
    /// backend's stop_owned_pending. Missing history never completes this owner.
    pub fn finish_pending_retired(&mut self, receipt: &mvm_client::OwnedRetirement) -> Result<()> {
        self.require_original_retirement(receipt)?;
        self.complete_already_drained()
    }

    fn require_original_retirement(&self, receipt: &mvm_client::OwnedRetirement) -> Result<()> {
        let original: Arc<dyn CoreCurrentParentPermit> = self.grant.clone();
        ensure!(
            receipt.binding() == &self.grant.binding,
            "original retirement binding differs"
        );
        ensure!(
            receipt.retains_parent_permit(&original),
            "original retirement permit differs"
        );
        Ok(())
    }

    fn complete_already_drained(&mut self) -> Result<()> {
        // The short outcome lock is gone before touching any other owner.
        // This cleanup-safe check never reopens launch admission or does I/O.
        self.grant.reservation.check_retained_identity()?;
        self.grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .seal_retired()?;
        self.completed = true;
        Ok(())
    }
}
impl Drop for ParentLaunchOwner {
    fn drop(&mut self) {
        // Each retained owner consumes one of the SAME two guest reservations.
        // No record eviction, PID query or caller bool can release uncertainty.
        // Incomplete ownership stays quarantined until process exit. Only an
        // explicit original unused/retired completion disarms this disposition.
        if !self.completed {
            std::mem::forget(self.grant.clone());
        }
    }
}
impl PreparedParentVm {
    pub fn binding(&self) -> &HostLaunchBinding {
        &self.grant.binding
    }
    pub fn begin_launch(self) -> Result<(PreparedHostLifetime, ParentLaunchOwner)> {
        let grant = self.grant.clone();
        let lifetime = self.into_lifetime()?;
        Ok((
            lifetime,
            ParentLaunchOwner {
                grant,
                completed: false,
            },
        ))
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
            lifecycle: Mutex::new(GrantLifecycle::default()),
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
        self.lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .check_launch_open()?;
        Ok(())
    }
}

impl CoreCurrentParentPermit for VmGrant {
    fn check_current(&self, binding: &HostLaunchBinding) -> Result<()> {
        ensure!(*binding == self.binding, "parent binding substituted");
        self.check_open()?;
        let lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
        lifecycle.check_launch_open()?;
        ensure!(
            lifecycle.admitted_plan.is_some(),
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
        let mut lifecycle = grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
        lifecycle.check_launch_open()?;
        ensure!(
            lifecycle.admitted_plan.is_none(),
            "original parent grant already bound"
        );
        lifecycle.admitted_plan = Some(admitted.plan_id().0.clone());
        drop(lifecycle);
        grant.check_current(binding)?;
        Ok(grant)
    }
}

#[cfg(test)]
mod retirement_policy_tests {
    use super::*;

    // These exercise private bookkeeping policy, never native-exit evidence.
    fn admitted_policy(endpoint: EndpointIssuance) -> GrantLifecycle {
        GrantLifecycle {
            admitted_plan: Some("state-policy-only".into()),
            endpoint,
            retired: false,
        }
    }

    #[test]
    fn incomplete_endpoint_policy_cannot_complete_the_vm_owner() {
        for endpoint in [
            EndpointIssuance::Preparing,
            EndpointIssuance::Issued,
            EndpointIssuance::Uncertain,
        ] {
            let mut state = admitted_policy(endpoint);
            assert!(state.seal_retired().is_err());
            assert!(!state.retired);
            assert_eq!(state.endpoint, endpoint);
        }
    }

    #[test]
    fn eligible_policy_seals_against_later_launch_work_idempotently() {
        for endpoint in [
            EndpointIssuance::NeverIssued,
            EndpointIssuance::CleanupComplete,
        ] {
            let mut state = admitted_policy(endpoint);
            state.seal_retired().unwrap();
            assert!(state.retired);
            assert!(state.check_launch_open().is_err());
            state.seal_retired().unwrap();
        }
    }

    #[test]
    fn missing_admission_and_uncertain_endpoint_policy_refuse_without_reset() {
        let mut unadmitted = GrantLifecycle::default();
        assert!(unadmitted.seal_retired().is_err());
        assert!(!unadmitted.retired);
        for endpoint in [
            EndpointIssuance::Uncertain,
            EndpointIssuance::CleanupComplete,
        ] {
            assert!(admitted_policy(endpoint).check_launch_open().is_err());
        }
        assert!(
            admitted_policy(EndpointIssuance::NeverIssued)
                .check_launch_open()
                .is_ok()
        );
    }

    #[test]
    fn unused_cancellation_policy_requires_no_admission_and_no_endpoint_effects() {
        let mut unused = GrantLifecycle::default();
        unused.seal_unused().unwrap();
        assert!(unused.retired);
        assert!(unused.check_launch_open().is_err());
        unused.seal_unused().unwrap();
        let mut admitted = admitted_policy(EndpointIssuance::NeverIssued);
        assert!(admitted.seal_unused().is_err());
        assert!(!admitted.retired);
        for endpoint in [
            EndpointIssuance::Preparing,
            EndpointIssuance::Issued,
            EndpointIssuance::Uncertain,
            EndpointIssuance::CleanupComplete,
        ] {
            let mut pending = GrantLifecycle {
                endpoint,
                ..Default::default()
            };
            assert!(pending.seal_unused().is_err());
            assert!(!pending.retired);
        }
    }
}
