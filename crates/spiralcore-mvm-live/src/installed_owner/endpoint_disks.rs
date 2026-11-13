//! Durable bookkeeping for this installation's disposable endpoint namespaces.
//! Projections in this file are not resource or MVM authorization.
use super::*;
use std::ffi::CStr;

pub(crate) const INVENTORY: &str = ".endpoint-disks.bin";
const INTENT: &str = ".endpoint-disks.intent";
const NEXT: &str = ".endpoint-disks.next";
const MAGIC: &[u8; 8] = b"SCDISK01";
const PREFIX: &str = ".endpoint-cache-v1-";
const DOMAIN: &[u8] = b"spiralcore/disposable-endpoint-inventory/v1\0";
const MAX_RECORDS: usize = 16;
const MAX_BYTES: usize = 64 << 10;
const MAX_NODES: usize = 4096;
const HEADER: usize = 84;
const ENTRY: usize = 120;

#[derive(Clone, Copy, Debug)]
pub struct EndpointDiskRequest {
    pub tenant: [u8; 32],
    pub session: [u8; 32],
    pub parent_serial: u64,
    pub cache_bytes: u64,
    pub staging_bytes: u64,
}
/// Borrowed bookkeeping facts. These cannot create an owner or authorize I/O.
pub struct EndpointDiskBinding<'a> {
    pub key: InstallationKey,
    pub generation: u64,
    pub tenant: [u8; 32],
    pub session: [u8; 32],
    pub parent_serial: u64,
    pub reservation_serial: u64,
    pub cache_bytes: u64,
    pub staging_bytes: u64,
    pub path: &'a Path,
    pub device: u64,
    pub inode: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    tenant: [u8; 32],
    session: [u8; 32],
    parent: u64,
    serial: u64,
    cache: u64,
    stage: u64,
    phase: u8,
    device: u64,
    inode: u64,
}
impl Entry {
    fn name(&self, epoch: u64) -> String {
        let session: String = self.session.iter().map(|b| format!("{b:02x}")).collect();
        format!("{PREFIX}{epoch:016x}-{:016x}-{session}", self.serial)
    }
    fn valid(&self) -> bool {
        self.tenant != [0; 32]
            && self.session != [0; 32]
            && self.parent != 0
            && self.serial != 0
            && self.cache <= 1024 << 20
            && self.stage <= 512 << 20
            && self.cache.checked_add(self.stage).is_some_and(|n| n != 0)
            && match self.phase {
                1 => self.device == 0 && self.inode == 0,
                2 | 3 => self.device != 0 && self.inode != 0,
                _ => false,
            }
    }
}
struct Snapshot {
    key: InstallationKey,
    generation: u64,
    entries: Vec<Entry>,
}
impl Snapshot {
    fn encode(&self) -> Result<Vec<u8>> {
        if self.entries.len() > MAX_RECORDS || self.generation == 0 {
            return Err(OwnerError::Fenced);
        }
        let mut bytes = Vec::with_capacity(HEADER + ENTRY * self.entries.len() + 32);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&self.key.installation);
        bytes.extend_from_slice(&self.key.profile);
        bytes.extend_from_slice(&self.generation.to_le_bytes());
        bytes.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&[0; 2]);
        let (mut cache, mut stage) = (0_u64, 0_u64);
        for (i, e) in self.entries.iter().enumerate() {
            if !e.valid()
                || self.entries[..i]
                    .iter()
                    .any(|p| p.serial == e.serial || p.session == e.session)
            {
                return Err(OwnerError::Fenced);
            }
            cache = cache.checked_add(e.cache).ok_or(OwnerError::Fenced)?;
            stage = stage.checked_add(e.stage).ok_or(OwnerError::Fenced)?;
            if cache > 1024 << 20 || stage > 512 << 20 {
                return Err(OwnerError::Fenced);
            }
            bytes.extend_from_slice(&e.tenant);
            bytes.extend_from_slice(&e.session);
            for n in [e.parent, e.serial, e.cache, e.stage] {
                bytes.extend_from_slice(&n.to_le_bytes());
            }
            bytes.push(e.phase);
            bytes.extend_from_slice(&[0; 7]);
            bytes.extend_from_slice(&e.device.to_le_bytes());
            bytes.extend_from_slice(&e.inode.to_le_bytes());
        }
        let mut hash = Sha256::new();
        hash.update(DOMAIN);
        hash.update(&bytes);
        bytes.extend_from_slice(&hash.finalize());
        Ok(bytes)
    }
    fn decode(bytes: &[u8], key: InstallationKey, generation: u64) -> Result<Self> {
        if bytes.len() < HEADER + 32 || bytes.len() > MAX_BYTES || &bytes[..8] != MAGIC {
            return Err(OwnerError::Fenced);
        }
        let count = u16::from_le_bytes(
            bytes[80..82]
                .try_into()
                .expect("fixed-width inventory field"),
        ) as usize;
        if count > MAX_RECORDS
            || bytes.len() != HEADER + count * ENTRY + 32
            || bytes[82..84] != [0; 2]
        {
            return Err(OwnerError::Fenced);
        }
        let mut hash = Sha256::new();
        hash.update(DOMAIN);
        hash.update(&bytes[..bytes.len() - 32]);
        if hash.finalize().as_slice() != &bytes[bytes.len() - 32..] {
            return Err(OwnerError::Fenced);
        }
        let actual = InstallationKey {
            installation: bytes[8..40]
                .try_into()
                .expect("fixed-width inventory field"),
            profile: bytes[40..72]
                .try_into()
                .expect("fixed-width inventory field"),
        };
        if actual != key
            || u64::from_le_bytes(
                bytes[72..80]
                    .try_into()
                    .expect("fixed-width inventory field"),
            ) != generation
        {
            return Err(OwnerError::StaleEpoch);
        }
        let mut entries = Vec::with_capacity(count);
        for raw in bytes[HEADER..bytes.len() - 32].chunks_exact(ENTRY) {
            let number = |at| {
                u64::from_le_bytes(
                    raw[at..at + 8]
                        .try_into()
                        .expect("fixed-width inventory field"),
                )
            };
            if raw[97..104] != [0; 7] {
                return Err(OwnerError::Fenced);
            }
            entries.push(Entry {
                tenant: raw[..32].try_into().expect("fixed-width inventory field"),
                session: raw[32..64].try_into().expect("fixed-width inventory field"),
                parent: number(64),
                serial: number(72),
                cache: number(80),
                stage: number(88),
                phase: raw[96],
                device: number(104),
                inode: number(112),
            });
        }
        let result = Self {
            key,
            generation,
            entries,
        };
        if result.encode()? != bytes {
            return Err(OwnerError::Fenced);
        }
        Ok(result)
    }
}

