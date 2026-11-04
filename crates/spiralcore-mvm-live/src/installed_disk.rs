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

#[cfg(all(test, feature = "canonical-host"))]
mod retirement_tests {
    use super::*;
    use crate::installed_owner::{InstallationKey, InstallationOwner};
    use mvm_core::{
        host_lifetime::{
            EndpointPartitionLimits, EndpointPartitionProjection, EndpointStoreLocator,
            HostLaunchBinding,
        },
        vm_backend::BackendKind,
    };
    use std::{
        fs,
        os::unix::fs::DirBuilderExt,
        sync::atomic::{AtomicU64, Ordering},
    };

    #[test]
    fn retirement_projection_rejects_foreign_store_scope_without_mutating_the_real_owner() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "spiral-retirement-binding-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let config = crate::installed_supervisor::InstalledConfig::canonical_native_profile();
        let key = InstallationKey {
            installation: [51; 32],
            profile: config.profile_hash(),
        };
        let owner = InstallationOwner::initialize(&root.join("installation"), key).unwrap();
        let mut controller = InstalledSupervisor::start(owner, config).unwrap();
        let inventory = controller.endpoint_disk_inventory().unwrap();
        let store = inventory
            .prepare(EndpointDiskRequest {
                tenant: [52; 32],
                session: [53; 32],
                parent_serial: 7,
                cache_bytes: 65536,
                staging_bytes: 65536,
            })
            .unwrap();
        let actual = store.binding();
        // Data matching alone does not construct a retirement receipt or grant.
        let projection = EndpointPartitionProjection {
            version: 1,
            binding: HostLaunchBinding {
                installation: actual.key.installation,
                profile: actual.key.profile,
                generation: actual.generation,
                grant_serial: actual.parent_serial,
                vm_name: "binding-fixture".into(),
                backend: if cfg!(target_os = "linux") {
                    BackendKind::Firecracker
                } else {
                    BackendKind::Hvf
                },
                cpus: 1,
                memory_mib: 1,
            },
            admitted_plan_id: "not-an-authority-constructor".into(),
            session: actual.session,
            partition_serial: 99,
            profile: 2,
            store: Some(EndpointStoreLocator {
                version: 1,
                path: actual.path.to_owned(),
                device: actual.device,
                inode: actual.inode,
                reservation_serial: actual.reservation_serial,
                tenant: actual.tenant,
                session: actual.session,
            }),
            limits: EndpointPartitionLimits {
                host_plaintext_bytes: 0,
                host_codec_bytes: 1,
                host_carrier_bytes: 0,
                host_metadata_bytes: 1,
                host_stack_bytes: 0,
                cache_disk_bytes: crate::installed_retained::RECORD_BYTES
                    + crate::installed_retained::SIDECAR_BYTES
                    + 65536,
                staging_disk_bytes: 65536,
                workers: 1,
                max_streams: 1,
            },
        };
        let before = controller.managed_resources().snapshot().unwrap();
        assert!(projection_matches_store(&projection, &actual));
        for field in 0..17 {
            let mut foreign = projection.clone();
            match field {
                0 => foreign.binding.installation[0] ^= 1,
                1 => foreign.binding.profile[0] ^= 1,
                2 => foreign.binding.generation += 1,
                3 => foreign.binding.grant_serial += 1,
                4 => foreign.session[0] ^= 1,
                5 => foreign.profile = 1,
                6 => foreign.store = None,
                7 => foreign.store.as_mut().unwrap().version = 2,
                8 => foreign.store.as_mut().unwrap().path = root.join("foreign"),
                9 => foreign.store.as_mut().unwrap().device ^= 1,
                10 => foreign.store.as_mut().unwrap().inode ^= 1,
                11 => foreign.store.as_mut().unwrap().reservation_serial += 1,
                12 => foreign.store.as_mut().unwrap().tenant[0] ^= 1,
                13 => foreign.store.as_mut().unwrap().session[0] ^= 1,
                14 => foreign.limits.cache_disk_bytes += 1,
                15 => foreign.limits.staging_disk_bytes += 1,
                _ => foreign.version = 2,
            }
            assert!(
                !projection_matches_store(&foreign, &actual),
                "foreign field {field}"
            );
        }
        let after = controller.managed_resources().snapshot().unwrap();
        assert_eq!(before.usage, after.usage);
        assert!(store.path().is_dir());
        store.cancel_unused().unwrap();
        drop(inventory);
        controller.close_admission();
        controller.try_finish().unwrap();
        drop(controller);
        fs::remove_dir_all(root).unwrap();
    }
}
