//! Original installed reservations bound to canonical opaque MVM admission.
use crate::{
    installed_resources::{ManagedClaim, ManagedReservation, ManagedResourceBudget},
    installed_supervisor::{InstalledNativeIssuer, InstalledSupervisor},
};
use anyhow::{Result, anyhow, ensure};
use mvm_core::vm_backend::VmStartConfig;
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

#[path = "installed_parent/audit_archive.rs"]
pub(crate) mod audit_archive;
#[path = "installed_parent/endpoint.rs"]
mod endpoint;
#[path = "installed_parent/host_agent.rs"]
mod host_agent;
#[path = "installed_parent/tenant_agent.rs"]
mod tenant_agent;
pub use tenant_agent::TenantHelperLimits;
#[path = "installed_parent/startup.rs"]
pub(crate) mod startup;
pub use startup::PortableStartupQuote;
#[cfg(target_os = "linux")]
#[path = "installed_parent/driver.rs"]
mod driver;
#[cfg(target_os = "linux")]
pub use driver::CanonicalLaunch;
#[cfg(target_os = "linux")]
#[path = "installed_parent/startup_bounds.rs"]
mod startup_bounds;

#[cfg(test)]
#[path = "installed_parent/config_tests.rs"]
mod config_tests;
#[cfg(test)]
#[path = "installed_parent/endpoint_tests.rs"]
mod endpoint_tests;

const BOOKKEEPING_BYTES: u64 = 4096;
const MIB: u64 = 1024 * 1024;