pub(crate) struct DiskStore {
    pub(crate) path: PathBuf,
    directory: File,
    entry: Entry,
    epoch: Epoch,
    retirement: RetirementProgress,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum RetirementProgress {
    Live,
    NamespaceUnlinked,
    NamespaceSynced,
    Complete,
}
pub(crate) struct DiskPrepareFailure {
    pub(crate) error: OwnerError,
    pub(crate) may_have_effects: bool,
}
impl DiskStore {
    pub(crate) fn binding(&self) -> EndpointDiskBinding<'_> {
        EndpointDiskBinding {
            key: self.epoch.key,
            generation: self.epoch.generation,
            tenant: self.entry.tenant,
            session: self.entry.session,
            parent_serial: self.entry.parent,
            reservation_serial: self.entry.serial,
            cache_bytes: self.entry.cache,
            staging_bytes: self.entry.stage,
            path: &self.path,
            device: self.entry.device,
            inode: self.entry.inode,
        }
    }
}
fn unlink(dir: &File, name: &str, directory: bool) -> Result<()> {
    let name = CString::new(name).map_err(|_| OwnerError::InvalidStorage)?;
    // SAFETY: owned live parent descriptor, checked one-component name.
    if unsafe {
        libc::unlinkat(
            dir.as_raw_fd(),
            name.as_ptr(),
            if directory { libc::AT_REMOVEDIR } else { 0 },
        )
    } != 0
    {
        return Err(io::Error::last_os_error().into());
    }
    Ok(())
}
fn open_directory(dir: &File, name: &str) -> Result<File> {
    let name = CString::new(name).map_err(|_| OwnerError::InvalidStorage)?;
    // SAFETY: live parent/name; NOFOLLOW prevents traversing a replacement link.
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
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
    // SAFETY: openat returned a fresh owned descriptor.
    let file = unsafe { File::from_raw_fd(fd) };
    check_private(&file, true)?;
    Ok(file)
}
fn scan_limits(block_hint: Option<u64>, union_mount: bool) -> Result<()> {
    if union_mount || block_hint.is_some_and(|hint| hint == 0 || hint > 32768) {
        return Err(OwnerError::InvalidStorage);
    }
    Ok(())
}
fn visit_names(dir: &File, max: usize, mut visit: impl FnMut(&str) -> Result<()>) -> Result<()> {
    #[cfg(target_os = "linux")]
    let block_hint = Some(dir.metadata()?.blksize());
    #[cfg(not(target_os = "linux"))]
    let block_hint = None;
    #[cfg(target_os = "macos")]
    let union_mount = {
        let mut info = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: retained live directory; fstatfs initializes info on success.
        if unsafe { libc::fstatfs(dir.as_raw_fd(), info.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: fstatfs succeeded. Union directories can preload all names.
        unsafe { info.assume_init() }.f_flags & libc::MNT_UNION as u32 != 0
    };
    #[cfg(not(target_os = "macos"))]
    let union_mount = false;
    // Enforce the qualified directory-buffer shape BEFORE libc can allocate.
    scan_limits(block_hint, union_mount)?;
    // SAFETY: duplicate only this live directory, retaining CLOEXEC.
    let fd = unsafe { libc::fcntl(dir.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: fdopendir assumes ownership on success.
    let ptr = unsafe { libc::fdopendir(fd) };
    if ptr.is_null() {
        let error = io::Error::last_os_error();
        // SAFETY: fdopendir failed, leaving ownership with this call.
        unsafe { libc::close(fd) };
        return Err(error.into());
    }
    struct Directory(*mut libc::DIR);
    impl Drop for Directory {
        fn drop(&mut self) {
            unsafe { libc::closedir(self.0) };
        }
    }
    let guard = Directory(ptr);
    // The duplicated open description shares its offset; all callers hold the
    // original owner transaction mutex, and no other operation enumerates it.
    unsafe { libc::rewinddir(guard.0) };
    let mut count = 0;
    loop {
        #[cfg(target_os = "linux")]
        unsafe {
            *libc::__errno_location() = 0;
        }
        #[cfg(target_os = "macos")]
        unsafe {
            *libc::__error() = 0;
        }
        // SAFETY: live DIR; copy d_name before the next readdir call.
        let item = unsafe { libc::readdir(guard.0) };
        if item.is_null() {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(0) {
                return Err(error.into());
            }
            break;
        }
        let raw = unsafe { CStr::from_ptr((*item).d_name.as_ptr()) }.to_bytes();
        if raw == b"." || raw == b".." {
            continue;
        }
        if count == max || raw.is_empty() || raw.len() > 192 {
            return Err(OwnerError::Fenced);
        }
        count += 1;
        visit(std::str::from_utf8(raw).map_err(|_| OwnerError::Fenced)?)?;
    }
    Ok(())
}

/// A generated namespace without metadata is still an unresolved disk charge.
pub(super) fn no_namespaces(directory: &File) -> Result<()> {
    visit_names(directory, 128, |name| {
        if name.starts_with(PREFIX) {
            return Err(OwnerError::Fenced);
        }
        Ok(())
    })
}

fn remove_contents(dir: &File, device: u64, depth: usize, work: &mut usize) -> Result<()> {
    if depth > 3 {
        return Err(OwnerError::Fenced);
    }
    visit_names(dir, MAX_NODES.saturating_sub(*work), |name| {
        *work = work.checked_add(1).ok_or(OwnerError::Fenced)?;
        if *work > MAX_NODES {
            return Err(OwnerError::Fenced);
        }
        let c = CString::new(name).map_err(|_| OwnerError::InvalidStorage)?;
        let mut raw = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: live parent and one checked component; initialized on success.
        if unsafe {
            libc::fstatat(
                dir.as_raw_fd(),
                c.as_ptr(),
                raw.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        let raw = unsafe { raw.assume_init() };
        #[cfg(target_os = "linux")]
        let observed_device = raw.st_dev;
        #[cfg(not(target_os = "linux"))]
        let observed_device = u64::try_from(raw.st_dev).map_err(|_| OwnerError::InvalidStorage)?;
        if observed_device != device {
            return Err(OwnerError::InvalidStorage);
        }
        let directory = match raw.st_mode & libc::S_IFMT {
            libc::S_IFDIR => true,
            libc::S_IFREG => false,
            _ => return Err(OwnerError::InvalidStorage),
        };
        let held = if directory {
            open_directory(dir, name)?
        } else {
            open_at(dir, name, libc::O_RDONLY)?
        };
        let id = object_id(&held)?;
        if id.device != observed_device || id.inode != raw.st_ino {
            return Err(OwnerError::InvalidStorage);
        }
        if directory {
            remove_contents(&held, device, depth + 1, work)?;
        }
        unlink(dir, name, directory)?;
        Ok(())
    })?;
    dir.sync_all()?;
    Ok(())
}
impl Storage {
    fn disk_snapshot(&self, epoch: Epoch) -> Result<Snapshot> {
        if !self.name_present(INVENTORY)? {
            return Ok(Snapshot {
                key: epoch.key,
                generation: epoch.generation,
                entries: Vec::new(),
            });
        }
        match open_at(&self.directory, INVENTORY, libc::O_RDONLY) {
            Ok(file) => {
                if file.metadata()?.len() > MAX_BYTES as u64 {
                    return Err(OwnerError::Fenced);
                }
                let mut bytes = Vec::with_capacity(4096);
                file.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
                Snapshot::decode(&bytes, epoch.key, epoch.generation)
            }
            Err(e) => Err(e),
        }
    }
    fn publish_disks(&self, snapshot: &Snapshot) -> Result<()> {
        self.check()?;
        self.absent_names(&[INTENT, NEXT])?;
        let bytes = snapshot.encode()?;
        let mut file = open_at(
            &self.directory,
            INTENT,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        file.write_all(&bytes)?;
        #[cfg(test)]
        tests::checkpoint("intent-file-written")?;
        file.sync_all()?;
        #[cfg(test)]
        tests::checkpoint("intent-file-synced")?;
        let from = CString::new(INTENT).expect("static intent name has no NUL");
        let to = CString::new(INVENTORY).expect("static inventory name has no NUL");
        self.check()?;
        // SAFETY: fixed names on the same original validated parent.
        if unsafe {
            libc::renameat(
                self.directory.as_raw_fd(),
                from.as_ptr(),
                self.directory.as_raw_fd(),
                to.as_ptr(),
            )
        } != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        #[cfg(test)]
        tests::checkpoint("publish-before-parent-sync")?;
        self.directory.sync_all()?;
        if snapshot.entries.is_empty() {
            #[cfg(test)]
            tests::checkpoint("inventory-empty-committed")?;
            unlink(&self.directory, INVENTORY, false)?;
            #[cfg(test)]
            tests::checkpoint("inventory-removed")?;
            self.directory.sync_all()?;
        }
        Ok(())
    }
}
impl InstallationOwner {
    /// Recover only recorded disposable endpoint caches after original kernel
    /// holder exclusion. This does not retire the installation or audit data.
    pub fn recover_endpoint_disks(&mut self) -> Result<usize> {
        let current = self.current()?;
        let _holders = self.holders_for_transition()?;
        self.storage.absent_names(&[NEXT])?;
        let epoch = Epoch {
            key: current.key,
            generation: current.generation,
        };
        let mut snapshot = self.storage.disk_snapshot(epoch)?;
        let allowed: Vec<_> = snapshot
            .entries
            .iter()
            .map(|e| e.name(epoch.generation))
            .collect();
        // A publisher's unrenamed intent cannot authorize directory effects.
        // Every actual directory must already belong to the committed snapshot.
        visit_names(&self.storage.directory, 128, |name| {
            if name.starts_with(PREFIX) && !allowed.iter().any(|item| item == name) {
                return Err(OwnerError::Fenced);
            }
            Ok(())
        })?;
        if self.storage.name_present(INTENT)? {
            let file = open_at(&self.storage.directory, INTENT, libc::O_RDONLY)?;
            if file.metadata()?.len() > MAX_BYTES as u64 {
                return Err(OwnerError::Fenced);
            }
            // The old committed snapshot governs effects until rename+sync.
            // A partial uncommitted intent is disposable only after the
            // kernel barrier and the no-untracked-namespace check above.
            unlink(&self.storage.directory, INTENT, false)?;
            self.storage.directory.sync_all()?;
        }
        let mut recovered = 0;
        let device = self.storage.directory.metadata()?.dev();
        while !snapshot.entries.is_empty() {
            let e = snapshot.entries[0];
            let name = e.name(epoch.generation);
            if !self.storage.name_present(&name)? {
                if e.phase == 2 {
                    return Err(OwnerError::Fenced);
                }
                // Creation never reached mkdir, or a durable CleanupIntent
                // already authorized removal. Persist the observed absence.
                self.storage.directory.sync_all()?;
            } else {
                // Creation intent carries no original inode. Even a private
                // directory at this name is ambiguous after an interrupted mkdir.
                if e.phase == 1 {
                    return Err(OwnerError::Fenced);
                }
                let directory = open_directory(&self.storage.directory, &name)?;
                let id = object_id(&directory)?;
                if id.device != device
                    || (e.phase != 1 && (id.device != e.device || id.inode != e.inode))
                {
                    return Err(OwnerError::InvalidStorage);
                }
                snapshot.entries[0] = Entry {
                    phase: 3,
                    device: id.device,
                    inode: id.inode,
                    ..e
                };
                self.storage.publish_disks(&snapshot)?;
                let mut work = 0;
                remove_contents(&directory, device, 0, &mut work)?;
                // Recheck the exact root name before removing the namespace.
                if object_id(&open_directory(&self.storage.directory, &name)?)? != id {
                    return Err(OwnerError::InvalidStorage);
                }
                unlink(&self.storage.directory, &name, true)?;
                self.storage.directory.sync_all()?;
            }
            snapshot.entries.remove(0);
            self.storage.publish_disks(&snapshot)?;
            recovered += 1;
        }
        // A crash after publishing an empty snapshot but before unlink is also
        // a recoverable metadata-only state, with no untracked namespaces.
        if self.storage.name_present(INVENTORY)? {
            self.storage.publish_disks(&snapshot)?;
        }
        Ok(recovered)
    }
    fn disk_epoch(&self, epoch: Epoch) -> Result<()> {
        let current = self.current()?;
        if current.key != epoch.key
            || current.generation != epoch.generation
            || current.phase != Phase::Active
        {
            return Err(OwnerError::StaleEpoch);
        }
        Ok(())
    }
    pub(crate) fn check_endpoint_disk(&self, store: &DiskStore) -> Result<()> {
        self.disk_epoch(store.epoch)?;
        self.storage.absent_names(&[INTENT, NEXT])?;
        let directory = open_directory(
            &self.storage.directory,
            &store.entry.name(store.epoch.generation),
        )?;
        if store.retirement != RetirementProgress::Live
            || object_id(&directory)?
                != (ObjectId {
                    device: store.entry.device,
                    inode: store.entry.inode,
                })
            || object_id(&store.directory)? != object_id(&directory)?
            || !self
                .storage
                .disk_snapshot(store.epoch)?
                .entries
                .contains(&store.entry)
        {
            return Err(OwnerError::Fenced);
        }
        Ok(())
    }

    pub(crate) fn prepare_endpoint_disk(
        &mut self,
        epoch: Epoch,
        serial: u64,
        request: EndpointDiskRequest,
    ) -> std::result::Result<DiskStore, DiskPrepareFailure> {
        let mut may_have_effects = false;
        let result = (|| -> Result<DiskStore> {
            self.disk_epoch(epoch)?;
            self.storage.absent_names(&[INTENT, NEXT])?;
            let mut snapshot = self.storage.disk_snapshot(epoch)?;
            let entry = Entry {
                tenant: request.tenant,
                session: request.session,
                parent: request.parent_serial,
                serial,
                cache: request.cache_bytes,
                stage: request.staging_bytes,
                phase: 1,
                device: 0,
                inode: 0,
            };
            if !entry.valid() || snapshot.entries.len() == MAX_RECORDS {
                return Err(OwnerError::Fenced);
            }
            let name = entry.name(epoch.generation);
            self.storage.absent_names(&[&name])?;
            snapshot.entries.push(entry);
            // Complete bounded pure validation before arming uncertainty. In
            // particular, duplicates must not burn a second actual reservation.
            snapshot.encode()?;
            may_have_effects = true;
            self.storage.publish_disks(&snapshot)?;
            #[cfg(test)]
            tests::checkpoint("creation-committed")?;
            let c = CString::new(name.as_str()).map_err(|_| OwnerError::InvalidStorage)?;
            // SAFETY: generated single component relative to original private root.
            if unsafe { libc::mkdirat(self.storage.directory.as_raw_fd(), c.as_ptr(), 0o700) } != 0
            {
                return Err(io::Error::last_os_error().into());
            }
            #[cfg(test)]
            tests::checkpoint("namespace-created")?;
            let directory = open_directory(&self.storage.directory, &name)?;
            directory.sync_all()?;
            self.storage.directory.sync_all()?;
            let id = object_id(&directory)?;
            let active = Entry {
                phase: 2,
                device: id.device,
                inode: id.inode,
                ..entry
            };
            *snapshot
                .entries
                .last_mut()
                .ok_or(OwnerError::InvalidStorage)? = active;
            self.storage.publish_disks(&snapshot)?;
            #[cfg(test)]
            tests::checkpoint("active-committed")?;
            Ok(DiskStore {
                path: self.storage.root.join(name),
                directory,
                entry: active,
                epoch,
                retirement: RetirementProgress::Live,
            })
        })();
        result.map_err(|error| DiskPrepareFailure {
            error,
            may_have_effects,
        })
    }
    pub(crate) fn cancel_unused_endpoint_disk(&mut self, store: &DiskStore) -> Result<()> {
        self.disk_epoch(store.epoch)?;
        self.storage.absent_names(&[INTENT, NEXT])?;
        let mut snapshot = self.storage.disk_snapshot(store.epoch)?;
        let position = snapshot
            .entries
            .iter()
            .position(|e| *e == store.entry)
            .ok_or(OwnerError::StaleEpoch)?;
        let name = store.entry.name(store.epoch.generation);
        let directory = open_directory(&self.storage.directory, &name)?;
        if object_id(&directory)? != object_id(&store.directory)? {
            return Err(OwnerError::Fenced);
        }
        // Refuse on the first entry, without collecting an uncharged name vector.
        visit_names(&directory, 1, |_| Err(OwnerError::Fenced))?;
        snapshot.entries[position].phase = 3;
        self.storage.publish_disks(&snapshot)?;
        #[cfg(test)]
        tests::checkpoint("cleanup-committed")?;
        unlink(&self.storage.directory, &name, true)?;
        #[cfg(test)]
        tests::checkpoint("remove-before-parent-sync")?;
        self.storage.directory.sync_all()?;
        #[cfg(test)]
        tests::checkpoint("namespace-removal-committed")?;
        snapshot.entries.remove(position);
        self.storage.publish_disks(&snapshot)
    }

    /// Filesystem primitive for the resident's sealed original-child retirement
    /// path. The caller must retain that receipt and the actual disk credit.
    /// This method is private to the trusted embedding; it creates no exit proof.
    pub(crate) fn retire_endpoint_disk(&mut self, store: &mut DiskStore) -> Result<()> {
        self.disk_epoch(store.epoch)?;
        if store.retirement == RetirementProgress::Complete {
            return Ok(());
        }
        self.storage.absent_names(&[INTENT, NEXT])?;
        let original = ObjectId {
            device: store.entry.device,
            inode: store.entry.inode,
        };
        if object_id(&store.directory)? != original {
            return Err(OwnerError::InvalidStorage);
        }
        let name = store.entry.name(store.epoch.generation);
        let mut snapshot = self.storage.disk_snapshot(store.epoch)?;
        let position = snapshot.entries.iter().position(|entry| {
            matches!(entry.phase, 2 | 3) && Entry { phase: 2, ..*entry } == store.entry
        });
        let Some(position) = position else {
            // A final publication may have renamed the empty snapshot before
            // failing its synchronization. Only this same store's positive
            // prior remove+sync can resolve a missing entry; absence alone can't.
            if store.retirement != RetirementProgress::NamespaceSynced
                || self.storage.name_present(&name)?
            {
                return Err(OwnerError::Fenced);
            }
            self.storage.publish_disks(&snapshot)?;
            store.retirement = RetirementProgress::Complete;
            return Ok(());
        };
        if self.storage.name_present(&name)? {
            if store.retirement != RetirementProgress::Live {
                return Err(OwnerError::Fenced);
            }
            let directory = open_directory(&self.storage.directory, &name)?;
            if object_id(&directory)? != original {
                return Err(OwnerError::InvalidStorage);
            }
            snapshot.entries[position].phase = 3;
            self.storage.publish_disks(&snapshot)?;
            let mut work = 0;
            remove_contents(&directory, original.device, 0, &mut work)?;
            if object_id(&open_directory(&self.storage.directory, &name)?)? != original {
                return Err(OwnerError::InvalidStorage);
            }
            unlink(&self.storage.directory, &name, true)?;
            store.retirement = RetirementProgress::NamespaceUnlinked;
            #[cfg(test)]
            tests::checkpoint("remove-before-parent-sync")?;
        } else if snapshot.entries[position].phase != 3
            || !matches!(
                store.retirement,
                RetirementProgress::NamespaceUnlinked | RetirementProgress::NamespaceSynced
            )
        {
            return Err(OwnerError::Fenced);
        }
        self.storage.directory.sync_all()?;
        store.retirement = RetirementProgress::NamespaceSynced;
        snapshot.entries.remove(position);
        self.storage.publish_disks(&snapshot)?;
        store.retirement = RetirementProgress::Complete;
        Ok(())
    }
}

#[cfg(test)]
#[path = "endpoint_disks_tests.rs"]
mod tests;
