//! Closed old-epoch debt custody. No issuer, deletion or epoch transition API.
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

type RecoveryResult<T> = std::result::Result<T, SupervisorError>;

/// Compiled compatibility, never a caller-supplied profile or limit grant.
pub(crate) struct RetainedRecoveryCompatibility {
    profile: [u8; 32],
}
impl RetainedRecoveryCompatibility {
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
                funding,
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
    funding: ClosedRetainedFunding,
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
            managed: held.funding.snapshot()?,
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
    fn observe_catalog(&mut self) -> RecoveryResult<(Vec<u8>, Catalog)> {
        self.funding.check()?;
        let authority = self.funding.authority();
        authority.with_disk_owner(|owner, epoch| {
            owner.active_binding(epoch)?;
            if epoch != self.epoch {
                return Err(OwnerError::StaleEpoch);
            }
            let mut beacon = Vec::with_capacity(112);
            beacon.extend_from_slice(b"SCRETB01");
            beacon.extend_from_slice(&epoch.key.installation);
            beacon.extend_from_slice(&epoch.key.profile);
            beacon.extend_from_slice(&epoch.generation.to_le_bytes());
            beacon.extend_from_slice(&Sha256::digest(&beacon));
            if bounded_bytes(&owner._owner, 112)? != beacon {
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
            let catalog = Catalog::decode(&bytes, epoch)?;
            if catalog.entries.is_empty() {
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
            tenants[at] = row.tenant;
        }
        self.funding.fund_rows(&tenants[..catalog.entries.len()])?;
        self.layout.check()?;
        self.protection = Some(RuntimeStateProtection::acquire(
            &self.layout.path,
            &self.layout.directory,
        )?);
        let protection = self.protection.as_ref().ok_or(SupervisorError::State)?;
        for (at, record) in catalog.entries.into_iter().enumerate() {
            let observed = &mut self.observations[at];
            Self::capture_state(&self.layout, protection, &record, observed)?;
            // This state has passed its original checks. Unqualified later
            // observations remain in a SEPARATE private fixed slot on errors.
            self.entries[at] = Some(PinnedEntry {
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
                self.epoch,
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
        let protection = self.protection.as_ref().ok_or(SupervisorError::State)?;
        for at in 0..MAX_ENTRIES {
            let Some(entry) = self.entries[at].as_ref() else {
                continue;
            };
            let fresh = &mut self.observations[at];
            Self::capture_state(&self.layout, protection, &entry.record, fresh)?;
            Self::capture_files(
                protection,
                self.epoch,
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
        self.funding.check()?;
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
            .funding
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
            .funding
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
        let snapshot = held.funding.snapshot().unwrap();
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
}
