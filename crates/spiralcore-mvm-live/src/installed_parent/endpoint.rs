//! Installed endpoint partition custody. Limits describe claims on the original shared pool.
use super::*;
use crate::installed_disk::{EndpointDiskInventory, EndpointDiskRequest, PreparedEndpointStore};
use crate::installed_retained::{
    RECORD_BYTES, RetainedTelemetryInventory, RetainedTelemetryScope, SIDECAR_BYTES,
};
use mvm_core::host_lifetime::{
    EndpointPartitionGuard, EndpointPartitionLimits, EndpointPartitionProjection,
    EndpointStoreLocator, PreparedEndpointPartition,
};

struct StoreFunding {
    file_descriptors: u64,
    codec_bytes: u64,
}

impl StoreFunding {
    fn for_serial_endpoint() -> Result<Self> {
        let quote =
            mvm_hostd::supervisor::flowmux::SpiralCorePayloadFactory::owned_store_requirements()?;
        // One shared parent and one active store in the installed serial profile.
        let file_descriptors = u64::from(quote.shared_parent_fds) + u64::from(quote.fds_per_store);
        let codec_bytes = spiralcore_transfer::resource_profile::ResourceProfile::SmallV1
            .operation_charge()
            .map_err(|error| anyhow!("{error:?}"))?
            .checked_add(quote.workspace_bytes)
            .ok_or_else(|| anyhow!("endpoint store workspace requirement overflow"))?;
        Ok(Self {
            file_descriptors,
            codec_bytes,
        })
    }
}

pub(super) struct QualifiedEndpointProfile {
    limits: EndpointPartitionLimits,
}
impl QualifiedEndpointProfile {
    #[cfg(test)]
    pub(super) fn fixture(limits: EndpointPartitionLimits) -> Self {
        Self { limits }
    }
    #[cfg(not(target_os = "linux"))]
    fn from_declared(_: EndpointPartitionLimits) -> Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "installed endpoint process address-space enforcement requires Linux",
        )
        .into())
    }
    #[cfg(target_os = "linux")]
    fn from_declared(limits: EndpointPartitionLimits) -> Result<Self> {
        // The local supervisor owns these configured ceilings. Only the returned
        // reservation establishes authority; the canonical child independently
        // checks its complete effective process limit and each sub-allocation.
        let runtime = mvm_hostd::installed_endpoint::owned_network_runtime_requirements()?;
        ensure!(
            runtime.process_address_space_enforced,
            "installed native process memory bound unavailable"
        );
        let stacks = runtime
            .controller
            .main_stack_envelope_bytes
            .checked_add(runtime.session_stack_bytes)
            .ok_or_else(|| anyhow!("endpoint stack requirement overflow"))?;
        let metadata = runtime
            .controller
            .controller_metadata_bytes
            .checked_add(mvm_vmm::host::network_endpoint_spawn::INSTALLED_BOOTSTRAP_MIN_BYTES)
            .ok_or_else(|| anyhow!("endpoint metadata requirement overflow"))?;
        let profile = spiralcore_transfer::resource_profile::ResourceProfile::SmallV1;
        let disk = profile
            .receiver_limits()
            .cache_bytes
            .checked_mul(2)
            .and_then(|n| n.checked_add(1 << 20))
            .ok_or_else(|| anyhow!("endpoint store requirement overflow"))?;
        let disposable = limits
            .cache_disk_bytes
            .checked_sub(RECORD_BYTES + SIDECAR_BYTES)
            .and_then(|n| n.checked_add(limits.staging_disk_bytes))
            .ok_or_else(|| anyhow!("endpoint retained or disposable disk capacity missing"))?;
        // The selected child services one original session serially. A wider
        // stream claim needs a different enforced runtime and component model.
        ensure!(
            limits.max_streams == 1
                && limits.workers
                    >= 1 + u32::from(runtime.controller.additional_threads)
                        + u32::from(runtime.additional_threads)
                && limits.host_stack_bytes >= stacks
                && limits.host_metadata_bytes >= metadata
                && limits.host_codec_bytes >= StoreFunding::for_serial_endpoint()?.codec_bytes
                && limits.host_carrier_bytes > 0
                && limits.host_plaintext_bytes > 0
                && disposable >= disk,
            "declared installed endpoint partition is too small"
        );
        limits.host_resident_bytes()?;
        Ok(Self { limits })
    }
    fn claim(&self, binding: &HostLaunchBinding, plan: &str) -> Result<ManagedClaim> {
        let metadata = std::mem::size_of::<RootEndpointOwner>()
            + 2 * std::mem::size_of::<usize>()
            + std::mem::align_of::<RootEndpointOwner>()
            + 2 * std::mem::size_of::<EndpointPartitionProjection>()
            + 4 * (binding.vm_name.len() + plan.len())
            + 2 * 4096;
        ensure!(
            self.limits.host_metadata_bytes >= u64::try_from(metadata)?,
            "endpoint owner metadata model insufficient"
        );
        ensure!(
            self.limits.workers > 0 && self.limits.max_streams == 1,
            "endpoint worker model missing"
        );
        // The projection's disk allowance includes the mandatory collector;
        // spare host quota cannot enlarge this original endpoint allowance.
        ensure!(
            self.limits.cache_disk_bytes >= RECORD_BYTES + SIDECAR_BYTES
                && SIDECAR_BYTES == 32 << 10
                && RECORD_BYTES == mvm_vmm::host::telemetry_provisioning::DEFAULT_RECORDS_BYTE_CAP,
            "endpoint total cache cannot contain mandatory telemetry"
        );
        self.limits.host_resident_bytes()?;
        Ok(ManagedClaim {
            host_plaintext_bytes: self.limits.host_plaintext_bytes,
            host_codec_bytes: self.limits.host_codec_bytes,
            codec_store_file_descriptors: StoreFunding::for_serial_endpoint()?.file_descriptors,
            host_carrier_bytes: self.limits.host_carrier_bytes,
            host_metadata_bytes: self.limits.host_metadata_bytes,
            host_stack_bytes: self.limits.host_stack_bytes,
            workers: u64::from(self.limits.workers),
            ..Default::default()
        })
    }
}
pub(super) struct EndpointOwnerInputs {
    inventory: EndpointDiskInventory,
    retained: Arc<RetainedTelemetryInventory>,
    profile: QualifiedEndpointProfile,
}
impl InstalledParentProvider {
    /// Select the bounded Object-TCP service before the first original VM grant.
    /// Host, tenant and device accounting remain in this supervisor's ledger.
    pub fn configure_object_tcp(
        &mut self,
        supervisor: &InstalledSupervisor,
        limits: EndpointPartitionLimits,
    ) -> Result<()> {
        self.configure_endpoints(supervisor, QualifiedEndpointProfile::from_declared(limits)?)
    }

