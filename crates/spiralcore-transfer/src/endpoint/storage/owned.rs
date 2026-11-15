//! Directory custody for the admitted Linux/macOS SmallV1 store.
use crate::{endpoint::Limits, resource_profile::ResourceProfile, Error, Id, Result};

/// Exact namespace IDs derived from the currently authenticated lane contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnedLaneNamespaces {
    One(Id),
    Two(Id, Id),
}

/// Finite filesystem work funded by the caller for one original store.
#[derive(Clone, Debug)]
pub struct OwnedStorePolicy {
    lanes: OwnedLaneNamespaces,
    max_entries: usize,
    limits: Limits,
    requirement: OwnedWorkspaceRequirements,
    funding: OwnedWorkspaceFunding,
}

/// A declaration by the existing resource owner, not a new capacity grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnedWorkspaceFunding {
    pub heap_bytes: u64,
    pub existing_stack_bytes: u64,
}

/// Requested allocation layouts and the separately funded existing stack.
/// Allocator rounding/overhead require the original owner's enforced memory
/// envelope; these fields are not a universal malloc or RSS bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnedWorkspaceRequirements {
    pub directory_buffer_bytes: u64,
    pub fixed_store_layout_bytes: u64,
    pub ordered_index_bytes: u64,
    pub scan_record_bytes: u64,
    pub synchronization_bytes: u64,
    pub retained_allocation_count: u8,
    pub retained_requested_bytes: u64,
    pub peak_temporary_requested_bytes: u64,
    pub required_heap_bytes: u64,
    pub required_existing_stack_bytes: u64,
    pub original_owner_memory_enforcement_required: bool,
}

impl OwnedStorePolicy {
    pub fn workspace_requirements(
        limits: &Limits,
        lanes: OwnedLaneNamespaces,
    ) -> Result<OwnedWorkspaceRequirements> {
        if option_env!("SPIRALCORE_OWNED_STD_LAYOUT") != Some("qualified-v1") {
            return Err(Error::Unsupported);
        }
        Self::validate(limits, lanes)?;
        // Supported 64-bit std Arc header and alignment allowance. Physical
        // allocation effects remain under the original owner's memory ceiling.
        if std::mem::align_of::<Store>() > 16 {
            return Err(Error::Unsupported);
        }
        let fixed = u64::try_from(
            std::mem::size_of::<Store>()
                .checked_add(64)
                .ok_or(Error::Limit)?,
        )
        .map_err(|_| Error::Limit)?;
        let (index, scan) =
            crate::metadata_work::PreparedIndex::layout_bytes(limits.max_publications)?;
        let directory = Self::directory_buffer_bytes() as u64;
        // Pinned std uses three lazy boxed pthread mutexes on Darwin. Warm
        // them before publication so no later metadata operation allocates.
        #[cfg(target_os = "macos")]
        let synchronization = u64::try_from(
            3usize
                .checked_mul(std::mem::size_of::<libc::pthread_mutex_t>())
                .ok_or(Error::Limit)?,
        )
        .map_err(|_| Error::Limit)?;
        #[cfg(target_os = "linux")]
        let synchronization = 0u64;
        let retained = directory
            .checked_add(fixed)
            .and_then(|n| n.checked_add(index))
            .and_then(|n| n.checked_add(scan))
            .and_then(|n| n.checked_add(synchronization))
            .ok_or(Error::Limit)?;
        Ok(OwnedWorkspaceRequirements {
            directory_buffer_bytes: directory,
            fixed_store_layout_bytes: fixed,
            ordered_index_bytes: index,
            scan_record_bytes: scan,
            synchronization_bytes: synchronization,
            retained_allocation_count: 3
                + u8::from(index != 0)
                + u8::from(scan != 0)
                + if cfg!(target_os = "macos") { 3 } else { 0 },
            retained_requested_bytes: retained,
            peak_temporary_requested_bytes: 0,
            required_heap_bytes: retained,
            required_existing_stack_bytes: crate::metadata_work::EXISTING_STACK_BYTES,
            original_owner_memory_enforcement_required: true,
        })
    }
    pub fn workspace_requirement(&self) -> OwnedWorkspaceRequirements {
        self.requirement
    }
    pub fn workspace_funding(&self) -> OwnedWorkspaceFunding {
        self.funding
    }

