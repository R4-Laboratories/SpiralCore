//! Supervisor-owned retained ciphertext debt. No VM or deletion authority.
use crate::{
    installed_owner::retained_telemetry::{MAX_ENTRIES, RetainedRecord},
    installed_resources::{ManagedClaim, ManagedReservation, ManagedResourceBudget},
    installed_supervisor::{InstalledAuthority, InstalledSupervisor, SupervisorError},
};
use mvm_core::runtime_state_protection::{
    RuntimeStateProtection, TELEMETRY_RECORDS_FILE, TELEMETRY_RECORDS_KEY_FILE,
};
use std::{
    fs::File,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
    sync::{Arc, Mutex, Weak},
};

type Result<T> = std::result::Result<T, SupervisorError>;
pub(crate) const RECORD_BYTES: u64 = 8 << 20;
// Two <=8KiB catalog records, two <=4KiB status files, two <=1KiB
// wrapped keys, and one <=1KiB protection marker fit below this ceiling.
// The owned status writer still needs its explicit encoded-size gate before
// profile activation. These are logical bytes, not filesystem block/RSS proof.
pub(crate) use mvm_core::runtime_state_protection::INSTALLED_TELEMETRY_SIDECAR_BYTES as SIDECAR_BYTES;
pub(crate) const FIXED_DISK_BYTES: u64 = 8192;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Reserved,
    Protecting,
    Protected,
    Binding,
    Bound,
    Sealing,
    Sealed,
}
struct Entry {
    tenant: [u8; 32],
    session: [u8; 32],
    parent: u64,
    reservation: ManagedReservation,
    phase: Phase,
    record: Option<RetainedRecord>,
    directory: Option<File>,
    files: Option<(File, File)>,
}
pub(crate) struct RetainedTelemetryInventory {
    authority: Arc<InstalledAuthority>,
    budget: ManagedResourceBudget,
    metadata: ManagedReservation,
    entries: Mutex<[Option<Entry>; MAX_ENTRIES]>,
}
/// Original in-process lookup only; it cannot be reconstructed from a catalog.
pub(crate) struct RetainedTelemetryScope {
    original: Weak<RetainedTelemetryInventory>,
    slot: usize,
    serial: u64,
}
impl RetainedTelemetryInventory {
    pub(crate) fn new(supervisor: &InstalledSupervisor) -> Result<Arc<Self>> {
        supervisor.require_canonical_profile()?;
        if RECORD_BYTES != mvm_vmm::host::telemetry_provisioning::DEFAULT_RECORDS_BYTE_CAP
            || SIDECAR_BYTES != 32 << 10
        {
            return Err(SupervisorError::Profile);
        }
        let budget = supervisor.managed_resources().clone();
        let metadata = budget.reserve(
            [0xd2; 32],
            ManagedClaim {
                host_metadata_bytes: 1 << 20,
                cache_disk_bytes: FIXED_DISK_BYTES,
                ..Default::default()
            },
        )?;
        Ok(Arc::new(Self {
            authority: supervisor.disk_authority(),
            budget,
            metadata,
            entries: Mutex::new(std::array::from_fn(|_| None)),
        }))
    }
    #[cfg(feature = "canonical-host")]
    pub(crate) fn original_sealed(
        &self,
        grant: u64,
        session: [u8; 32],
    ) -> Result<[ManagedReservation; 2]> {
        let entries = self.entries.lock().map_err(|_| SupervisorError::State)?;
        let mut values = entries.iter().flatten();
        let original = values.next().ok_or(SupervisorError::State)?;
        if values.next().is_some()
            || original.phase != Phase::Sealed
            || original.parent != grant
            || original.session != session
            || original.record.is_none()
        {
            return Err(SupervisorError::State);
        }
        Ok([self.metadata.clone(), original.reservation.clone()])
    }
    pub(crate) fn is_empty(&self) -> Result<bool> {
        Ok(self
            .entries
            .lock()
            .map_err(|_| SupervisorError::State)?
            .iter()
            .all(Option::is_none))
    }
    pub(crate) fn reserve_for_endpoint(
        self: &Arc<Self>,
        tenant: [u8; 32],
        session: [u8; 32],
        parent: u64,
    ) -> Result<RetainedTelemetryScope> {
        if tenant == [0; 32] || session == [0; 32] || parent == 0 {
            return Err(SupervisorError::Profile);
        }
        // No inventory lock is held during original ledger/epoch checks.
        let reservation = self.budget.reserve(
            tenant,
            ManagedClaim {
                cache_disk_bytes: RECORD_BYTES + SIDECAR_BYTES,
                ..Default::default()
            },
        )?;
        let serial = reservation.serial();
        let mut entries = self.entries.lock().map_err(|_| SupervisorError::State)?;
        if entries
            .iter()
            .flatten()
            .any(|entry| entry.session == session)
        {
            return Err(SupervisorError::State);
        }
        let slot = entries
            .iter()
            .position(Option::is_none)
            .ok_or(SupervisorError::Busy)?;
        entries[slot] = Some(Entry {
            tenant,
            session,
            parent,
            reservation,
            phase: Phase::Reserved,
            record: None,
            directory: None,
            files: None,
        });
        Ok(RetainedTelemetryScope {
            original: Arc::downgrade(self),
            slot,
            serial,
        })
    }
    fn same_scope(self: &Arc<Self>, scope: &RetainedTelemetryScope) -> Result<()> {
        if !scope
            .original
            .upgrade()
            .is_some_and(|original| Arc::ptr_eq(self, &original))
        {
            return Err(SupervisorError::Profile);
        }
        self.metadata.check_current()?;
        Ok(())
    }
    fn entry<'a>(
        entries: &'a mut [Option<Entry>; MAX_ENTRIES],
        scope: &RetainedTelemetryScope,
    ) -> Result<&'a mut Entry> {
        entries
            .get_mut(scope.slot)
            .and_then(Option::as_mut)
            .filter(|entry| entry.reservation.serial() == scope.serial)
            .ok_or(SupervisorError::State)
    }
    pub(crate) fn prepare_state(
        self: &Arc<Self>,
        scope: &RetainedTelemetryScope,
        path: &Path,
        directory: &File,
    ) -> Result<()> {
        self.same_scope(scope)?;
        let parent = path.parent().ok_or(SupervisorError::Profile)?;
        let parent_file = File::options()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(parent)?;
        let protection = RuntimeStateProtection::acquire(parent, &parent_file)?;
        protection.check_state(path, directory)?;
        let meta = directory.metadata()?;
        if meta.mode() & 0o7777 != 0o700 || path.as_os_str().len() > 4096 {
            return Err(SupervisorError::Profile);
        }
        let mut entries = self.entries.lock().map_err(|_| SupervisorError::State)?;
        if entries.iter().flatten().any(|entry| {
            entry.record.as_ref().is_some_and(|record| {
                record.state == path
                    || (record.state_device, record.state_inode) == (meta.dev(), meta.ino())
            })
        }) {
            return Err(SupervisorError::State);
        }
        let held: Vec<_> = entries
            .iter()
            .flatten()
            .filter_map(|entry| entry.record.clone())
            .collect();
        let entry = Self::entry(&mut entries, scope)?;
        if entry.phase != Phase::Reserved {
            return Err(SupervisorError::State);
        }
        // Arm custody and retain the actual pin before any durable side effect.
        entry.directory = Some(directory.try_clone()?);
        entry.phase = Phase::Protecting;
        let record = RetainedRecord {
            tenant: entry.tenant,
            session: entry.session,
            parent: entry.parent,
            serial: scope.serial,
            records: RECORD_BYTES,
            sidecars: SIDECAR_BYTES,
            state: path.to_owned(),
            state_device: meta.dev(),
            state_inode: meta.ino(),
            phase: 1,
            record_id: (0, 0),
            key_id: (0, 0),
            sealed_len: 0,
            sealed_sha256: [0; 32],
        };
        entry.record = Some(record.clone());
        let prepared = self
            .authority
            .with_disk_owner(|owner, epoch| {
                owner.protect_retained_telemetry(epoch, &record, directory, &held)
            })
            .and_then(|()| protection.check_state(path, directory).map_err(Into::into));
        if let Err(error) = prepared {
            self.authority.close_admission();
            return Err(error);
        }
        entry.record.as_mut().ok_or(SupervisorError::State)?.phase = 2;
        entry.phase = Phase::Protected;
        Ok(())
    }
    pub(crate) fn bind_files(
        self: &Arc<Self>,
        scope: &RetainedTelemetryScope,
        path: &Path,
        directory: &File,
        records: &File,
        wrapped_key: &File,
    ) -> Result<()> {
        self.same_scope(scope)?;
        let parent = path.parent().ok_or(SupervisorError::Profile)?;
        let parent_file = File::options()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(parent)?;
        let protection = RuntimeStateProtection::acquire(parent, &parent_file)?;
        protection.check_state(path, directory)?;
        crate::installed_owner::retained_telemetry::require_private_retained_directory(directory)?;
        let mut entries = self.entries.lock().map_err(|_| SupervisorError::State)?;
        let held: Vec<_> = entries
            .iter()
            .flatten()
            .filter_map(|entry| entry.record.clone())
            .collect();
        let entry = Self::entry(&mut entries, scope)?;
        if entry.phase != Phase::Protected {
            return Err(SupervisorError::State);
        }
        let old = entry.record.as_ref().ok_or(SupervisorError::State)?.clone();
        let meta = directory.metadata()?;
        if old.state != path || (old.state_device, old.state_inode) != (meta.dev(), meta.ino()) {
            return Err(SupervisorError::Profile);
        }
        require_records_access(records)?;
        let record_id = original_file(directory, TELEMETRY_RECORDS_FILE, records, RECORD_BYTES)?;
        let key_id = original_file(directory, TELEMETRY_RECORDS_KEY_FILE, wrapped_key, 1024)?;
        let files = (records.try_clone()?, wrapped_key.try_clone()?);
        entry.files = Some(files);
        entry.phase = Phase::Binding;
        let mut next = old.clone();
        next.phase = 3;
        next.record_id = record_id;
        next.key_id = key_id;
        let bound = (|| -> Result<()> {
            records.sync_all()?;
            wrapped_key.sync_all()?;
            directory.sync_all()?;
            self.authority.with_disk_owner(|owner, epoch| {
                owner.update_retained_telemetry(epoch, &old, &next, &held)
            })?;
            protection.check_state(path, directory)?;
            Ok(())
        })();
        if let Err(error) = bound {
            self.authority.close_admission();
            return Err(error);
        }
        entry.record = Some(next);
        entry.phase = Phase::Bound;
        Ok(())
    }
    /// The original positive endpoint receipt is required before final sync.
    /// Sealing never deletes data or releases retained disk debt. A failed
    /// pre-binding transaction requires separate original recovery custody.
    #[cfg(feature = "canonical-host")]
    pub(crate) fn seal_after_endpoint(
        self: &Arc<Self>,
        scope: &RetainedTelemetryScope,
        receipt: &mvm_client::EndpointRetirement,
        guard: &Arc<dyn mvm_core::host_lifetime::EndpointPartitionGuard>,
        projection: &mvm_core::host_lifetime::EndpointPartitionProjection,
        original_vm: &ManagedReservation,
    ) -> Result<()> {
        use sha2::{Digest, Sha256};
        use std::os::unix::fs::FileExt;
        if !receipt.retains_partition_guard(guard)
            || receipt.projection() != projection
            || !scope
                .original
                .upgrade()
                .is_some_and(|original| Arc::ptr_eq(self, &original))
        {
            return Err(SupervisorError::Profile);
        }
        self.metadata.check_retained_identity()?;
        // Capture only a clone of the actual reserved credit, then release the
        // inventory lock before consulting its original resource ledger.
        let retained = {
            let mut entries = self.entries.lock().map_err(|_| SupervisorError::State)?;
            Self::entry(&mut entries, scope)?.reservation.clone()
        };
        if original_vm.serial() != projection.binding.grant_serial {
            return Err(SupervisorError::Profile);
        }
        retained.check_same_tenant(original_vm)?;
        let mut entries = self.entries.lock().map_err(|_| SupervisorError::State)?;
        let entry = Self::entry(&mut entries, scope)?;
        let store_matches = match (projection.profile, projection.store.as_ref()) {
            (1, None) => true,
            (2, Some(store)) => store.tenant == entry.tenant,
            _ => false,
        };
        if entry.session != projection.session
            || entry.parent != projection.binding.grant_serial
            || !store_matches
        {
            return Err(SupervisorError::Profile);
        }
        if entry.phase == Phase::Sealed {
            return Ok(());
        }
        if entry.phase != Phase::Bound {
            return Err(SupervisorError::State);
        }
        let old = entry.record.as_ref().ok_or(SupervisorError::State)?.clone();
        let directory = entry
            .directory
            .as_ref()
            .ok_or(SupervisorError::State)?
            .try_clone()?;
        // Match preparation/binding lock order: protection, then inventory,
        // then original catalog owner. No table is held while acquiring it.
        drop(entries);
        let parent = old.state.parent().ok_or(SupervisorError::Profile)?;
        let parent_file = File::options()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(parent)?;
        let protection = RuntimeStateProtection::acquire(parent, &parent_file)?;
        protection.check_state(&old.state, &directory)?;
        crate::installed_owner::retained_telemetry::require_private_retained_directory(&directory)?;
        let mut entries = self.entries.lock().map_err(|_| SupervisorError::State)?;
        let held: Vec<_> = entries
            .iter()
            .flatten()
            .filter_map(|entry| entry.record.clone())
            .collect();
        let entry = Self::entry(&mut entries, scope)?;
        if entry.phase != Phase::Bound || entry.record.as_ref() != Some(&old) {
            return Err(SupervisorError::State);
        }
        let (records, key) = entry.files.as_ref().ok_or(SupervisorError::State)?;
        require_records_access(records)?;
        if original_file(&directory, TELEMETRY_RECORDS_FILE, records, RECORD_BYTES)?
            != old.record_id
            || original_file(&directory, TELEMETRY_RECORDS_KEY_FILE, key, 1024)? != old.key_id
        {
            return Err(SupervisorError::Profile);
        }
        entry.phase = Phase::Sealing;
        records.sync_all()?;
        key.sync_all()?;
        let length = records.metadata()?.len();
        let mut offset = 0;
        let mut buffer = [0_u8; 8192];
        let mut digest = Sha256::new();
        while offset < length {
            let count = records.read_at(
                &mut buffer[..usize::try_from((length - offset).min(8192))
                    .map_err(|_| SupervisorError::State)?],
                offset,
            )?;
            if count == 0 {
                return Err(SupervisorError::State);
            }
            digest.update(&buffer[..count]);
            offset += count as u64;
        }
        if records.metadata()?.len() != length {
            return Err(SupervisorError::State);
        }
        directory.sync_all()?;
        let mut next = old.clone();
        next.phase = 4;
        next.sealed_len = length;
        next.sealed_sha256 = digest.finalize().into();
        self.authority.with_disk_owner(|owner, epoch| {
            owner.update_retained_telemetry(epoch, &old, &next, &held)
        })?;
        protection.check_state(&old.state, &directory)?;
        entry.record = Some(next);
        entry.phase = Phase::Sealed;
        Ok(())
    }
}

