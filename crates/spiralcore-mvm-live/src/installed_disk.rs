//! Held parent credits paired with the original owner's durable disk inventory.
pub use crate::installed_owner::endpoint_disks::EndpointDiskBinding;
use crate::installed_owner::endpoint_disks::EndpointDiskRequest as OwnedDiskRequest;
use crate::{
    installed_owner::endpoint_disks::DiskStore,
    installed_resources::{ManagedClaim, ManagedReservation, ManagedResourceBudget},
    installed_supervisor::{InstalledAuthority, InstalledSupervisor, SupervisorError},
};
use std::{path::Path, sync::Arc};
type Result<T> = std::result::Result<T, SupervisorError>;

/// Bounded disk capacities for one session of the funded guest.
#[derive(Clone, Copy, Debug)]
pub struct EndpointDiskRequest {
    pub tenant: [u8; 32],
    pub session: [u8; 32],
    pub cache_bytes: u64,
    pub staging_bytes: u64,
}

struct AttemptReservation {
    reservation: Option<(ManagedReservation, ManagedReservation)>,
    uncertain: bool,
}
impl Drop for AttemptReservation {
    fn drop(&mut self) {
        if self.uncertain
            && let Some(reservation) = self.reservation.take()
        {
            std::mem::forget(reservation);
        }
    }
}

pub struct EndpointDiskInventory {
    authority: Arc<InstalledAuthority>,
    budget: ManagedResourceBudget,
    _metadata: Arc<ManagedReservation>,
}
impl EndpointDiskInventory {
    pub(crate) fn new(supervisor: &InstalledSupervisor) -> Result<Self> {
        supervisor.require_canonical_profile()?;
        let budget = supervisor.managed_resources().clone();
        let metadata = budget.reserve(
            [0xd1; 32],
            ManagedClaim {
                host_metadata_bytes: 512 << 10,
                ..Default::default()
            },
        )?;
        Ok(Self {
            authority: supervisor.disk_authority(),
            budget,
            _metadata: Arc::new(metadata),
        })
    }
    pub fn prepare(
        &self,
        parent: &ManagedReservation,
        request: EndpointDiskRequest,
    ) -> Result<PreparedEndpointStore> {
        let request = OwnedDiskRequest {
            parent_serial: self.budget.original_vm_serial(parent, request.tenant)?,
            tenant: request.tenant,
            session: request.session,
            cache_bytes: request.cache_bytes,
            staging_bytes: request.staging_bytes,
        };
        let reservation = self.budget.reserve(
            request.tenant,
            ManagedClaim {
                host_metadata_bytes: 4096,
                cache_disk_bytes: request.cache_bytes,
                staging_disk_bytes: request.staging_bytes,
                ..Default::default()
            },
        )?;
        let serial = reservation.serial();
        let mut attempt = AttemptReservation {
            reservation: Some((parent.clone(), reservation)),
            uncertain: true,
        };
        match self
            .authority
            .with_disk_owner(|owner, epoch| Ok(owner.prepare_endpoint_disk(epoch, serial, request)))
        {
            Ok(Ok(store)) => Ok(PreparedEndpointStore {
                authority: self.authority.clone(),
                _metadata: self._metadata.clone(),
                store: Some(store),
                reservation: attempt.reservation.take(),
            }),
            Ok(Err(failure)) => {
                // Only the original private preflight phase can prove this
                // invocation made no effects. Erased errors/unwind stay armed.
                attempt.uncertain = failure.may_have_effects;
                Err(failure.error.into())
            }
            Err(error) => Err(error),
        }
    }
}
/// Unhanded disposable store. Exposing a path is not a child-exit proof.
pub struct PreparedEndpointStore {
    authority: Arc<InstalledAuthority>,
    _metadata: Arc<ManagedReservation>,
    store: Option<DiskStore>,
    reservation: Option<(ManagedReservation, ManagedReservation)>,
}
impl PreparedEndpointStore {
    pub fn binding(&self) -> EndpointDiskBinding<'_> {
        self.store.as_ref().expect("owned prepared store").binding()
    }
    pub fn path(&self) -> &Path {
        &self.store.as_ref().expect("owned prepared store").path
    }
    /// Reopen the original namespace relative to its held installation descriptor.
    pub fn check_current(&self) -> Result<()> {
        let (parent, disk) = self.reservation.as_ref().ok_or(SupervisorError::State)?;
        parent.check_current()?;
        disk.check_current()?;
        self.authority.with_disk_owner(|owner, _| {
            owner.check_endpoint_disk(
                self.store
                    .as_ref()
                    .ok_or(crate::installed_owner::OwnerError::Fenced)?,
            )
        })
    }

    pub fn cancel_unused(mut self) -> Result<()> {
        self.authority.with_disk_owner(|owner, _| {
            owner.cancel_unused_endpoint_disk(self.store.as_ref().expect("owned prepared store"))
        })?;
        self.store.take();
        self.reservation.take();
        Ok(())
    }
    /// The resident retains the sealed original receipt across the whole I/O
    /// barrier. Admission may be closed; cleanup uses identity, not new authority.
    #[cfg(feature = "canonical-host")]
    pub(crate) fn retire_with_endpoint_receipt(
        &mut self,
        receipt: &mvm_client::EndpointRetirement,
        guard: &Arc<dyn mvm_core::host_lifetime::EndpointPartitionGuard>,
        expected: &mvm_core::host_lifetime::EndpointPartitionProjection,
    ) -> Result<()> {
        if !receipt.retains_partition_guard(guard)
            || receipt.projection() != expected
            || !projection_matches_store(expected, &self.binding())
        {
            return Err(SupervisorError::Profile);
        }
        let store = self.store.as_mut().ok_or(SupervisorError::State)?;
        self.authority
            .with_disk_owner(|owner, _| owner.retire_endpoint_disk(store))?;
        // Keep the completed original store identity until this owner drops.
        // Any failed I/O above leaves both the store and this SAME credit held.
        self.reservation.take();
        Ok(())
    }
}

#[cfg(feature = "canonical-host")]
fn projection_matches_store(
    projection: &mvm_core::host_lifetime::EndpointPartitionProjection,
    actual: &EndpointDiskBinding<'_>,
) -> bool {
    let Some(store) = &projection.store else {
        return false;
    };
    projection.version == 1
        && projection.profile == 2
        && projection.binding.installation == actual.key.installation
        && projection.binding.profile == actual.key.profile
        && projection.binding.generation == actual.generation
        && projection.binding.grant_serial == actual.parent_serial
        && projection.session == actual.session
        && store.version == 1
        && store.path == actual.path
        && store.device == actual.device
        && store.inode == actual.inode
        && store.reservation_serial == actual.reservation_serial
        && store.tenant == actual.tenant
        && store.session == actual.session
        && projection.limits.cache_disk_bytes.checked_sub(
            crate::installed_retained::RECORD_BYTES + crate::installed_retained::SIDECAR_BYTES,
        ) == Some(actual.cache_bytes)
        && projection.limits.staging_disk_bytes == actual.staging_bytes
}
impl Drop for PreparedEndpointStore {
    fn drop(&mut self) {
        if let Some(reservation) = self.reservation.take() {
            // Missing explicit cleanup cannot publish a verified disk refund.
            std::mem::forget(reservation);
        }
    }
}