    fn check_funding(&self) -> Result<()> {
        if self.funding.heap_bytes < self.requirement.required_heap_bytes
            || self.funding.existing_stack_bytes < self.requirement.required_existing_stack_bytes
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
    /// Retained requested allocation for the two directory buffers per store.
    /// Fund before capture, through final lease destruction. This excludes
    /// allocator overhead, fixed bookkeeping, stacks and kernel memory.
    pub const fn directory_buffer_bytes() -> usize {
        2 * work::BUFFER_BYTES
    }

    pub fn small_v1(
        limits: &Limits,
        funded_entry_visits: u16,
        lanes: OwnedLaneNamespaces,
        funding: OwnedWorkspaceFunding,
    ) -> Result<Self> {
        let requirement = Self::workspace_requirements(limits, lanes)?;
        let policy = Self {
            lanes,
            max_entries: 5usize
                .checked_add(limits.max_publications)
                .and_then(|n| n.checked_add(limits.max_states))
                .ok_or(Error::Limit)?,
            limits: limits.clone(),
            requirement,
            funding,
        };
        policy.check_funding()?;
        if usize::from(funded_entry_visits) <= policy.max_entries {
            return Err(Error::Limit);
        }
        Ok(policy)
    }

    fn validate(limits: &Limits, lanes: OwnedLaneNamespaces) -> Result<()> {
        ResourceProfile::SmallV1.check_limits(limits)?;
        if limits.max_active != 1
            || limits.max_nodes == 0
            || limits.max_states < limits.max_active
            || matches!(lanes, OwnedLaneNamespaces::Two(a,b) if a == b)
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

mod component;
mod io;
mod retire;
mod walk;
mod work;
pub(crate) use component::Component;
pub(crate) use io::{check_file_at, create_new_at, open_read_at, quota_len_at, unlink_checked_at};

use rustix::fs::{FileType, OFlags};
use std::{
    ffi::OsStr,
    fs::{File, TryLockError},
    io::{Read, Write},
    os::fd::{AsFd, BorrowedFd},
    sync::{atomic::Ordering, Arc, Mutex},
    time::{Duration, Instant},
};

const OWNER: &str = ".owner";
const PROFILE: &str = ".spiralcore-profile-v1";

#[derive(Clone)]
pub struct OwnedStoreLease(Arc<Store>);

struct Store {
    parent: Arc<File>,
    leaf: Component,
    child: Arc<File>,
    policy: OwnedStorePolicy,
    claim: Mutex<Claim>,
    work: Mutex<work::Buffers>,
    metadata: Mutex<Option<crate::metadata_work::PreparedIndex>>,
    #[cfg(test)]
    fail_publication_sync: std::sync::atomic::AtomicBool,
}

#[derive(Default)]
struct Claim {
    started: bool,
    owner: Option<File>,
    locked: bool,
    ready: bool,
    retired: bool,
}

fn initialized_mutex<T>(value: T) -> Result<Mutex<T>> {
    let mutex = Mutex::new(value);
    // Unshared and unlocked: initialize any platform backing allocation before
    // claim/recovery can have effects. Moving the unlocked std mutex is safe.
    drop(mutex.try_lock().map_err(|_| Error::Storage)?);
    Ok(mutex)
}

fn verify_profile_content(mut file: impl Read) -> Result<()> {
    let mut actual = [0; super::PROFILE_CONTENT.len() + 1];
    let mut used = 0;
    while used < actual.len() {
        match file.read(&mut actual[used..]) {
            Ok(0) => break,
            Ok(n) => used += n,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(Error::Storage),
        }
    }
    if &actual[..used] != super::PROFILE_CONTENT {
        return Err(Error::Storage);
    }
    Ok(())
}

impl OwnedStoreLease {
    /// The provider supplies an original private mount tree with no nested mounts.
    /// Capture checks custody and fallibly allocates the funded directory buffers.
    /// Attach this lease to the original permit before opening an endpoint.
    pub fn capture(
        parent: Arc<File>,
        leaf: &OsStr,
        child: Arc<File>,
        policy: OwnedStorePolicy,
    ) -> Result<Self> {
        policy.check_funding()?;
        let leaf = Component::new(leaf)?;
        io::private(&io::held(parent.as_fd())?, FileType::Directory)?;
        io::check_named(
            parent.as_fd(),
            leaf.as_os_str(),
            child.as_fd(),
            FileType::Directory,
        )?;
        let work = work::Buffers::new()?;
        let metadata = crate::metadata_work::PreparedIndex::new(policy.limits.max_publications)?;
        let claim = initialized_mutex(Claim::default())?;
        let work = initialized_mutex(work)?;
        let metadata = initialized_mutex(Some(metadata))?;
        Ok(Self(Arc::new(Store {
            parent,
            leaf,
            child,
            policy,
            claim,
            work,
            metadata,
            #[cfg(test)]
            fail_publication_sync: std::sync::atomic::AtomicBool::new(false),
        })))
    }

    #[cfg(test)]
    pub(crate) fn fail_next_publication_sync(&self) {
        self.0.fail_publication_sync.store(true, Ordering::Relaxed);
    }

    #[cfg(test)]
    fn check_injected_publication_sync(&self) -> Result<()> {
        if self.0.fail_publication_sync.swap(false, Ordering::Relaxed) {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    }

    fn root(&self) -> BorrowedFd<'_> {
        self.0.child.as_fd()
    }
    fn original(&self) -> Result<()> {
        io::private(&io::held(self.0.parent.as_fd())?, FileType::Directory)?;
        io::check_named(
            self.0.parent.as_fd(),
            self.0.leaf.as_os_str(),
            self.root(),
            FileType::Directory,
        )
    }

    fn ready(&self) -> Result<()> {
        let state = self.0.claim.lock().map_err(|_| Error::Storage)?;
        if !state.ready || state.retired {
            return Err(Error::Storage);
        }
        self.original()?;
        io::check_named(
            self.root(),
            OWNER.as_ref(),
            state.owner.as_ref().ok_or(Error::Storage)?.as_fd(),
            FileType::RegularFile,
        )
    }
    pub(crate) fn take_ordered_workspace(&self) -> Result<crate::metadata_work::PreparedIndex> {
        self.ready()?;
        let mut workspace = self
            .0
            .metadata
            .try_lock()
            .map_err(|error| match error {
                std::sync::TryLockError::WouldBlock => Error::Limit,
                std::sync::TryLockError::Poisoned(_) => Error::Storage,
            })?
            .take()
            .ok_or(Error::Storage)?;
        workspace.cache.bind_lease(self.clone())?;
        Ok(workspace)
    }
    pub(crate) fn check_lane(&self, ns: Id) -> Result<()> {
        self.ready()?;
        self.member(ns)
    }
    fn member(&self, ns: Id) -> Result<()> {
        let member = match self.0.policy.lanes {
            OwnedLaneNamespaces::One(a) => ns == a,
            OwnedLaneNamespaces::Two(a, b) => ns == a || ns == b,
        };
        if member {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
    pub(crate) fn claim(&self, limits: &Limits, resources: ResourceProfile) -> Result<()> {
        if resources != ResourceProfile::SmallV1 {
            return Err(Error::Unsupported);
        }
        resources.check_limits(limits)?;
        if limits != &self.0.policy.limits {
            return Err(Error::Limit);
        }
        let mut state = self.0.claim.lock().map_err(|_| Error::Storage)?;
        if state.started {
            return Err(Error::Storage);
        }
        state.started = true;
        self.original()?;
        let root = self.root();
        let initialized = io::named(root, PROFILE.as_ref())?.is_some();
        let original_owner = if initialized {
            self.check_profile()?;
            Some(io::regular_stat(root, OWNER.as_ref())?)
        } else {
            if self.walk(None, |_, _, _| Ok(()))? != 0 {
                return Err(Error::Storage);
            }
            None
        };
        let create = if initialized {
            OFlags::empty()
        } else {
            OFlags::CREATE | OFlags::EXCL
        };
        state.owner = Some(io::open_raw(root, OWNER.as_ref(), OFlags::RDWR | create)?);
        let file = state.owner.as_ref().ok_or(Error::Storage)?;
        let actual = io::held(file.as_fd())?;
        if original_owner.is_some_and(|named| !io::same(&named, &actual)) {
            return Err(Error::Storage);
        }
        io::check_named(root, OWNER.as_ref(), file.as_fd(), FileType::RegularFile)?;
        let until = Instant::now() + Duration::from_millis(50);
        loop {
            match file.try_lock() {
                Ok(()) => break,
                Err(TryLockError::WouldBlock) if initialized => {
                    let remaining = until.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return Err(Error::Storage);
                    }
                    std::thread::sleep(remaining.min(Duration::from_millis(2)));
                    if Instant::now() >= until {
                        return Err(Error::Storage);
                    }
                }
                Err(_) => return Err(Error::Storage),
            }
        }
        state.locked = true;
        self.original()?;
        io::check_named(
            root,
            OWNER.as_ref(),
            state.owner.as_ref().ok_or(Error::Storage)?.as_fd(),
            FileType::RegularFile,
        )?;
        if !initialized {
            self.initialize_profile()?;
        }
        self.check_profile()?;
        self.recover_stages(state.owner.as_ref().ok_or(Error::Storage)?)?;
        state.ready = true;
        Ok(())
    }
    fn check_profile(&self) -> Result<()> {
        let file = match open_read_at(self.root(), PROFILE.as_ref()) {
            Err(Error::Unknown) => return Err(Error::Storage),
            result => result?,
        };
        verify_profile_content(file)
    }
    fn initialize_profile(&self) -> Result<()> {
        let root = self.root();
        let name = Component::formatted(format_args!(
            ".profile-{}-{}",
            std::process::id(),
            super::PROFILE_STAGE.fetch_add(1, Ordering::Relaxed)
        ))?;
        let stage = name.as_os_str();
        let mut file = create_new_at(root, stage)?;
        file.write_all(super::PROFILE_CONTENT)?;
        file.sync_all()?;
        self.original()?;
        io::check_named(root, stage, file.as_fd(), FileType::RegularFile)?;
        if io::named(root, PROFILE.as_ref())?.is_some() {
            return Err(Error::Storage);
        }
        io::rename_at(root, stage, PROFILE.as_ref())?;
        io::check_named(root, PROFILE.as_ref(), file.as_fd(), FileType::RegularFile)?;
        io::sync(root)
    }
}
