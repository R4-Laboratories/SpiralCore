//! Original partition custody. Profile activation awaits the accepted managed
//! bootstrap/runtime/codec/carrier model; a projection never grants resources.
use super::*;
use crate::installed_disk::{EndpointDiskInventory, EndpointDiskRequest, PreparedEndpointStore};
use crate::installed_retained::{
    RECORD_BYTES, RetainedTelemetryInventory, RetainedTelemetryScope, SIDECAR_BYTES,
};
use mvm_core::host_lifetime::{
    EndpointPartitionGuard, EndpointPartitionLimits, EndpointPartitionProjection,
    EndpointStoreLocator, PreparedEndpointPartition,
};

pub(super) struct QualifiedEndpointProfile {
    limits: EndpointPartitionLimits,
}
impl QualifiedEndpointProfile {
    #[cfg(test)]
    pub(super) fn fixture(limits: EndpointPartitionLimits) -> Self {
        Self { limits }
    }
    // No production constructor until the selected managed model is accepted.
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
            self.limits.workers > 0 && self.limits.max_streams > 0,
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
    // Explicit trusted configuration only; called before publishing the parent.
    // The default registry retains its original 4KiB footprint. This is not a
    // public quota/profile constructor or a native-support switch.
    #[cfg(test)]
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
            state.endpoint = EndpointIssuance::Uncertain;
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
        };
        let reservation = parent.pool.reserve(tenant_tag(&self.tenant), claim)?;
        let serial = reservation.serial();
        attempt.resources.as_mut().unwrap().endpoint = Some(reservation);
        let retained = inputs.retained.reserve_for_endpoint(
            tenant_tag(&self.tenant),
            session,
            binding.grant_serial,
        )?;
        attempt.resources.as_mut().unwrap().retained = Some((inputs.retained.clone(), retained));
        let store = inputs.inventory.prepare(EndpointDiskRequest {
            tenant: tenant_tag(&self.tenant),
            session,
            parent_serial: binding.grant_serial,
            cache_bytes: inputs
                .profile
                .limits
                .cache_disk_bytes
                .checked_sub(RECORD_BYTES + SIDECAR_BYTES)
                .ok_or_else(|| anyhow!("original retained disk subdivision missing"))?,
            staging_bytes: inputs.profile.limits.staging_disk_bytes,
        })?;
        attempt.resources.as_mut().unwrap().store = Some(store);
        let locator = attempt
            .resources
            .as_ref()
            .unwrap()
            .store
            .as_ref()
            .unwrap()
            .binding();
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
            projection: projection.clone(),
            resources: Mutex::new(attempt.resources.take().unwrap()),
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
        let grant = self
            .original
            .upgrade()
            .ok_or_else(|| anyhow!("original grant unavailable"))?;
        let registration = grant
            .lifecycle
            .lock()
            .map_err(|_| anyhow!("parent lifecycle poisoned"))?
            .registration
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("original registration missing"))?;
        registration.check_retained_state(state, directory)
    }
    fn matches_receipt(self: &Arc<Self>, receipt: &mvm_client::EndpointRetirement) -> bool {
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
    pub(super) fn cleanup_outcome_endpoint(
        &mut self,
        outcome: &mvm_client::LaunchOutcome,
    ) -> Result<()> {
        outcome.with_owned_retirement(|receipt| self.require_original_retirement(receipt))??;
        if self.endpoint_retirement.is_none() {
            self.endpoint_retirement = outcome.take_endpoint_retirement()?;
        }
        self.cleanup_endpoint()
    }
    pub(super) fn cleanup_pending_endpoint(
        &mut self,
        receipt: &mut mvm_client::OwnedRetirement,
    ) -> Result<()> {
        self.require_original_retirement(receipt)?;
        if self.endpoint_retirement.is_none() {
            self.endpoint_retirement = receipt.take_endpoint_retirement();
        }
        self.cleanup_endpoint()
    }
}
