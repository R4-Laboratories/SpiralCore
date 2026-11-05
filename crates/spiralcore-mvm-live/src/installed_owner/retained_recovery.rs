//! Closed old-epoch debt custody. No issuer, deletion or epoch transition API.
use super::transfer::{CatalogMode, CatalogV2, OriginRecord};
use super::*;
use crate::{
    installed_resources::{ClosedRetainedFunding, ManagedSnapshot},
    installed_supervisor::{InstalledAuthority, SupervisorError},
};
use mvm_core::runtime_state_protection::{
    RETAINED_TELEMETRY_MARKER_FILE, RuntimeStateProtection, TELEMETRY_COLLECTOR_STATUS_FILE,
    TELEMETRY_RECORDS_FILE, TELEMETRY_RECORDS_KEY_FILE,
};
use std::os::unix::fs::FileExt;

#[path = "retained_transition.rs"]
pub(crate) mod transition;

#[cfg(test)]
#[path = "retained_live_tests.rs"]
mod live_tests;

type RecoveryResult<T> = std::result::Result<T, SupervisorError>;

/// Compiled compatibility, never a caller-supplied profile or limit grant.
pub(crate) struct RetainedRecoveryCompatibility {
    profile: [u8; 32],
}
impl RetainedRecoveryCompatibility {
    pub(crate) fn current() -> RecoveryResult<Self> {
        Self::published_v1()?; // preserve the qualified role/architecture check
        Ok(Self {
            profile: crate::installed_supervisor::InstalledConfig::canonical_native_profile()
                .profile_hash(),
        })
    }
    pub(crate) fn published_v1() -> RecoveryResult<Self> {
        if !cfg!(all(
            target_os = "linux",
            target_arch = "x86_64",
            target_pointer_width = "64"
        )) {
            return Err(SupervisorError::Profile);
        }
        // Actual protected old Linux/x64 source+profile fixture, not current
        // include_bytes-derived config. Other profiles require qualification.
        Ok(Self {
            profile: [
                0xfc, 0xa2, 0xaf, 0x7e, 0x09, 0xa0, 0xb8, 0xf0, 0x78, 0x2b, 0x32, 0x3d, 0x09, 0x33,
                0x4c, 0x96, 0x83, 0xd9, 0x09, 0x79, 0x6b, 0x91, 0x8e, 0xff, 0x49, 0x52, 0x27, 0xb6,
                0xf2, 0x7c, 0x4f, 0x0f,
            ],
        })
    }
}

/// Actual trusted canonical parent, not authority reconstructed from a row.
pub(crate) struct CanonicalRecoveryLayout {
    path: PathBuf,
    directory: File,
    id: ObjectId,
}
impl CanonicalRecoveryLayout {
    pub(crate) fn from_trusted_parent(path: &Path, directory: File) -> Result<Self> {
        if path.as_os_str().len() > 4096 {
            return Err(OwnerError::InvalidStorage);
        }
        let original = Self {
            path: path.to_owned(),
            id: object_id(&directory)?,
            directory,
        };
        original.check()?;
        Ok(original)
    }
    fn check(&self) -> Result<()> {
        check_private(&self.directory, true)?;
        check_filesystem(&self.directory)?;
        // SAFETY: F_GETFL only observes this live original descriptor.
        let flags = unsafe { libc::fcntl(self.directory.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error().into());
        }
        if flags & libc::O_ACCMODE != libc::O_RDONLY
            || object_id(&self.directory)? != self.id
            || object_id(&open_root(&self.path)?)? != self.id
        {
            return Err(OwnerError::InvalidStorage);
        }
        Ok(())
    }
    fn duplicate(&self) -> Result<Self> {
        self.check()?;
        Ok(Self {
            path: self.path.clone(),
            directory: self.directory.try_clone()?,
            id: self.id,
        })
    }
}

/// Private original kernel exclusion, consumed by the closed authority.
pub(crate) struct RetainedExclusion {
    owner: InstallationOwner,
    holders: File,
    epoch: Epoch,
}
impl RetainedExclusion {
    pub(crate) fn into_parts(self) -> (InstallationOwner, File, Epoch) {
        (self.owner, self.holders, self.epoch)
    }
}

// Armed from function entry, including unwind before a Result is returned.
// Every already-acquired observation stays private and grants no authority.
struct Preparation {
    owner: Option<Box<InstallationOwner>>,
    holders: Option<File>,
    funding: Option<ClosedRetainedFunding>,
}
impl Drop for Preparation {
    fn drop(&mut self) {
        if let Some(value) = self.holders.take() {
            std::mem::forget(value);
        }
        if let Some(value) = self.owner.take() {
            std::mem::forget(value);
        }
        if let Some(value) = self.funding.take() {
            std::mem::forget(value);
        }
    }
}
pub(crate) struct RecoveryStartFailure {
    error: SupervisorError,
    _custody: Preparation,
}
impl fmt::Debug for RecoveryStartFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("closed retained recovery preparation refused; custody retained")
    }
}

impl InstallationOwner {
    pub(crate) fn prepare_retained_recovery(
        self,
        compatibility: &RetainedRecoveryCompatibility,
        layout: &CanonicalRecoveryLayout,
    ) -> std::result::Result<RetainedRecovery, RecoveryStartFailure> {
        let mut custody = Preparation {
            owner: Some(Box::new(self)),
            holders: None,
            funding: None,
        };
        let checked = (|| -> RecoveryResult<_> {
            let owner = custody.owner.as_ref().ok_or(SupervisorError::State)?;
            let current = owner.current()?;
            if current.phase != Phase::Active
                || current.generation == 0
                || current.key.profile != compatibility.profile
                || owner.supervisor.is_some()
                || crate::installed_retained::RECORD_BYTES != 8 << 20
                || crate::installed_retained::SIDECAR_BYTES != 32 << 10
            {
                return Err(SupervisorError::Profile);
            }
            layout.check()?;
            owner.storage.check()?;
            owner
                .storage
                .absent_names(&[TRANSFER_INTENT, TRANSFER_NEXT])?;
            // Own the raw observation BEFORE private/identity/lock checks.
            observe_at(
                &mut custody.holders,
                &owner.storage.directory,
                HOLDERS,
                libc::O_RDWR,
            )?;
            let holders = custody.holders.as_ref().ok_or(SupervisorError::State)?;
            check_private(holders, false)?;
            if object_id(holders)? != owner.storage.ids[2] {
                return Err(SupervisorError::Profile);
            }
            lock(holders, false)?;
            owner.storage.check()?;
            #[cfg(test)]
            tests::post_holders_current_fault()?;
            owner.current()?;
            Ok(Epoch {
                key: current.key,
                generation: current.generation,
            })
        })();
        let epoch = match checked {
            Ok(value) => value,
            Err(error) => {
                return Err(RecoveryStartFailure {
                    error,
                    _custody: custody,
                });
            }
        };
        // Only infallible moves/fixed allocator construction occur between
        // these owners; allocator exhaustion aborts rather than unwinding.
        let authority = InstalledAuthority::from_retained_recovery(RetainedExclusion {
            owner: *custody.owner.take().expect("original owner"),
            holders: custody.holders.take().expect("original holders"),
            epoch,
        });
        custody.funding = Some(ClosedRetainedFunding::new(authority));
        if let Err(error) = custody
            .funding
            .as_ref()
            .expect("original funding")
            .fund_metadata()
        {
            return Err(RecoveryStartFailure {
                error: error.into(),
                _custody: custody,
            });
        }
        let layout = match layout.duplicate() {
            Ok(layout) => layout,
            Err(error) => {
                return Err(RecoveryStartFailure {
                    error: error.into(),
                    _custody: custody,
                });
            }
        };
        let funding = custody.funding.take().expect("original funding");
        Ok(RetainedRecovery {
            held: Some(Box::new(Custody {
                funding: Some(funding),
                rebind_failure: None,
                layout,
                epoch,
                protection: None,
                catalog: None,
                catalog_observation: None,
                catalog_bytes: Vec::new(),
                entries: std::array::from_fn(|_| None),
                observations: std::array::from_fn(|_| Observations::default()),
                failed: false,
            })),
        })
    }
}