fn require_records_access(records: &File) -> Result<()> {
    // SAFETY: F_GETFL observes flags on this live original File descriptor.
    let flags = unsafe { libc::fcntl(records.as_raw_fd(), libc::F_GETFL) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // The canonical producer appends through this same open-file description;
    // later sealing reads it without changing the shared writer offset.
    if flags & libc::O_ACCMODE != libc::O_RDWR || flags & libc::O_APPEND == 0 {
        return Err(SupervisorError::Profile);
    }
    Ok(())
}

fn original_file(directory: &File, name: &str, actual: &File, cap: u64) -> Result<(u64, u64)> {
    let name = std::ffi::CString::new(name).map_err(|_| SupervisorError::Profile)?;
    // SAFETY: fixed canonical name and live original directory; fresh FD.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    use std::os::fd::FromRawFd;
    // SAFETY: successful openat returned this fresh, uniquely owned descriptor.
    let named = unsafe { File::from_raw_fd(fd) };
    crate::installed_owner::retained_telemetry::require_private_retained_file(&named)?;
    crate::installed_owner::retained_telemetry::require_private_retained_file(actual)?;
    let expected = named.metadata()?;
    let held = actual.metadata()?;
    for meta in [&expected, &held] {
        mvm_core::private_fs::require_current_owner(meta)?;
        if !meta.is_file() || meta.mode() & 0o7777 != 0o600 || meta.nlink() != 1 || meta.len() > cap
        {
            return Err(SupervisorError::Profile);
        }
    }
    if (expected.dev(), expected.ino()) != (held.dev(), held.ino()) {
        return Err(SupervisorError::Profile);
    }
    Ok((held.dev(), held.ino()))
}

impl Drop for RetainedTelemetryInventory {
    fn drop(&mut self) {
        let entries = self
            .entries
            .get_mut()
            .unwrap_or_else(|error| error.into_inner());
        if entries.iter().any(Option::is_some) {
            self.authority.close_admission();
            // Preserve the complete bounded original table, file pins and
            // credits. The durable catalog also denies a fresh zero epoch.
            let held = std::mem::replace(entries, std::array::from_fn(|_| None));
            std::mem::forget(held);
            std::mem::forget(self.metadata.clone());
        }
    }
}
