//! Held parent credits paired with the original owner's durable disk inventory.
pub use crate::installed_owner::endpoint_disks::{EndpointDiskBinding, EndpointDiskRequest};
use crate::{
    installed_owner::endpoint_disks::DiskStore,
    installed_resources::{ManagedClaim, ManagedReservation, ManagedResourceBudget},
    installed_supervisor::{InstalledAuthority, InstalledSupervisor, SupervisorError},
};
use std::{path::Path, sync::Arc};
type Result<T> = std::result::Result<T, SupervisorError>;

struct AttemptReservation {
    reservation: Option<ManagedReservation>,
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
    pub fn prepare(&self, request: EndpointDiskRequest) -> Result<PreparedEndpointStore> {
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
            reservation: Some(reservation),
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
    reservation: Option<ManagedReservation>,
}
impl PreparedEndpointStore {
    pub fn binding(&self) -> EndpointDiskBinding<'_> {
        self.store.as_ref().expect("owned prepared store").binding()
    }
    pub fn path(&self) -> &Path {
        &self.store.as_ref().expect("owned prepared store").path
    }
    pub fn cancel_unused(mut self) -> Result<()> {
        self.authority.with_disk_owner(|owner, _| {
            owner.cancel_unused_endpoint_disk(self.store.as_ref().expect("owned prepared store"))
        })?;
        self.store.take();
        self.reservation.take();
        Ok(())
    }
}
impl Drop for PreparedEndpointStore {
    fn drop(&mut self) {
        if let Some(reservation) = self.reservation.take() {
            // Missing explicit cleanup cannot publish a verified disk refund.
            std::mem::forget(reservation);
        }
    }
}