struct Registry {
    closed: bool,
    slots: [Weak<VmGrant>; 2],
}
struct Shared {
    registry: Mutex<Registry>,
    _original_controller: Arc<()>,
    pool: ManagedResourceBudget,
    issuer: InstalledNativeIssuer,
    _bookkeeping: ManagedReservation,
    endpoint_inputs: Option<endpoint::EndpointOwnerInputs>,
    tenant_helper: Mutex<tenant_agent::TenantSlot>,
    audit_debts: Arc<Mutex<audit_archive::Inventory>>,
    portable_debts: Arc<Mutex<startup::Inventory>>,
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
    _original_controller: Arc<()>,
    lifecycle: Mutex<GrantLifecycle>,
    original: Weak<VmGrant>,
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
    armed: bool,
    freshness: Option<mvm_core::plan::validity::CheckedFreshness>,
    admitted_plan: Option<String>,
    admitted_session: Option<[u8; 32]>,
    selected_network: Option<mvm_vmm::host::network_endpoint_spawn::AdmittedNetworkPlan>,
    registration: Option<Arc<host_agent::RegistrationBinding>>,
    startup: Option<startup::StartupWorkspace>,
    endpoint: EndpointIssuance,
    endpoint_owner: Option<Arc<endpoint::RootEndpointOwner>>,
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
    endpoint_retirement: Option<mvm_client::EndpointRetirement>,
}
impl ParentLaunchOwner {
    /// Release an unused preflight only when the SDK returns the unconsumed
    /// lifetime issued by this same linear owner.
    pub fn cancel_unused(
        &mut self,
        unused: &mvm_client::UnusedHostLifetimePreparation,
    ) -> Result<()> {
        ensure!(
            unused.binding() == &self.grant.binding && unused.retains_parent_owner(&self.grant),
            "foreign unused host lifetime"
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

    /// Finalize a positively stopped SDK outcome after explicit endpoint disposal.
    pub fn finish_retired(&mut self, outcome: &mvm_client::LaunchOutcome) -> Result<()> {
        outcome.with_owned_retirement(|receipt| self.require_original_retirement(receipt))??;
        self.complete_already_drained()
    }

    /// Failed native startup uses the receipt from that same backend's pending stop.
    pub fn finish_pending_retired(&mut self, receipt: &mvm_client::OwnedRetirement) -> Result<()> {
        self.require_original_retirement(receipt)?;
        self.complete_already_drained()
    }

    fn require_original_retirement(&self, receipt: &mvm_client::OwnedRetirement) -> Result<()> {
        let permit: Arc<dyn CoreCurrentParentPermit> = self.grant.clone();
        ensure!(
            receipt.binding() == &self.grant.binding && receipt.retains_parent_permit(&permit),
            "retirement differs from original VM owner"
        );
        Ok(())
    }

    fn complete_already_drained(&mut self) -> Result<()> {
        self.grant.reservation.check_retained_identity()?;
        self.grant.check_retired_portable_debt()?;
        self.grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .seal_retired()?;
        self.completed = true;
        Ok(())
    }

    pub fn binding(&self) -> &HostLaunchBinding {
        &self.grant.binding
    }

    /// Persist the original endpoint receipt in this launch owner before cleanup.
    /// On an I/O failure retain the same receipt and retry with retry_endpoint_cleanup.
    /// The VM reservation remains charged until the separately held original VMM retires.
    pub fn retire_endpoint(
        &mut self,
        pending: &mut Option<mvm_client::EndpointRetirement>,
    ) -> Result<()> {
        ensure!(
            self.endpoint_retirement.is_none(),
            "original endpoint receipt already retained"
        );
        let state = self
            .grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
        let owner = state
            .endpoint_owner
            .as_ref()
            .ok_or_else(|| anyhow!("original endpoint owner missing"))?;
        let receipt = pending
            .as_ref()
            .ok_or_else(|| anyhow!("original endpoint receipt missing"))?;
        ensure!(
            owner.matches_receipt(receipt),
            "foreign endpoint retirement"
        );
        drop(state);
        self.endpoint_retirement = pending.take();
        self.retry_endpoint_cleanup()
    }
    pub fn retry_endpoint_cleanup(&mut self) -> Result<()> {
        ensure!(
            self.endpoint_retirement.is_some(),
            "original endpoint receipt missing"
        );
        self.cleanup_endpoint()
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
            if let Some(receipt) = self.endpoint_retirement.take() {
                std::mem::forget(receipt);
            }
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
        grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .armed = true;
        Ok((
            lifetime,
            ParentLaunchOwner {
                grant,
                completed: false,
                endpoint_retirement: None,
            },
        ))
    }
    fn into_lifetime(self) -> Result<PreparedHostLifetime> {
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
        let lifetime = PreparedHostLifetime::bind(inputs, self.grant.clone())?;
        self.grant.bind_portable_quote(lifetime)
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
        let original_controller = supervisor.claim_parent_controller()?;
        Ok(Self {
            shared: Arc::new(Shared {
                _original_controller: original_controller,
                registry: Mutex::new(Registry {
                    closed: false,
                    slots: std::array::from_fn(|_| Weak::new()),
                }),
                pool,
                issuer: supervisor.native_issuer(),
                _bookkeeping: bookkeeping,
                endpoint_inputs: None,
                tenant_helper: Mutex::new(tenant_agent::TenantSlot::Vacant),
                audit_debts: supervisor.audit_inventory.clone(),
                portable_debts: supervisor.portable_inventory.clone(),
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

    /// Reserve from the SDK request that will consume the prepared lifetime.
    /// The request supplies sizing hints only; bind_config still verifies the
    /// original signed plan, tenant, network asset and armed grant.
    pub fn prepare_request(
        &self,
        request: &mvm_client::LaunchRequest,
        tenant: &str,
        backend: BackendKind,
    ) -> Result<PreparedParentVm> {
        let intent = request.host_lifetime_intent();
        ensure!(
            intent.mode == mvm_client::LifecycleMode::Transient,
            "owned endpoint requires a transient SDK launch"
        );
        let backend_name = match backend {
            BackendKind::Firecracker => "firecracker",
            BackendKind::Hvf => "hvf",
            _ => anyhow::bail!("unsupported installed native backend"),
        };
        ensure!(
            intent
                .backend
                .as_deref()
                .is_none_or(|requested| requested == backend_name),
            "SDK backend differs"
        );
        let config = VmStartConfig {
            name: intent
                .name
                .ok_or_else(|| anyhow!("owned SDK VM name missing"))?,
            cpus: intent.cpus,
            memory_mib: intent.memory_mib,
            tenant_id: Some(tenant.to_owned()),
            ..Default::default()
        };
        self.prepare_vm(&config, tenant, backend)
    }

    pub fn prepare_vm(
        &self,
        config: &VmStartConfig,
        tenant: &str,
        backend: BackendKind,
    ) -> Result<PreparedParentVm> {
        ensure!(
            !tenant.is_empty() && tenant.len() <= 128,
            "bounded nonempty parent tenant required"
        );
        let name = config.name.as_str();
        mvm_core::naming::validate_vm_name(name)?;
        ensure!(
            config.tenant_id.as_deref() == Some(tenant) && config.host_lifetime.is_none(),
            "parent requires matching tenant and unbound original config"
        );
        ensure!(
            config.cpus == 1
                && (1..=1024).contains(&config.memory_mib)
                && config.warm_pool_size == 0,
            "installed parent native profile exceeded"
        );
        ensure!(
            (cfg!(target_os = "linux") && backend == BackendKind::Firecracker)
                || (cfg!(all(target_os = "macos", target_arch = "aarch64"))
                    && backend == BackendKind::Hvf),
            "unsupported installed native role"
        );
        // The selected FC driver's original workload-exit listener lives until
        // native settlement. Its worker and known Rust stack stay with the VM.
        #[cfg(target_os = "linux")]
        let (workers, host_stack_bytes) =
            if self.shared.endpoint_inputs.is_some() && backend == BackendKind::Firecracker {
                (
                    1,
                    mvm_vmm::host::fc_lifetime::worker_terms::owned_exit_capture_stack_terms()?
                        .known_rust_bytes,
                )
            } else {
                (0, 0)
            };
        #[cfg(not(target_os = "linux"))]
        let (workers, host_stack_bytes) = (0, 0);
        // Fresh filesystem checks occur before taking the registry lock.
        let reservation = self.shared.pool.reserve(
            tenant_tag(tenant),
            ManagedClaim {
                host_metadata_bytes: BOOKKEEPING_BYTES,
                workers,
                host_stack_bytes,
                guest_memory_bytes: u64::from(config.memory_mib) * MIB,
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
        let grant = Arc::new_cyclic(|original| VmGrant {
            binding: HostLaunchBinding {
                installation: installation.key.installation,
                profile: installation.key.profile,
                generation: installation.generation,
                grant_serial: serial,
                vm_name: name.into(),
                backend,
                cpus: config.cpus,
                memory_mib: config.memory_mib,
            },
            tenant: tenant.into(),
            reservation,
            parent: Arc::downgrade(&self.shared),
            _original_controller: self.shared._original_controller.clone(),
            lifecycle: Mutex::new(GrantLifecycle::default()),
            original: original.clone(),
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
    #[cfg(target_os = "linux")]
    fn authorize_original_root_copy(
        &self,
        binding: &HostLaunchBinding,
        kernel: &Arc<mvm_core::canonical_linux::PreparedCanonicalLinux>,
        directory: &std::fs::File,
        bytes: std::num::NonZeroU64,
    ) -> Result<()> {
        self.check_current(binding)?;
        self.check_original_kernel(kernel, Some((directory, bytes)))
    }
    fn begin_startup_workspace(&self, binding: &HostLaunchBinding) -> Result<()> {
        self.transition_portable_workspace(binding, startup::Phase::Held, startup::Phase::Active)
    }
    fn complete_startup_workspace(&self, binding: &HostLaunchBinding) -> Result<()> {
        self.transition_portable_workspace(
            binding,
            startup::Phase::Active,
            startup::Phase::Complete,
        )
    }
    fn prepare_endpoint_partition(
        &self,
        binding: &HostLaunchBinding,
    ) -> Result<mvm_core::host_lifetime::PreparedEndpointPartition> {
        {
            let lifecycle = self
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            ensure!(
                lifecycle
                    .startup
                    .as_ref()
                    .is_none_or(|s| s.phase == startup::Phase::Complete),
                "original portable workspace has not completed"
            );
        }
        self.prepare_original_endpoint(binding)
    }

    fn check_current(&self, binding: &HostLaunchBinding) -> Result<()> {
        ensure!(*binding == self.binding, "parent binding substituted");
        self.check_open()?;
        let lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
        lifecycle.check_launch_open()?;
        ensure!(
            lifecycle.armed && lifecycle.admitted_plan.is_some(),
            "original launch owner or canonical admission unavailable"
        );
        use mvm_core::time::{Clock, SystemClock};
        let window = lifecycle
            .freshness
            .as_ref()
            .ok_or_else(|| anyhow!("original admitted validity window missing"))?;
        mvm_core::plan::check_window(window, SystemClock.now())?;
        Ok(())
    }
}

impl InstalledParentProvider {
    fn original_grant(&self, binding: &HostLaunchBinding) -> Result<Arc<VmGrant>> {
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
        Ok(grant)
    }
    fn bind_original_admission(
        &self,
        admitted: &AdmittedPlan,
        binding: &HostLaunchBinding,
        config: Option<&mvm_core::vm_backend::VmStartConfig>,
    ) -> Result<Arc<dyn CoreCurrentParentPermit>> {
        let grant = self.original_grant(binding)?;
        grant.reservation.check_current()?;
        grant.check_open()?;
        ensure!(
            grant
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?
                .armed,
            "original parent launch is not armed"
        );
        let plan = admitted.plan();
        let runtime = match binding.backend {
            BackendKind::Firecracker => "firecracker",
            BackendKind::Hvf => "hvf",
            _ => return Err(anyhow!("unsupported original parent backend")),
        };
        ensure!(
            plan.tenant.0 == grant.tenant
                && (config.is_some() || plan.workload.0 == binding.vm_name)
                && plan.runtime_profile.0 == runtime
                && plan.resources.cpus == binding.cpus
                && plan.resources.mem_mib == u64::from(binding.memory_mib),
            "canonical admission differs from original parent grant"
        );
        let selected_network =
            config
                .and_then(|cfg| {
                    cfg.network_policy
                        .payload_mapping()
                        .map(|mapping| (cfg, mapping))
                })
                .map(|(cfg, mapping)| -> Result<_> {
                    use mvm_contract::policy::network_policy::PayloadMappingProfile;
                    ensure!(
                        mapping.peers().iter().all(|peer| peer.profile
                            == PayloadMappingProfile::SpiralCoreObjectStreamSmallV1),
                        "installed endpoint requires the small object stream mapping"
                    );
                    mvm_vmm::host::network_endpoint_spawn::AdmittedNetworkPlan::from_plan(
                        plan,
                        &cfg.name,
                        &cfg.network_policy,
                    )
                })
                .transpose()?;
        if let Some(config) = config {
            ensure!(
                config.name == binding.vm_name
                    && config.tenant_id.as_deref() == Some(grant.tenant.as_str()),
                "original launch config identity differs"
            );
            let anchor = config
                .host_lifetime
                .as_ref()
                .ok_or_else(|| anyhow!("original launch anchor missing"))?;
            ensure!(
                anchor.retains_parent_owner(&grant),
                "launch anchor does not retain original parent grant"
            );
            // Recheck the actual opaque plan here, even when the canonical
            // caller already did so. This public DI method may be called
            // directly; equal config/plan labels are not admission continuity.
            anchor.bind_admitted(config, binding.backend, plan)?;
        }
        // Validate and reserve the registration candidate before committing
        // this grant. A pure preflight failure must not consume admission.
        // Its local owner outlives the lock below, so a losing concurrent
        // candidate releases its storage only after that mutex guard drops.
        let registration = config
            .map(|config| grant.prepare_original_registration(admitted, config, binding.backend))
            .transpose()?;
        let mut lifecycle = grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
        lifecycle.check_launch_open()?;
        ensure!(
            lifecycle.admitted_plan.is_none() && lifecycle.registration.is_none(),
            "original parent grant already bound"
        );
        lifecycle.admitted_plan = Some(admitted.plan_id().0.clone());
        let mut session = Sha256::new();
        session.update(b"SpiralCore/installed-parent/endpoint-session/v1\0");
        session.update(binding.installation);
        session.update(binding.generation.to_le_bytes());
        session.update(binding.grant_serial.to_le_bytes());
        session.update(admitted.plan_id().0.as_bytes());
        session.update(plan.nonce.as_hex().as_bytes());
        lifecycle.admitted_session = Some(session.finalize().into());
        lifecycle.freshness = Some(mvm_core::plan::validity::CheckedFreshness {
            valid_from: plan.valid_from,
            valid_until: plan.valid_until,
            nonce: plan.nonce.clone(),
        });
        lifecycle.selected_network = selected_network;
        lifecycle.registration = registration;
        drop(lifecycle);
        grant.check_current(binding)?;
        Ok(grant)
    }
}

impl ParentAdmissionProvider for InstalledParentProvider {
    #[cfg(target_os = "linux")]
    fn check_canonical_startup_workspace(
        &self,
        owner: &Arc<mvm_core::host_lifetime::HostLifetimeAnchor>,
        kernel: &Arc<mvm_core::canonical_linux::PreparedCanonicalLinux>,
        pin: &mvm_core::plan::PlanArtifact,
    ) -> Result<()> {
        let grant = self.original_grant(owner.binding())?;
        ensure!(
            owner.retains_parent_owner(&grant)
                && kernel.pin() == pin
                && owner
                    .canonical_kernel()
                    .is_some_and(|original| Arc::ptr_eq(original, kernel)),
            "foreign original canonical startup"
        );
        grant.check_original_kernel(kernel, None)?;
        grant.check_portable_quote(owner, pin)
    }
    fn bind(
        &self,
        admitted: &AdmittedPlan,
        binding: &HostLaunchBinding,
    ) -> Result<Arc<dyn CoreCurrentParentPermit>> {
        let _ = (admitted, binding);
        anyhow::bail!("original bound configuration required for installed parent admission")
    }

    fn bind_config(
        &self,
        admitted: &AdmittedPlan,
        config: &mvm_core::vm_backend::VmStartConfig,
        backend: BackendKind,
    ) -> Result<Arc<dyn CoreCurrentParentPermit>> {
        let anchor = config
            .host_lifetime
            .as_ref()
            .ok_or_else(|| anyhow!("original launch anchor missing"))?;
        anchor.verify_config(config, backend)?;
        self.bind_original_admission(admitted, anchor.binding(), Some(config))
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
            ..Default::default()
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