struct PinnedEntry {
    origin: Epoch,
    record: RetainedRecord,
    directory: File,
    marker: Option<File>,
    records: Option<File>,
    key: Option<File>,
    status: Option<File>,
}
#[derive(Default)]
struct FileObservations {
    marker: Option<File>,
    records: Option<File>,
    key: Option<File>,
    status: Option<File>,
}
#[derive(Default)]
struct Observations {
    state: Option<File>,
    files: FileObservations,
}
struct Custody {
    funding: Option<ClosedRetainedFunding>,
    rebind_failure: Option<crate::installed_resources::RetainedFundingRebindFailure>,
    layout: CanonicalRecoveryLayout,
    epoch: Epoch,
    protection: Option<RuntimeStateProtection>,
    catalog: Option<File>,
    catalog_observation: Option<File>,
    catalog_bytes: Vec<u8>,
    entries: [Option<PinnedEntry>; MAX_ENTRIES],
    observations: [Observations; MAX_ENTRIES],
    failed: bool,
}

pub(crate) struct RetainedRecovery {
    held: Option<Box<Custody>>,
}
impl RetainedRecovery {
    pub(crate) fn adopt_closed(
        mut self,
    ) -> std::result::Result<ClosedRetainedDebt, RecoveryBlocked> {
        let outcome = self
            .held
            .as_mut()
            .ok_or(SupervisorError::State)
            .and_then(|held| held.adopt());
        match outcome {
            Ok(()) => Ok(ClosedRetainedDebt { recovery: self }),
            Err(error) => {
                if let Some(held) = self.held.as_mut() {
                    held.failed = true;
                }
                Err(RecoveryBlocked {
                    error,
                    recovery: self,
                })
            }
        }
    }
}
impl Drop for RetainedRecovery {
    fn drop(&mut self) {
        // Until a separately reviewed transfer/disposition exists, retain the
        // COMPLETE bounded table, ledger and original locks through uncertainty.
        if let Some(held) = self.held.take() {
            std::mem::forget(held);
        }
    }
}
pub(crate) struct RecoveryBlocked {
    error: SupervisorError,
    recovery: RetainedRecovery,
}
impl fmt::Debug for RecoveryBlocked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("closed retained recovery refused; same custody retained")
    }
}
pub(crate) struct RetainedDebtSnapshot {
    pub(crate) entries: usize,
    pub(crate) unresolved: usize,
    pub(crate) managed: ManagedSnapshot,
}
pub(crate) struct ClosedRetainedDebt {
    recovery: RetainedRecovery,
}
impl ClosedRetainedDebt {
    pub(crate) fn snapshot(&self) -> RecoveryResult<RetainedDebtSnapshot> {
        let held = self.recovery.held.as_ref().ok_or(SupervisorError::State)?;
        if held.failed {
            return Err(SupervisorError::State);
        }
        let entries = held.entries.iter().flatten();
        Ok(RetainedDebtSnapshot {
            entries: entries.clone().count(),
            unresolved: entries.filter(|entry| entry.record.phase != 4).count(),
            managed: held.funding()?.snapshot()?,
        })
    }
    pub(crate) fn revalidate(&mut self) -> RecoveryResult<()> {
        let held = self.recovery.held.as_mut().ok_or(SupervisorError::State)?;
        let result = held.revalidate();
        if result.is_err() {
            held.failed = true;
        }
        result
    }
}

fn supported_profile(profile: [u8; 32]) -> bool {
    RetainedRecoveryCompatibility::published_v1().is_ok_and(|known| known.profile == profile)
        || RetainedRecoveryCompatibility::current().is_ok_and(|known| known.profile == profile)
}
fn decode_beacon(bytes: &[u8]) -> Result<Epoch> {
    if bytes.len() != 112
        || &bytes[..8] != b"SCRETB01"
        || Sha256::digest(&bytes[..80]).as_slice() != &bytes[80..]
    {
        return Err(OwnerError::Fenced);
    }
    let value = Epoch {
        key: InstallationKey {
            installation: bytes[8..40].try_into().unwrap(),
            profile: bytes[40..72].try_into().unwrap(),
        },
        generation: u64::from_le_bytes(bytes[72..80].try_into().unwrap()),
    };
    check_key(value.key)?;
    if value.generation == 0 || !supported_profile(value.key.profile) {
        return Err(OwnerError::Fenced);
    }
    Ok(value)
}
fn decode_catalog(bytes: &[u8], epoch: Epoch) -> Result<CatalogV2> {
    if bytes.starts_with(b"SCRETN01") {
        let old = Catalog::decode(bytes, epoch)?;
        Ok(CatalogV2 {
            epoch,
            mode: CatalogMode::Active,
            entries: old
                .entries
                .into_iter()
                .map(|record| OriginRecord {
                    origin: epoch,
                    record,
                })
                .collect(),
        })
    } else {
        CatalogV2::decode(bytes, epoch)
    }
}

fn bounded_bytes(file: &File, cap: usize) -> Result<Vec<u8>> {
    let length = usize::try_from(file.metadata()?.len()).map_err(|_| OwnerError::Fenced)?;
    if length > cap {
        return Err(OwnerError::Fenced);
    }
    let mut bytes = vec![0; length];
    file.read_exact_at(&mut bytes, 0)?;
    if file.metadata()?.len() != length as u64 {
        return Err(OwnerError::Fenced);
    }
    Ok(bytes)
}
fn present(directory: &File, name: &str) -> Result<bool> {
    let name = CString::new(name).map_err(|_| OwnerError::InvalidStorage)?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: live directory, fixed NUL-terminated name and writable output.
    if unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } == 0
    {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ENOENT) {
        Ok(false)
    } else {
        Err(error.into())
    }
}
fn observe_at(slot: &mut Option<File>, directory: &File, name: &str, flags: i32) -> Result<()> {
    if slot.is_some() {
        return Err(OwnerError::Fenced);
    }
    let name = CString::new(name).map_err(|_| OwnerError::InvalidStorage)?;
    // SAFETY: live directory and NUL-terminated name; no incoming raw FD.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(storage_open_error(io::Error::last_os_error()));
    }
    // SAFETY: the successful open returned this fresh owned FD. Assignment to
    // an empty custody slot precedes EVERY fallible post-open operation.
    *slot = Some(unsafe { File::from_raw_fd(fd) });
    Ok(())
}
fn observe_state(slot: &mut Option<File>, path: &Path) -> Result<()> {
    if slot.is_some() {
        return Err(OwnerError::Fenced);
    }
    let path = c_path(path)?;
    // SAFETY: live NUL-terminated path; successful open creates a fresh FD.
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(storage_open_error(io::Error::last_os_error()));
    }
    // SAFETY: immediately own the fresh descriptor before metadata validation.
    *slot = Some(unsafe { File::from_raw_fd(fd) });
    Ok(())
}
fn capture_optional(
    slot: &mut Option<File>,
    directory: &File,
    name: &str,
    cap: usize,
) -> Result<()> {
    if !present(directory, name)? {
        return Ok(());
    }
    observe_at(slot, directory, name, libc::O_RDONLY)?;
    let file = slot.as_ref().ok_or(OwnerError::Fenced)?;
    require_private_retained_file(file)?;
    if file.metadata()?.len() > cap as u64 {
        return Err(OwnerError::Fenced);
    }
    Ok(())
}
fn same_optional(a: &Option<File>, b: &Option<File>) -> Result<bool> {
    match (a, b) {
        (Some(a), Some(b)) => Ok(object_id(a)? == object_id(b)?),
        (None, None) => Ok(true),
        _ => Ok(false),
    }
}