    pub(super) fn configure_endpoints(
        &mut self,
        supervisor: &InstalledSupervisor,
        profile: QualifiedEndpointProfile,
    ) -> Result<()> {
        ensure!(
            self.shared.pool.same_pool(supervisor.managed_resources()),
            "endpoint supervisor differs from original parent"
        );
        ensure!(
            Arc::strong_count(&self.shared) == 1,
            "parent already published"
        );
        let shared = Arc::get_mut(&mut self.shared).ok_or_else(|| anyhow!("parent shared"))?;
        ensure!(
            shared
                .registry
                .get_mut()
                .map_err(|_| anyhow!("parent registry poisoned"))?
                .slots
                .iter()
                .all(|slot| slot.upgrade().is_none()),
            "VM grants already issued"
        );
        ensure!(
            shared.endpoint_inputs.is_none(),
            "endpoint profile already installed"
        );
        shared.endpoint_inputs = Some(EndpointOwnerInputs {
            inventory: supervisor.endpoint_disk_inventory()?,
            retained: supervisor.retained_telemetry_inventory()?,
            profile,
        });
        Ok(())
    }
}

struct Resources {
    vm: ManagedReservation,
    endpoint: Option<ManagedReservation>,
    store: Option<PreparedEndpointStore>,
    retained: Option<(Arc<RetainedTelemetryInventory>, RetainedTelemetryScope)>,
    cleaned: bool,
}
/// The guard never strongly owns its grant. Receipts live in ParentLaunchOwner,
/// outside this object, because receipts themselves retain this guard.
pub(super) struct RootEndpointOwner {
    original: Weak<VmGrant>,
    projection: EndpointPartitionProjection,
    resources: Mutex<Resources>,
    identity: std::sync::OnceLock<ObservedFlowMuxGuest>,
}
impl Drop for Resources {
    fn drop(&mut self) {
        if !self.cleaned {
            std::mem::forget(self.vm.clone());
            if let Some(resource) = self.endpoint.take() {
                std::mem::forget(resource);
            }
            if let Some(store) = self.store.take() {
                std::mem::forget(store);
            }
        }
    }
}
struct Attempt<'a> {
    grant: &'a VmGrant,
    resources: Option<Resources>,
    published: bool,
    effects_possible: bool,
}
impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if !self.published {
            // Even an unwind/erased preparation error cannot reset issuance or
            // refund resources whose effects are no longer provably absent.
            let mut state = self
                .grant
                .lifecycle
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            state.endpoint = if self.effects_possible {
                EndpointIssuance::Uncertain
            } else {
                if let Some(resources) = self.resources.as_mut() {
                    resources.cleaned = true;
                }
                EndpointIssuance::NeverIssued
            };
        }
    }
}
impl VmGrant {
    pub(super) fn prepare_original_endpoint(
        &self,
        binding: &HostLaunchBinding,
    ) -> Result<PreparedEndpointPartition> {
        self.check_current(binding)?;
        let parent = self
            .parent
            .upgrade()
            .ok_or_else(|| anyhow!("original parent unavailable"))?;
        let inputs = parent
            .endpoint_inputs
            .as_ref()
            .ok_or_else(|| anyhow!("installed endpoint resource model not qualified"))?;
        let (session, claim) = {
            let mut state = self
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            state.check_launch_open()?;
            ensure!(
                state.endpoint == EndpointIssuance::NeverIssued,
                "endpoint already issued"
            );
            let plan = state
                .admitted_plan
                .as_ref()
                .ok_or_else(|| anyhow!("canonical admission missing"))?;
            ensure!(
                state
                    .selected_network
                    .as_ref()
                    .is_some_and(|network| network.plan_id.0 == *plan),
                "selected network policy absent from original admitted plan"
            );
            let session = state
                .admitted_session
                .ok_or_else(|| anyhow!("canonical endpoint session missing"))?;
            let claim = inputs.profile.claim(binding, plan)?;
            // Metadata is reserved below before copying the plan or constructing
            // any endpoint owner/projection. No effects have happened yet.
            state.endpoint = EndpointIssuance::Preparing;
            (session, claim)
        };
        let mut attempt = Attempt {
            grant: self,
            resources: Some(Resources {
                vm: self.reservation.clone(),
                endpoint: None,
                store: None,
                retained: None,
                cleaned: false,
            }),
            published: false,
            effects_possible: false,
        };
        let reservation = parent.pool.reserve(tenant_tag(&self.tenant), claim)?;
        let serial = reservation.serial();
        let resources = attempt
            .resources
            .as_mut()
            .ok_or_else(|| anyhow!("endpoint preparation lost its resource guard"))?;
        resources.endpoint = Some(reservation);
        let retained = inputs.retained.reserve_for_endpoint(
            tenant_tag(&self.tenant),
            session,
            binding.grant_serial,
        )?;
        attempt.effects_possible = true;
        resources.retained = Some((inputs.retained.clone(), retained));
        let store = inputs.inventory.prepare(
            &self.reservation,
            EndpointDiskRequest {
                tenant: tenant_tag(&self.tenant),
                session,
                cache_bytes: inputs
                    .profile
                    .limits
                    .cache_disk_bytes
                    .checked_sub(RECORD_BYTES + SIDECAR_BYTES)
                    .ok_or_else(|| anyhow!("original retained disk subdivision missing"))?,
                staging_bytes: inputs.profile.limits.staging_disk_bytes,
            },
        )?;
        let locator = resources.store.insert(store).binding();
        let plan = self
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .admitted_plan
            .as_ref()
            .ok_or_else(|| anyhow!("canonical admission missing"))?
            .clone();
        let projection = EndpointPartitionProjection {
            version: 1,
            binding: binding.clone(),
            admitted_plan_id: plan,
            session,
            partition_serial: serial,
            profile: 2,
            limits: inputs.profile.limits,
            store: Some(EndpointStoreLocator {
                version: 1,
                path: locator.path.to_owned(),
                device: locator.device,
                inode: locator.inode,
                reservation_serial: locator.reservation_serial,
                tenant: locator.tenant,
                session: locator.session,
            }),
        };
        let owner = Arc::new(RootEndpointOwner {
            original: self.original.clone(),
            identity: Default::default(),
            projection: projection.clone(),
            resources: Mutex::new(
                attempt
                    .resources
                    .take()
                    .ok_or_else(|| anyhow!("endpoint preparation lost its resource guard"))?,
            ),
        });
        {
            let mut state = self
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            ensure!(
                !state.retired && state.endpoint == EndpointIssuance::Preparing,
                "original endpoint issuance interrupted"
            );
            state.endpoint_owner = Some(owner.clone());
            state.endpoint = EndpointIssuance::Issued;
        }
        attempt.published = true;
        // Core immediately calls back into the guard: no lifecycle lock here.
        PreparedEndpointPartition::new(projection, owner)
    }
}
impl EndpointPartitionGuard for RootEndpointOwner {
    fn check_owned_payload_store_resources(
        &self,
        projection: &EndpointPartitionProjection,
    ) -> Result<()> {
        self.check_current(projection)?;
        let funding = StoreFunding::for_serial_endpoint()?;
        {
            let resources = self
                .resources
                .lock()
                .map_err(|_| anyhow!("endpoint resources poisoned"))?;
            let original = resources
                .endpoint
                .as_ref()
                .ok_or_else(|| anyhow!("endpoint credit missing"))?;
            ensure!(
                original.serial() == projection.partition_serial,
                "endpoint reservation serial differs"
            );
            original.check_codec_store_funding(funding.file_descriptors, funding.codec_bytes)?;
        }
        self.check_current(projection)
    }