impl Custody {
    fn funding(&self) -> RecoveryResult<&ClosedRetainedFunding> {
        self.funding.as_ref().ok_or(SupervisorError::State)
    }
    fn observe_catalog(&mut self) -> RecoveryResult<(Vec<u8>, CatalogV2)> {
        self.funding()?.check()?;
        let authority = self.funding()?.authority();
        authority.with_disk_owner(|owner, epoch| {
            owner.active_binding(epoch)?;
            if epoch != self.epoch {
                return Err(OwnerError::StaleEpoch);
            }
            let beacon = bounded_bytes(&owner._owner, 112)?;
            let origin = decode_beacon(&beacon)?;
            if origin.key.installation != epoch.key.installation
                || origin.generation > epoch.generation
            {
                return Err(OwnerError::Fenced);
            }
            owner
                .storage
                .absent_names(&[CATALOG_INTENT, CATALOG_NEXT])?;
            observe_at(
                &mut self.catalog_observation,
                &owner.storage.directory,
                CATALOG,
                libc::O_RDONLY,
            )?;
            let file = self
                .catalog_observation
                .as_ref()
                .ok_or(OwnerError::Fenced)?;
            require_private_retained_file(file)?;
            let bytes = bounded_bytes(file, MAX_BYTES)?;
            let catalog = decode_catalog(&bytes, epoch)?;
            if catalog.entries.is_empty()
                || catalog.entries[0].origin != origin
                || catalog
                    .entries
                    .iter()
                    .any(|row| !supported_profile(row.origin.key.profile))
            {
                return Err(OwnerError::Fenced);
            }
            owner.active_binding(epoch)?;
            Ok((bytes, catalog))
        })
    }
    fn adopt(&mut self) -> RecoveryResult<()> {
        let (bytes, catalog) = self.observe_catalog()?;
        self.catalog = self.catalog_observation.take();
        self.catalog_bytes = bytes;
        let mut tenants = [[0; 32]; MAX_ENTRIES];
        for (at, row) in catalog.entries.iter().enumerate() {
            tenants[at] = row.record.tenant;
        }
        self.funding
            .as_mut()
            .ok_or(SupervisorError::State)?
            .fund_rows(&tenants[..catalog.entries.len()])?;
        self.layout.check()?;
        self.protection = Some(RuntimeStateProtection::acquire(
            &self.layout.path,
            &self.layout.directory,
        )?);
        let protection = self.protection.as_ref().ok_or(SupervisorError::State)?;
        for (at, row) in catalog.entries.into_iter().enumerate() {
            let OriginRecord { origin, record } = row;
            let observed = &mut self.observations[at];
            Self::capture_state(&self.layout, protection, &record, observed)?;
            // This state has passed its original checks. Unqualified later
            // observations remain in a SEPARATE private fixed slot on errors.
            self.entries[at] = Some(PinnedEntry {
                origin,
                record,
                directory: observed.state.take().ok_or(SupervisorError::State)?,
                marker: None,
                records: None,
                key: None,
                status: None,
            });
            let entry = self.entries[at].as_mut().ok_or(SupervisorError::State)?;
            Self::capture_files(
                protection,
                entry.origin,
                &entry.record,
                &entry.directory,
                &mut observed.files,
            )?;
            entry.marker = observed.files.marker.take();
            entry.records = observed.files.records.take();
            entry.key = observed.files.key.take();
            entry.status = observed.files.status.take();
        }
        self.revalidate()
    }
    fn capture_state(
        layout: &CanonicalRecoveryLayout,
        protection: &RuntimeStateProtection,
        record: &RetainedRecord,
        observed: &mut Observations,
    ) -> RecoveryResult<()> {
        if !record.valid() || record.state.parent() != Some(layout.path.as_path()) {
            return Err(SupervisorError::Profile);
        }
        observe_state(&mut observed.state, &record.state)?;
        let directory = observed.state.as_ref().ok_or(SupervisorError::State)?;
        require_private_retained_directory(directory)?;
        if object_id(directory)?
            != (ObjectId {
                device: record.state_device,
                inode: record.state_inode,
            })
        {
            return Err(SupervisorError::Profile);
        }
        protection.check_state(&record.state, directory)?;
        Ok(())
    }
    fn capture_files(
        protection: &RuntimeStateProtection,
        epoch: Epoch,
        record: &RetainedRecord,
        directory: &File,
        files: &mut FileObservations,
    ) -> RecoveryResult<()> {
        capture_optional(
            &mut files.marker,
            directory,
            RETAINED_TELEMETRY_MARKER_FILE,
            224,
        )?;
        if let Some(marker) = &files.marker {
            if bounded_bytes(marker, 224)? != record.marker(epoch) {
                return Err(SupervisorError::State);
            }
        } else if record.phase != 1 {
            return Err(SupervisorError::State);
        }
        capture_optional(
            &mut files.records,
            directory,
            TELEMETRY_RECORDS_FILE,
            8 << 20,
        )?;
        capture_optional(&mut files.key, directory, TELEMETRY_RECORDS_KEY_FILE, 1024)?;
        capture_optional(
            &mut files.status,
            directory,
            TELEMETRY_COLLECTOR_STATUS_FILE,
            4096,
        )?;
        if record.phase >= 3 {
            let records = files.records.as_ref().ok_or(SupervisorError::State)?;
            let key = files.key.as_ref().ok_or(SupervisorError::State)?;
            if object_id(records)?
                != (ObjectId {
                    device: record.record_id.0,
                    inode: record.record_id.1,
                })
                || object_id(key)?
                    != (ObjectId {
                        device: record.key_id.0,
                        inode: record.key_id.1,
                    })
            {
                return Err(SupervisorError::Profile);
            }
            if record.phase == 4 {
                if records.metadata()?.len() != record.sealed_len {
                    return Err(SupervisorError::State);
                }
                let mut digest = Sha256::new();
                let mut buffer = [0; 8192];
                let mut offset = 0;
                while offset < record.sealed_len {
                    let limit = usize::try_from((record.sealed_len - offset).min(8192))
                        .map_err(|_| SupervisorError::State)?;
                    let count = records.read_at(&mut buffer[..limit], offset)?;
                    if count == 0 {
                        return Err(SupervisorError::State);
                    }
                    digest.update(&buffer[..count]);
                    offset += count as u64;
                }
                if digest.finalize().as_slice() != record.sealed_sha256
                    || records.metadata()?.len() != record.sealed_len
                {
                    return Err(SupervisorError::State);
                }
            }
        }
        protection.check_state(&record.state, directory)?;
        require_private_retained_directory(directory)?;
        Ok(())
    }
    fn revalidate_rows(&mut self) -> RecoveryResult<()> {
        let protection = self.protection.as_ref().ok_or(SupervisorError::State)?;
        for at in 0..MAX_ENTRIES {
            let Some(entry) = self.entries[at].as_ref() else {
                continue;
            };
            let fresh = &mut self.observations[at];
            Self::capture_state(&self.layout, protection, &entry.record, fresh)?;
            Self::capture_files(
                protection,
                entry.origin,
                &entry.record,
                fresh.state.as_ref().ok_or(SupervisorError::State)?,
                &mut fresh.files,
            )?;
            if object_id(fresh.state.as_ref().ok_or(SupervisorError::State)?)?
                != object_id(&entry.directory)?
                || !same_optional(&fresh.files.marker, &entry.marker)?
                || !same_optional(&fresh.files.records, &entry.records)?
                || !same_optional(&fresh.files.key, &entry.key)?
                || !same_optional(&fresh.files.status, &entry.status)?
            {
                return Err(SupervisorError::State);
            }
        }
        // Every duplicate has positively matched the retained ORIGINAL pins.
        // Release only these fresh observations before a later revalidation.
        self.observations = std::array::from_fn(|_| Observations::default());
        Ok(())
    }
    fn revalidate(&mut self) -> RecoveryResult<()> {
        if self.failed {
            return Err(SupervisorError::State);
        }
        self.layout.check()?;
        let (bytes, _) = self.observe_catalog()?;
        if bytes != self.catalog_bytes
            || object_id(
                self.catalog_observation
                    .as_ref()
                    .ok_or(SupervisorError::State)?,
            )? != object_id(self.catalog.as_ref().ok_or(SupervisorError::State)?)?
        {
            return Err(SupervisorError::State);
        }
        self.revalidate_rows()?;
        self.funding()?.check()?;
        // Only successful comparison permits dropping duplicate observations.
        // Originals have never been overwritten, including on Err/unwind.
        self.catalog_observation.take();
        self.observations = std::array::from_fn(|_| Observations::default());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    thread_local! { static POST_HOLDERS_FAULT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) }; }

    // Explicit synthetic fault at the late-current validation boundary, not a
    // claim that the fixture naturally corrupts an epoch or kernel lock.
    pub(super) fn post_holders_current_fault() -> RecoveryResult<()> {
        match POST_HOLDERS_FAULT.with(|fault| fault.replace(0)) {
            1 => Err(SupervisorError::State),
            2 => panic!("explicit post-holder validation unwind"),
            _ => Ok(()),
        }
    }

    fn actual_old_fixture() -> tempfile::TempDir {
        let binary = PathBuf::from(
            std::env::var_os("SPIRALCORE_OLD_RETAINED_BIN")
                .expect("protected old executable required"),
        );
        let digest = Sha256::digest(fs::read(&binary).unwrap());
        let digest: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            digest,
            "45ddf9d61f3bdb853f554788645aa3a176aac9294712f7da482755d33e996659"
        );
        let home = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let status = std::process::Command::new(binary)
            .args([
                "--exact",
                "installed_supervisor::retained_tests::retained_recovery_old_process_fixture",
                "--test-threads=1",
            ])
            .env("SPIRALCORE_OLD_RETAINED_FIXTURE", home.path())
            .status()
            .unwrap();
        assert!(status.success());
        home
    }
    fn prepare_old(home: &Path) -> RetainedRecovery {
        let compatibility = RetainedRecoveryCompatibility::published_v1().unwrap();
        let owner = InstallationOwner::open(
            &home.join("installation"),
            InstallationKey {
                installation: [121; 32],
                profile: compatibility.profile,
            },
        )
        .unwrap();
        let parent = home.join("vms");
        let layout =
            CanonicalRecoveryLayout::from_trusted_parent(&parent, File::open(&parent).unwrap())
                .unwrap();
        owner
            .prepare_retained_recovery(&compatibility, &layout)
            .unwrap()
    }
    fn open_observations(id: ObjectId) -> usize {
        // Linux test-only observation of THIS process's actual descriptors.
        // No PID attachment, signaling, authority construction or FD adoption.
        fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| fs::metadata(entry.path()).ok())
            .filter(|m| m.dev() == id.device && m.ino() == id.inode)
            .count()
    }
    fn expect_acquisition_blocked(recovery: RetainedRecovery) -> RecoveryBlocked {
        match recovery.adopt_closed() {
            Ok(_) => panic!("fault was admitted"),
            Err(blocked) => blocked,
        }
    }

    #[test]
    #[ignore = "requires protected previous-product process; Linux fd observation"]
    fn acquisition_catalog_crc_refusal_retains_actual_descriptor() {
        let home = actual_old_fixture();
        let path = home.path().join("installation/.retained-telemetry.bin");
        let original = File::open(&path).unwrap();
        let id = object_id(&original).unwrap();
        let mut bytes = fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(&path, &bytes).unwrap();
        let blocked = expect_acquisition_blocked(prepare_old(home.path()));
        assert!(
            open_observations(id) >= 2,
            "CRC refusal closed the actual catalog observation"
        );
        drop(blocked);
        assert!(
            open_observations(id) >= 2,
            "failed-object Drop closed the catalog observation"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    #[test]
    #[ignore = "requires protected previous-product process; Linux fd observation"]
    fn acquisition_oversized_marker_retains_actual_descriptor_and_full_debt() {
        let home = actual_old_fixture();
        let path = home
            .path()
            .join("vms/retained-before-restart/.installed-retained-telemetry");
        let original = File::open(&path).unwrap();
        let id = object_id(&original).unwrap();
        let mut bytes = fs::read(&path).unwrap();
        bytes.push(0);
        fs::write(&path, &bytes).unwrap();
        let blocked = expect_acquisition_blocked(prepare_old(home.path()));
        let snapshot = blocked
            .recovery
            .held
            .as_ref()
            .unwrap()
            .funding()
            .unwrap()
            .snapshot()
            .unwrap();
        assert_eq!(
            snapshot.usage.cache_disk_bytes,
            (8 << 20) + (32 << 10) + 8192
        );
        assert!(snapshot.fenced);
        assert!(
            open_observations(id) >= 2,
            "cap refusal closed the actual marker observation"
        );
        drop(blocked);
        assert!(
            open_observations(id) >= 2,
            "failed-object Drop closed marker observation"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    #[test]
    #[ignore = "requires protected previous-product process; Linux fd observation"]
    fn acquisition_state_privacy_refusal_retains_actual_descriptor() {
        let home = actual_old_fixture();
        let path = home.path().join("vms/retained-before-restart");
        let original = File::open(&path).unwrap();
        let id = object_id(&original).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let blocked = expect_acquisition_blocked(prepare_old(home.path()));
        let snapshot = blocked
            .recovery
            .held
            .as_ref()
            .unwrap()
            .funding()
            .unwrap()
            .snapshot()
            .unwrap();
        assert_eq!(
            snapshot.usage.cache_disk_bytes,
            (8 << 20) + (32 << 10) + 8192
        );
        assert!(snapshot.fenced);
        assert!(
            open_observations(id) >= 2,
            "post-open privacy refusal closed actual state observation"
        );
        drop(blocked);
        assert!(
            open_observations(id) >= 2,
            "failed-object Drop closed state observation"
        );
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o755);
    }
    fn late_fault(home: &Path, mode: u8) {
        let compatibility = RetainedRecoveryCompatibility::published_v1().unwrap();
        let key = InstallationKey {
            installation: [121; 32],
            profile: compatibility.profile,
        };
        let owner = InstallationOwner::open(&home.join("installation"), key).unwrap();
        let parent = home.join("vms");
        let layout =
            CanonicalRecoveryLayout::from_trusted_parent(&parent, File::open(&parent).unwrap())
                .unwrap();
        POST_HOLDERS_FAULT.with(|fault| fault.set(mode));
        if mode == 1 {
            let failure = match owner.prepare_retained_recovery(&compatibility, &layout) {
                Ok(_) => panic!("injected fault admitted"),
                Err(failure) => failure,
            };
            let probe = File::options()
                .read(true)
                .write(true)
                .open(home.join("installation/holders.lock"))
                .unwrap();
            assert!(
                matches!(probe.try_lock(), Err(fs::TryLockError::WouldBlock)),
                "late-current error released acquired holders EX"
            );
            drop(failure);
            assert!(
                matches!(probe.try_lock(), Err(fs::TryLockError::WouldBlock)),
                "failure Drop released holders EX"
            );
        } else {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                owner.prepare_retained_recovery(&compatibility, &layout)
            }));
            assert!(result.is_err());
            assert!(
                matches!(
                    InstallationOwner::open(&home.join("installation"), key),
                    Err(OwnerError::Busy)
                ),
                "unwind released original owner EX"
            );
            let probe = File::options()
                .read(true)
                .write(true)
                .open(home.join("installation/holders.lock"))
                .unwrap();
            assert!(
                matches!(probe.try_lock(), Err(fs::TryLockError::WouldBlock)),
                "unwind released holders EX"
            );
        }
    }
    #[test]
    #[ignore = "requires protected previous-product process; explicit cfg-test fault"]
    fn acquisition_late_current_fault_retains_holders_exclusion() {
        let home = actual_old_fixture();
        late_fault(home.path(), 1);
    }
    #[test]
    #[ignore = "requires protected previous-product process; explicit cfg-test unwind"]
    fn acquisition_unwind_retains_original_owner_and_holders() {
        let home = actual_old_fixture();
        late_fault(home.path(), 2);
    }

    #[test]
    #[ignore = "requires the protected previous-product executable"]
    fn malformed_marker_retains_the_original_state_pin_in_blocked_custody() {
        let binary = std::env::var_os("SPIRALCORE_OLD_RETAINED_BIN")
            .expect("explicit protected old product executable required");
        let binary = PathBuf::from(binary);
        let expected = "45ddf9d61f3bdb853f554788645aa3a176aac9294712f7da482755d33e996659";
        let actual = Sha256::digest(std::fs::read(&binary).unwrap());
        let actual: String = actual.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(actual, expected, "old fixture executable identity changed");
        let home = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let status = std::process::Command::new(&binary)
            .args([
                "--exact",
                "installed_supervisor::retained_tests::retained_recovery_old_process_fixture",
                "--test-threads=1",
            ])
            .env("SPIRALCORE_OLD_RETAINED_FIXTURE", home.path())
            .status()
            .unwrap();
        assert!(status.success());
        let state = home.path().join("vms/retained-before-restart");
        let original = File::open(&state).unwrap();
        let original_id = object_id(&original).unwrap();
        let marker = state.join(RETAINED_TELEMETRY_MARKER_FILE);
        let mut bytes = fs::read(&marker).unwrap();
        bytes[0] ^= 1;
        fs::write(&marker, &bytes).unwrap();
        let compatibility = RetainedRecoveryCompatibility::published_v1().unwrap();
        let key = InstallationKey {
            installation: [121; 32],
            profile: compatibility.profile,
        };
        let owner = InstallationOwner::open(&home.path().join("installation"), key).unwrap();
        let parent = home.path().join("vms");
        let layout =
            CanonicalRecoveryLayout::from_trusted_parent(&parent, File::open(&parent).unwrap())
                .unwrap();
        let recovery = owner
            .prepare_retained_recovery(&compatibility, &layout)
            .unwrap();
        let blocked = match recovery.adopt_closed() {
            Ok(_) => panic!("malformed marker was accepted"),
            Err(blocked) => blocked,
        };
        let held = blocked.recovery.held.as_ref().unwrap();
        let snapshot = held.funding().unwrap().snapshot().unwrap();
        assert_eq!(
            snapshot.usage.cache_disk_bytes,
            (8 << 20) + (32 << 10) + 8192
        );
        assert!(snapshot.fenced);
        let pinned = held.entries[0]
            .as_ref()
            .expect("late validation error dropped the original state pin");
        assert_eq!(object_id(&pinned.directory).unwrap(), original_id);
        assert_eq!(
            fs::read(&marker).unwrap(),
            bytes,
            "recovery changed denied evidence"
        );
        assert!(matches!(
            InstallationOwner::open(&home.path().join("installation"), key),
            Err(OwnerError::Busy)
        ));
    }

    #[test]
    #[ignore = "requires protected previous-product process; actual filesystem normal-publisher boundary"]
    fn b2_normal_publisher_cannot_replace_epoch_without_transfer_transaction() {
        let home = actual_old_fixture();
        let installation = home.path().join("installation");
        let catalog_path = installation.join(CATALOG);
        let before = fs::read(&catalog_path).unwrap();
        let state_before = fs::read(installation.join(STATE)).unwrap();
        let closed = prepare_old(home.path()).adopt_closed().unwrap();
        let held = closed.recovery.held.as_ref().unwrap();
        let usage_before = held.funding().unwrap().snapshot().unwrap().usage;
        let result = held
            .funding()
            .unwrap()
            .authority()
            .with_disk_owner(|owner, old| {
                let mut target = owner.storage.retained_catalog(old)?;
                target.epoch = Epoch {
                    key: InstallationKey {
                        installation: old.key.installation,
                        profile:
                            crate::installed_supervisor::InstalledConfig::canonical_native_profile()
                                .profile_hash(),
                    },
                    generation: old
                        .generation
                        .checked_add(1)
                        .ok_or(OwnerError::EpochExhausted)?,
                };
                owner.storage.publish_retained(&target)
            });
        let after = fs::read(&catalog_path).unwrap();
        let unchanged_state = fs::read(installation.join(STATE)).unwrap() == state_before;
        let intent_exists = installation.join(".retained-telemetry.transfer").exists();
        let hash = |bytes: &[u8]| -> String {
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        };
        eprintln!(
            "normal-publisher cross-epoch probe: before={} after={} owner_unchanged={} transfer_intent={}",
            hash(&before),
            hash(&after),
            unchanged_state,
            intent_exists
        );
        assert!(unchanged_state, "normal publication changed owner epoch");
        assert!(
            result.is_err(),
            "normal publisher replaced a catalog epoch/profile without immutable transfer intent"
        );
        assert_eq!(after, before);
        assert!(!intent_exists);
        assert_eq!(
            held.funding().unwrap().snapshot().unwrap().usage,
            usage_before
        );
    }

    pub(super) fn sealed_format_filesystem_fixture() -> tempfile::TempDir {
        // Real filesystem fixture ONLY. Direct phase4 format publication is NOT
        // a native producer, actual EndpointRetirement or activation evidence.
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let home = actual_old_fixture();
        let compatibility = RetainedRecoveryCompatibility::published_v1().unwrap();
        let key = InstallationKey {
            installation: [121; 32],
            profile: compatibility.profile,
        };
        let owner = InstallationOwner::open(&home.path().join("installation"), key).unwrap();
        let _exclusion = owner.holders_for_transition().unwrap();
        let current = owner.current().unwrap();
        let epoch = Epoch {
            key: current.key,
            generation: current.generation,
        };
        let mut catalog = owner.storage.retained_catalog(epoch).unwrap();
        let row = &mut catalog.entries[0];
        let directory = File::open(&row.state).unwrap();
        let parent = home.path().join("vms");
        let guard =
            RuntimeStateProtection::acquire(&parent, &File::open(&parent).unwrap()).unwrap();
        guard.check_state(&row.state, &directory).unwrap();
        let mut records = File::options()
            .read(true)
            .append(true)
            .create_new(true)
            .mode(0o600)
            .open(row.state.join(TELEMETRY_RECORDS_FILE))
            .unwrap();
        let bytes = b"synthetic retained ciphertext; no native proof\n";
        records.write_all(bytes).unwrap();
        records.sync_all().unwrap();
        let mut key_file = File::options()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(row.state.join(TELEMETRY_RECORDS_KEY_FILE))
            .unwrap();
        key_file.write_all(&[b'A'; 80]).unwrap();
        key_file.sync_all().unwrap();
        directory.sync_all().unwrap();
        let record_id = object_id(&records).unwrap();
        let key_id = object_id(&key_file).unwrap();
        row.phase = 4;
        row.record_id = (record_id.device, record_id.inode);
        row.key_id = (key_id.device, key_id.inode);
        row.sealed_len = bytes.len() as u64;
        row.sealed_sha256 = Sha256::digest(bytes).into();
        owner.storage.publish_retained(&catalog).unwrap();
        home
    }

    #[test]
    #[ignore = "requires protected old executable; actual FS metadata transaction, not native admission"]
    fn b2_closed_commit_preserves_full_debt_and_original_files() {
        use super::transition::CompiledRetainedMigration;
        let home = sealed_format_filesystem_fixture();
        let installation = home.path().join("installation");
        let state = home.path().join("vms/retained-before-restart");
        let evidence = [
            installation.join(OWNER),
            state.join(RETAINED_TELEMETRY_MARKER_FILE),
            state.join(TELEMETRY_RECORDS_FILE),
            state.join(TELEMETRY_RECORDS_KEY_FILE),
        ];
        let before: Vec<_> = evidence
            .iter()
            .map(|path| fs::read(path).unwrap())
            .collect();
        let closed = prepare_old(home.path()).adopt_closed().unwrap();
        let old_usage = closed.snapshot().unwrap().managed.usage;
        let held = closed.recovery.held.as_ref().unwrap();
        held.funding().unwrap().test_alias_guards(held.epoch);
        let migration = CompiledRetainedMigration::current().unwrap();
        let committed = closed
            .prepare_restart(&migration)
            .unwrap()
            .commit_closed()
            .unwrap();
        let binding = committed.binding().unwrap();
        assert_eq!(binding.generation, 2);
        assert_eq!(
            binding.key.profile,
            crate::installed_supervisor::InstalledConfig::canonical_native_profile().profile_hash()
        );
        let snapshot = committed.snapshot().unwrap();
        assert_eq!(snapshot.managed.usage, old_usage);
        assert!(snapshot.managed.fenced);
        assert_eq!(snapshot.entries, 1);
        assert_eq!(snapshot.unresolved, 0);
        let target = fs::read(installation.join(CATALOG)).unwrap();
        assert_eq!(&target[..8], b"SCRETN02");
        let record = Record::decode(&fs::read(installation.join(STATE)).unwrap()).unwrap();
        assert_eq!(record.phase, Phase::Active);
        assert_eq!(record.generation, 2);
        assert_eq!(record.key, binding.key);
        for (path, bytes) in evidence.iter().zip(before) {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        assert!(!installation.join(".retained-telemetry.transfer").exists());
        assert!(
            !installation
                .join(".retained-telemetry.transfer.next")
                .exists()
        );
        assert!(matches!(
            InstallationOwner::open(&installation, binding.key),
            Err(OwnerError::Busy)
        ));
    }
    // Explicit deterministic crash injection at a durably ordered transfer
    // boundary, never a claim about a naturally occurring storage failure.
    thread_local! {
        static LATE_STATE_REPLACEMENT: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
        static PUBLISH_INTENT_COLLISION: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
        static LATE_STATE_ID: std::cell::Cell<Option<(u64,u64)>> = const { std::cell::Cell::new(None) };
    }
    pub(super) fn transaction_barrier(name: &str) {
        if name == "publish_intent" {
            PUBLISH_INTENT_COLLISION.with(|slot| {
                if let Some(path) = slot.borrow_mut().take() {
                    review_private_file(&path, b"explicit before-publication collision");
                }
            });
        }

        if name == "state_intent" {
            LATE_STATE_REPLACEMENT.with(|slot| {
                if let Some(state) = slot.borrow_mut().take() {
                    use std::{
                        io::Write,
                        os::unix::fs::{MetadataExt, OpenOptionsExt},
                    };
                    fs::rename(&state, state.with_file_name("original-state-evidence")).unwrap();
                    let mut file = File::options()
                        .write(true)
                        .create_new(true)
                        .mode(0o600)
                        .open(&state)
                        .unwrap();
                    file.write_all(b"explicit malformed late state observation")
                        .unwrap();
                    file.sync_all().unwrap();
                    let meta = file.metadata().unwrap();
                    LATE_STATE_ID.with(|id| id.set(Some((meta.dev(), meta.ino()))));
                }
            });
        }

        if std::env::var("SPIRALCORE_B2_CRASH").as_deref() == Ok(name) {
            std::process::exit(77);
        }
    }

    #[test]
    fn b2_transaction_crash_child() {
        let Some(home) = std::env::var_os("SPIRALCORE_B2_HOME") else {
            return;
        };
        let closed = prepare_old(Path::new(&home)).adopt_closed().unwrap();
        let migration = super::transition::CompiledRetainedMigration::current().unwrap();
        let _committed = closed
            .prepare_restart(&migration)
            .unwrap()
            .commit_closed()
            .unwrap();
        if std::env::var("SPIRALCORE_B2_CRASH").as_deref() == Ok("committed") {
            std::process::exit(78);
        }
        panic!("requested transfer barrier was not reached");
    }

    #[test]
    #[ignore = "protected old process + actual crash durability metadata fixture; no native receipt"]
    fn b2_crash_boundaries_roll_forward_one_target_without_refunding() {
        use super::transition::{CompiledRetainedMigration, resume_retained_restart};
        for barrier in ["intent", "catalog", "state_intent", "state"] {
            let home = sealed_format_filesystem_fixture();
            let installation = home.path().join("installation");
            let original: Vec<_> = [OWNER, ".retained-telemetry.bin"]
                .iter()
                .map(|name| fs::read(installation.join(name)).unwrap())
                .collect();
            let state = home.path().join("vms/retained-before-restart");
            let original_files: Vec<_> = [
                RETAINED_TELEMETRY_MARKER_FILE,
                TELEMETRY_RECORDS_FILE,
                TELEMETRY_RECORDS_KEY_FILE,
            ]
            .iter()
            .map(|name| fs::read(state.join(name)).unwrap())
            .collect();
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "installed_owner::retained_telemetry::recovery::tests::b2_transaction_crash_child", "--test-threads=1"])
                .env("SPIRALCORE_B2_HOME", home.path()).env("SPIRALCORE_B2_CRASH", barrier)
                .status().unwrap();
            assert_eq!(status.code(), Some(77), "{barrier}");
            let layout_path = home.path().join("vms");
            let layout = CanonicalRecoveryLayout::from_trusted_parent(
                &layout_path,
                File::open(&layout_path).unwrap(),
            )
            .unwrap();
            let key = InstallationKey {
                installation: [121; 32],
                profile: RetainedRecoveryCompatibility::published_v1()
                    .unwrap()
                    .profile,
            };
            let migration = CompiledRetainedMigration::current().unwrap();
            let committed = resume_retained_restart(&installation, key, &layout, &migration)
                .unwrap()
                .commit_closed()
                .unwrap();
            assert_eq!(committed.binding().unwrap().generation, 2, "{barrier}");
            let snapshot = committed.snapshot().unwrap();
            assert!(snapshot.managed.fenced);
            assert_eq!(
                snapshot.managed.usage.cache_disk_bytes,
                (8 << 20) + (32 << 10) + 8192
            );
            assert_eq!(snapshot.entries, 1);
            assert_eq!(fs::read(installation.join(OWNER)).unwrap(), original[0]);
            for (name, bytes) in [
                RETAINED_TELEMETRY_MARKER_FILE,
                TELEMETRY_RECORDS_FILE,
                TELEMETRY_RECORDS_KEY_FILE,
            ]
            .iter()
            .zip(original_files)
            {
                assert_eq!(fs::read(state.join(name)).unwrap(), bytes, "{barrier}");
            }
            let record = Record::decode(&fs::read(installation.join(STATE)).unwrap()).unwrap();
            assert_eq!(record.generation, 2);
            assert_eq!(record.key, committed.binding().unwrap().key);
            assert!(!installation.join(TRANSFER_INTENT).exists());
            assert!(!installation.join(TRANSFER_NEXT).exists());
        }
    }
    #[test]
    #[ignore = "actual closed metadata retained-stop/restart; no backend stop or native finality proof"]
    fn b2_retained_stop_keeps_nonzero_debt_and_restarts_next_generation() {
        use super::transition::CompiledRetainedMigration;
        let home = sealed_format_filesystem_fixture();
        let installation = home.path().join("installation");
        let migration = CompiledRetainedMigration::current().unwrap();
        let first = prepare_old(home.path())
            .adopt_closed()
            .unwrap()
            .prepare_restart(&migration)
            .unwrap()
            .commit_closed()
            .unwrap();
        let before = first.snapshot().unwrap().managed.usage;
        let record = fs::read(installation.join(STATE)).unwrap();
        let stopped = first
            .prepare_retained_stop()
            .unwrap()
            .commit_retained_stop()
            .unwrap();
        assert_eq!(stopped.snapshot().unwrap().managed.usage, before);
        assert!(stopped.snapshot().unwrap().managed.fenced);
        assert_eq!(fs::read(installation.join(STATE)).unwrap(), record);
        let binding = stopped.binding().unwrap();
        assert_eq!(binding.generation, 2);
        let catalog = CatalogV2::decode(
            &fs::read(installation.join(CATALOG)).unwrap(),
            Epoch {
                key: binding.key,
                generation: binding.generation,
            },
        )
        .unwrap();
        assert!(catalog.mode == CatalogMode::StoppedRetained);
        let resumed = stopped
            .prepare_restart(&migration)
            .unwrap()
            .commit_closed()
            .unwrap();
        assert_eq!(resumed.binding().unwrap().generation, 3);
        assert_eq!(resumed.snapshot().unwrap().managed.usage, before);
        assert!(resumed.snapshot().unwrap().managed.fenced);
    }
    #[test]
    #[ignore = "actual transfer intent identity/privacy refusal; no native finality"]
    fn b2_pending_intent_faults_retain_original_locks_and_pins() {
        use super::transition::{CompiledRetainedMigration, resume_retained_restart};
        for fault in ["corrupt", "partial", "replaced"] {
            let home = sealed_format_filesystem_fixture();
            let installation = home.path().join("installation");
            let status=std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact","installed_owner::retained_telemetry::recovery::tests::b2_transaction_crash_child","--test-threads=1"])
                .env("SPIRALCORE_B2_HOME",home.path()).env("SPIRALCORE_B2_CRASH","intent").status().unwrap();
            assert_eq!(status.code(), Some(77));
            let intent = installation.join(TRANSFER_INTENT);
            if fault != "replaced" {
                let mut bytes = fs::read(&intent).unwrap();
                if fault == "corrupt" {
                    bytes[0] ^= 1;
                } else {
                    bytes.truncate(32);
                }
                fs::write(&intent, bytes).unwrap();
            }
            let original = File::open(&intent).unwrap();
            let original_id = object_id(&original).unwrap();
            let before: Vec<_> = [STATE, CATALOG, OWNER]
                .iter()
                .map(|name| fs::read(installation.join(name)).unwrap())
                .collect();
            let layout_path = home.path().join("vms");
            let layout = CanonicalRecoveryLayout::from_trusted_parent(
                &layout_path,
                File::open(&layout_path).unwrap(),
            )
            .unwrap();
            let key = InstallationKey {
                installation: [121; 32],
                profile: RetainedRecoveryCompatibility::published_v1()
                    .unwrap()
                    .profile,
            };
            let migration = CompiledRetainedMigration::current().unwrap();
            let failure = if fault == "replaced" {
                let prepared =
                    resume_retained_restart(&installation, key, &layout, &migration).unwrap();
                assert!(
                    prepared.ordinary_reserve_refused(),
                    "closed observer admitted an ordinary reservation"
                );
                fs::rename(&intent, installation.join("original-transfer-evidence")).unwrap();
                use std::io::Write;
                use std::os::unix::fs::OpenOptionsExt;
                let mut other = File::options()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&intent)
                    .unwrap();
                other
                    .write_all(&fs::read(installation.join("original-transfer-evidence")).unwrap())
                    .unwrap();
                other.sync_all().unwrap();
                match prepared.commit_closed() {
                    Ok(_) => panic!("replaced intent admitted"),
                    Err(failure) => failure,
                }
            } else {
                match resume_retained_restart(&installation, key, &layout, &migration) {
                    Ok(_) => panic!("malformed intent admitted"),
                    Err(failure) => failure,
                }
            };
            assert_eq!(
                failure.observed_intent_id(),
                (original_id.device, original_id.inode)
            );
            for (name, bytes) in [STATE, CATALOG, OWNER].iter().zip(before) {
                assert_eq!(fs::read(installation.join(name)).unwrap(), bytes, "{fault}");
            }
            let probe = File::options()
                .read(true)
                .write(true)
                .open(installation.join(HOLDERS))
                .unwrap();
            assert!(
                matches!(probe.try_lock(), Err(fs::TryLockError::WouldBlock)),
                "{fault}"
            );
            drop(failure);
            assert!(
                matches!(
                    InstallationOwner::open(&installation, key),
                    Err(OwnerError::Busy)
                ),
                "{fault}"
            );
            assert!(
                matches!(probe.try_lock(), Err(fs::TryLockError::WouldBlock)),
                "{fault}"
            );
        }
    }
    #[test]
    #[ignore = "actual current-source V2 process exit/reopen; metadata only, no native admission"]
    fn b2_current_source_restart_adopts_all_historical_rows_without_new_metadata_charge() {
        use super::transition::CompiledRetainedMigration;
        let home = sealed_format_filesystem_fixture();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "installed_owner::retained_telemetry::recovery::tests::b2_transaction_crash_child",
                "--test-threads=1",
            ])
            .env("SPIRALCORE_B2_HOME", home.path())
            .env("SPIRALCORE_B2_CRASH", "committed")
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(78));
        let current = RetainedRecoveryCompatibility::current().unwrap();
        let installation = home.path().join("installation");
        let key = InstallationKey {
            installation: [121; 32],
            profile: current.profile,
        };
        let owner = InstallationOwner::open(&installation, key).unwrap();
        let layout_path = home.path().join("vms");
        let layout = CanonicalRecoveryLayout::from_trusted_parent(
            &layout_path,
            File::open(&layout_path).unwrap(),
        )
        .unwrap();
        let closed = owner
            .prepare_retained_recovery(&current, &layout)
            .unwrap()
            .adopt_closed()
            .unwrap();
        let held = closed.recovery.held.as_ref().unwrap();
        assert_eq!(held.epoch.generation, 2);
        assert_eq!(held.entries[0].as_ref().unwrap().origin.generation, 1);
        assert_eq!(
            held.entries[0].as_ref().unwrap().origin.key.profile,
            RetainedRecoveryCompatibility::published_v1()
                .unwrap()
                .profile
        );
        let usage = closed.snapshot().unwrap().managed.usage;
        assert_eq!(usage.cache_disk_bytes, (8 << 20) + (32 << 10) + 8192);
        assert_eq!(usage.host_metadata_bytes, 1 << 20);
        let migration = CompiledRetainedMigration::current().unwrap();
        let committed = closed
            .prepare_restart(&migration)
            .unwrap()
            .commit_closed()
            .unwrap();
        assert_eq!(committed.binding().unwrap().generation, 3);
        assert_eq!(committed.snapshot().unwrap().managed.usage, usage);
        assert!(committed.snapshot().unwrap().managed.fenced);
    }
    fn review_private_file(path: &Path, bytes: &[u8]) -> File {
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let mut file = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
        file
    }
    pub(super) fn review_prepare(home: &Path) -> super::transition::PreparedRetainedRestart {
        let migration = super::transition::CompiledRetainedMigration::current().unwrap();
        prepare_old(home)
            .adopt_closed()
            .unwrap()
            .prepare_restart(&migration)
            .unwrap()
    }
    fn review_fresh_intent(identical: bool) {
        let home = sealed_format_filesystem_fixture();
        let installation = home.path().join("installation");
        let prepared = review_prepare(home.path());
        let expected = prepared.prepared_intent_bytes();
        let bytes = if identical {
            expected.as_slice()
        } else {
            b"unknown original immutable intent"
        };
        let unknown = review_private_file(&installation.join(TRANSFER_INTENT), bytes);
        let id = object_id(&unknown).unwrap();
        let before: Vec<_> = [STATE, CATALOG, OWNER]
            .iter()
            .map(|name| fs::read(installation.join(name)).unwrap())
            .collect();
        let result = prepared.commit_closed();
        let after: Vec<_> = [STATE, CATALOG, OWNER]
            .iter()
            .map(|name| fs::read(installation.join(name)).unwrap())
            .collect();
        eprintln!(
            "fresh canonical intent: refused={} source_unchanged={} inode_unchanged={} bytes_unchanged={}",
            result.is_err(),
            before == after,
            File::open(installation.join(TRANSFER_INTENT))
                .ok()
                .and_then(|file| object_id(&file).ok())
                .is_some_and(|named| named == id),
            fs::read(installation.join(TRANSFER_INTENT)).ok().as_deref() == Some(bytes)
        );
        assert!(
            result.is_err(),
            "fresh commit accepted a canonical intent introduced after preparation"
        );
        assert_eq!(before, after);
        let named = File::open(installation.join(TRANSFER_INTENT)).unwrap();
        assert_eq!(object_id(&named).unwrap(), id);
        assert_eq!(fs::read(installation.join(TRANSFER_INTENT)).unwrap(), bytes);
        drop(result);
        let holders = File::options()
            .read(true)
            .write(true)
            .open(installation.join(HOLDERS))
            .unwrap();
        assert!(matches!(
            holders.try_lock(),
            Err(fs::TryLockError::WouldBlock)
        ));
    }
    #[test]
    #[ignore = "real original-process filesystem fixture; unknown intent preservation"]
    fn b2_review_fresh_commit_refuses_unknown_canonical_intent() {
        review_fresh_intent(false);
    }
    #[test]
    #[ignore = "real original-process filesystem fixture; identical bytes are not replacement authority"]
    fn b2_review_fresh_commit_refuses_identical_canonical_intent() {
        review_fresh_intent(true);
    }

    fn review_resume_excluded(name: &str, directory: bool) {
        let home = sealed_format_filesystem_fixture();
        let installation = home.path().join("installation");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "installed_owner::retained_telemetry::recovery::tests::b2_transaction_crash_child",
                "--test-threads=1",
            ])
            .env("SPIRALCORE_B2_HOME", home.path())
            .env("SPIRALCORE_B2_CRASH", "intent")
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(77));
        let unknown = installation.join(name);
        let unknown_file = if directory {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new().mode(0o700).create(&unknown).unwrap();
            review_private_file(&unknown.join("payload"), b"untracked disposable bytes");
            File::open(&unknown).unwrap()
        } else {
            review_private_file(&unknown, b"unknown excluded metadata")
        };
        let unknown_id = object_id(&unknown_file).unwrap();
        let contents = fs::read(if directory {
            unknown.join("payload")
        } else {
            unknown.clone()
        })
        .unwrap();
        let before: Vec<_> = [STATE, CATALOG, OWNER, TRANSFER_INTENT]
            .iter()
            .map(|name| fs::read(installation.join(name)).unwrap())
            .collect();
        let layout_path = home.path().join("vms");
        let layout = CanonicalRecoveryLayout::from_trusted_parent(
            &layout_path,
            File::open(&layout_path).unwrap(),
        )
        .unwrap();
        let key = InstallationKey {
            installation: [121; 32],
            profile: RetainedRecoveryCompatibility::published_v1()
                .unwrap()
                .profile,
        };
        let migration = super::transition::CompiledRetainedMigration::current().unwrap();
        let result =
            super::transition::resume_retained_restart(&installation, key, &layout, &migration)
                .and_then(|prepared| prepared.commit_closed());
        let after: Vec<_> = [STATE, CATALOG, OWNER, TRANSFER_INTENT]
            .iter()
            .map(|name| fs::read(installation.join(name)).unwrap_or_default())
            .collect();
        eprintln!(
            "excluded resume {name}: refused={} source_unchanged={}",
            result.is_err(),
            before == after
        );
        assert!(
            before == after,
            "resume mutated source state/catalog before excluded-namespace refusal"
        );
        assert!(result.is_err(), "resume accepted an excluded namespace");
        assert_eq!(
            object_id(&File::open(&unknown).unwrap()).unwrap(),
            unknown_id
        );
        assert_eq!(
            fs::read(if directory {
                unknown.join("payload")
            } else {
                unknown
            })
            .unwrap(),
            contents
        );
        drop(result);
        assert!(matches!(
            InstallationOwner::open(&installation, key),
            Err(OwnerError::Busy)
        ));
        let holders = File::options()
            .read(true)
            .write(true)
            .open(installation.join(HOLDERS))
            .unwrap();
        assert!(matches!(
            holders.try_lock(),
            Err(fs::TryLockError::WouldBlock)
        ));
    }
    #[test]
    #[ignore = "actual interrupted process + unknown retained catalog intent"]
    fn b2_review_resume_refuses_catalog_intent_before_effects() {
        review_resume_excluded(CATALOG_INTENT, false);
    }
    #[test]
    #[ignore = "actual interrupted process + unaccounted disposable inventory"]
    fn b2_review_resume_refuses_disposable_inventory_before_effects() {
        review_resume_excluded(".endpoint-disks.bin", false);
    }
    #[test]
    #[ignore = "actual interrupted process + unaccounted disposable namespace"]
    fn b2_review_resume_refuses_disposable_namespace_before_effects() {
        review_resume_excluded(".endpoint-cache-v1-unknown", true);
    }

    #[test]
    #[cfg(target_os = "linux")]
    #[ignore = "explicit cfg-test post-state-intent fault; current-process FD observation only"]
    fn b2_review_late_state_refusal_retains_actual_candidate_fd() {
        use std::os::unix::fs::MetadataExt;
        let home = sealed_format_filesystem_fixture();
        let installation = home.path().join("installation");
        let prepared = review_prepare(home.path());
        LATE_STATE_REPLACEMENT.with(|slot| *slot.borrow_mut() = Some(installation.join(STATE)));
        let result = prepared.commit_closed();
        assert!(result.is_err());
        let id = LATE_STATE_ID
            .with(|id| id.take())
            .expect("explicit mutation hook not reached");
        let held = fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| fs::metadata(entry.path()).ok())
            .any(|meta| (meta.dev(), meta.ino()) == id);
        assert!(
            held,
            "late malformed STATE candidate FD was dropped on refusal"
        );
        drop(result);
        let held = fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| fs::metadata(entry.path()).ok())
            .any(|meta| (meta.dev(), meta.ino()) == id);
        assert!(
            held,
            "failure Drop released the actual late candidate observation"
        );
    }
    #[test]
    #[ignore = "deterministic cfg-test collision immediately before immutable publication syscall"]
    fn b2_review_atomic_publication_refuses_collision_after_absence_check() {
        let home = sealed_format_filesystem_fixture();
        let installation = home.path().join("installation");
        let prepared = review_prepare(home.path());
        let before: Vec<_> = [STATE, CATALOG, OWNER]
            .iter()
            .map(|name| fs::read(installation.join(name)).unwrap())
            .collect();
        PUBLISH_INTENT_COLLISION
            .with(|slot| *slot.borrow_mut() = Some(installation.join(TRANSFER_INTENT)));
        let result = prepared.commit_closed();
        assert!(
            PUBLISH_INTENT_COLLISION.with(|slot| slot.borrow().is_none()),
            "explicit final-boundary hook was not reached"
        );
        let after: Vec<_> = [STATE, CATALOG, OWNER]
            .iter()
            .map(|name| fs::read(installation.join(name)).unwrap())
            .collect();
        eprintln!(
            "final-boundary collision: refused={} source_unchanged={} canonical_preserved={}",
            result.is_err(),
            before == after,
            fs::read(installation.join(TRANSFER_INTENT)).ok().as_deref()
                == Some(b"explicit before-publication collision")
        );
        assert!(
            result.is_err(),
            "overwriting publication accepted final-boundary collision"
        );
        assert_eq!(before, after);
        assert_eq!(
            fs::read(installation.join(TRANSFER_INTENT)).unwrap(),
            b"explicit before-publication collision"
        );
    }
    #[test]
    #[ignore = "actual V2 filesystem publisher boundary; format fixture, no native finality"]
    fn b2_history_normal_v1_publisher_cannot_erase_origin_lineage() {
        let home = sealed_format_filesystem_fixture();
        let installation = home.path().join("installation");
        let committed = review_prepare(home.path()).commit_closed().unwrap();
        let before = fs::read(installation.join(CATALOG)).unwrap();
        let beacon = fs::read(installation.join(OWNER)).unwrap();
        assert_eq!(&before[..8], b"SCRETN02");
        let result = committed.attempt_v1_publication_for_format_regression();
        let after = fs::read(installation.join(CATALOG)).unwrap();
        eprintln!(
            "V2 ordinary publisher downgrade: refused={} unchanged={} before_magic={:?} after_magic={:?}",
            result.is_err(),
            before == after,
            &before[..8],
            &after[..8]
        );
        assert!(
            result.is_err(),
            "ordinary V1 publisher erased immutable historical origins"
        );
        assert!(before == after);
        assert_eq!(fs::read(installation.join(OWNER)).unwrap(), beacon);
    }
}