    fn observe_flowmux_guest(
        &self,
        projection: &EndpointPartitionProjection,
        guest: [u8; 32],
        host: [u8; 32],
    ) -> Result<()> {
        self.check_current(projection)?;
        let registration = self.original_registration()?;
        registration.check_directories(None)?;
        ensure!(
            !ed25519_dalek::VerifyingKey::from_bytes(&guest)?.is_weak()
                && !ed25519_dalek::VerifyingKey::from_bytes(&host)?.is_weak(),
            "invalid original FlowMux guest verifying key"
        );
        let observed = ObservedFlowMuxGuest {
            session: projection.session,
            guest,
            host,
        };
        ensure!(
            self.identity.get_or_init(|| observed) == &observed,
            "original FlowMux guest key changed"
        );
        registration.check_directories(None)?;
        self.check_current(projection)
    }

    fn prepare_retained_telemetry(
        &self,
        projection: &EndpointPartitionProjection,
        state: &std::path::Path,
        directory: &std::fs::File,
    ) -> Result<()> {
        self.check_original_retained_state(projection, state, directory)?;
        {
            let resources = self
                .resources
                .lock()
                .map_err(|_| anyhow!("endpoint resources poisoned"))?;
            let (inventory, scope) = resources
                .retained
                .as_ref()
                .ok_or_else(|| anyhow!("original retained scope missing"))?;
            inventory.prepare_state(scope, state, directory)?;
        }
        self.check_current(projection)
    }
    fn bind_retained_telemetry_files(
        &self,
        projection: &EndpointPartitionProjection,
        state: &std::path::Path,
        directory: &std::fs::File,
        records: &std::fs::File,
        wrapped_key: &std::fs::File,
    ) -> Result<()> {
        self.check_original_retained_state(projection, state, directory)?;
        {
            let resources = self
                .resources
                .lock()
                .map_err(|_| anyhow!("endpoint resources poisoned"))?;
            let (inventory, scope) = resources
                .retained
                .as_ref()
                .ok_or_else(|| anyhow!("original retained scope missing"))?;
            inventory.bind_files(scope, state, directory, records, wrapped_key)?;
        }
        self.check_current(projection)
    }
    fn check_current(&self, projection: &EndpointPartitionProjection) -> Result<()> {
        ensure!(
            projection == &self.projection,
            "endpoint projection substituted"
        );
        let grant = self
            .original
            .upgrade()
            .ok_or_else(|| anyhow!("original grant unavailable"))?;
        grant.check_current(&projection.binding)?;
        {
            let state = grant
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            ensure!(
                state.endpoint == EndpointIssuance::Issued
                    && state
                        .endpoint_owner
                        .as_ref()
                        .is_some_and(|owner| std::ptr::eq(owner.as_ref(), self)),
                "original endpoint owner differs"
            );
        }
        let resources = self
            .resources
            .lock()
            .map_err(|_| anyhow!("endpoint resources poisoned"))?;
        ensure!(!resources.cleaned, "endpoint already cleaned");
        resources.vm.check_current()?;
        resources
            .store
            .as_ref()
            .ok_or_else(|| anyhow!("original endpoint store unavailable"))?
            .check_current()?;
        resources
            .endpoint
            .as_ref()
            .ok_or_else(|| anyhow!("endpoint credit missing"))?
            .check_current()?;
        Ok(())
    }
}
impl RootEndpointOwner {
    fn check_original_retained_state(
        &self,
        projection: &EndpointPartitionProjection,
        state: &std::path::Path,
        directory: &std::fs::File,
    ) -> Result<()> {
        self.check_current(projection)?;
        self.original_registration()?
            .check_retained_state(state, directory)
    }

    fn original_registration(&self) -> Result<Arc<host_agent::RegistrationBinding>> {
        let grant = self
            .original
            .upgrade()
            .ok_or_else(|| anyhow!("original grant unavailable"))?;
        grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .registration
            .clone()
            .ok_or_else(|| anyhow!("original registration missing"))
    }

    #[cfg(any(target_os = "linux", test))]
    pub(super) fn observed_guest(&self, plan_id: &str) -> Result<ObservedFlowMuxGuest> {
        ensure!(
            plan_id == self.projection.admitted_plan_id,
            "different admitted plan"
        );
        self.check_current(&self.projection)?;
        self.original_registration()?.check_directories(None)?;
        self.identity
            .get()
            .copied()
            .ok_or_else(|| anyhow!("original FlowMux keys not observed"))
    }
    pub(super) fn matches_receipt(
        self: &Arc<Self>,
        receipt: &mvm_client::EndpointRetirement,
    ) -> bool {
        let guard: Arc<dyn EndpointPartitionGuard> = self.clone();
        receipt.projection() == &self.projection && receipt.retains_partition_guard(&guard)
    }
    fn cleanup_retired(self: &Arc<Self>, receipt: &mvm_client::EndpointRetirement) -> Result<()> {
        ensure!(
            self.matches_receipt(receipt),
            "endpoint retirement owner differs"
        );
        let grant = self
            .original
            .upgrade()
            .ok_or_else(|| anyhow!("original grant unavailable"))?;
        let guard: Arc<dyn EndpointPartitionGuard> = self.clone();
        {
            let mut resources = self
                .resources
                .lock()
                .map_err(|_| anyhow!("endpoint resources poisoned"))?;
            resources.vm.check_retained_identity()?;
            resources
                .endpoint
                .as_ref()
                .ok_or_else(|| anyhow!("endpoint credit missing"))?
                .check_retained_identity()?;
            if !resources.cleaned {
                let (inventory, scope) = resources
                    .retained
                    .as_ref()
                    .ok_or_else(|| anyhow!("original retained scope missing"))?;
                inventory.seal_after_endpoint(scope, receipt, &guard, &self.projection)?;
                resources
                    .store
                    .as_mut()
                    .ok_or_else(|| anyhow!("endpoint store missing"))?
                    .retire_with_endpoint_receipt(receipt, &guard, &self.projection)?;
                resources.cleaned = true;
            }
        }
        // Disk I/O and resource mutex are released before grant state changes.
        let mut state = grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
        ensure!(
            state
                .endpoint_owner
                .as_ref()
                .is_some_and(|owner| Arc::ptr_eq(owner, self)),
            "original endpoint owner differs"
        );
        state.endpoint = EndpointIssuance::CleanupComplete;
        Ok(())
    }
}
impl ParentLaunchOwner {
    pub(super) fn cleanup_endpoint(&mut self) -> Result<()> {
        let (phase, owner) = {
            let state = self
                .grant
                .lifecycle
                .lock()
                .map_err(|_| anyhow!("parent lifecycle poisoned"))?;
            (state.endpoint, state.endpoint_owner.clone())
        };
        if phase == EndpointIssuance::NeverIssued {
            ensure!(
                self.endpoint_retirement.is_none(),
                "unexpected endpoint receipt"
            );
            return Ok(());
        }
        let owner = owner.ok_or_else(|| anyhow!("original endpoint owner unavailable"))?;
        let receipt = self
            .endpoint_retirement
            .as_ref()
            .ok_or_else(|| anyhow!("original endpoint receipt missing"))?;
        owner.cleanup_retired(receipt)
    }
}
