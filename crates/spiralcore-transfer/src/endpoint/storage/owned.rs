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
}

impl OwnedStorePolicy {
    pub fn small_v1(
        limits: &Limits,
        funded_entry_visits: u16,
        lanes: OwnedLaneNamespaces,
    ) -> Result<Self> {
        ResourceProfile::SmallV1.check_limits(limits)?;
        if limits.max_active != 1
            || limits.max_nodes == 0
            || limits.max_states < limits.max_active
            || matches!(lanes, OwnedLaneNamespaces::Two(a,b) if a == b)
        {
            return Err(Error::Limit);
        }
        // Two root markers, one profile stage and two namespace directories.
        let max_entries = 5usize
            .checked_add(limits.max_publications)
            .and_then(|n| n.checked_add(limits.max_states))
            .ok_or(Error::Limit)?;
        if usize::from(funded_entry_visits) <= max_entries {
            return Err(Error::Limit);
        }
        Ok(Self {
            lanes,
            max_entries,
            limits: limits.clone(),
        })
    }
}

#[cfg(test)]
mod tests;

mod io;
mod retire;
mod walk;
pub(crate) use io::{check_file_at, create_new_at, open_read_at, quota_len_at, unlink_checked_at};

use rustix::fs::{FileType, OFlags};
use std::{
    ffi::{OsStr, OsString},
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
    leaf: OsString,
    child: Arc<File>,
    policy: OwnedStorePolicy,
    claim: Mutex<Claim>,
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

impl OwnedStoreLease {
    /// The provider supplies an original private mount tree with no nested mounts.
    /// Capture performs checks only. Attach this lease to the original permit before opening an endpoint.
    pub fn capture(
        parent: Arc<File>,
        leaf: &OsStr,
        child: Arc<File>,
        policy: OwnedStorePolicy,
    ) -> Result<Self> {
        io::private(&io::held(parent.as_fd())?, FileType::Directory)?;
        io::check_named(parent.as_fd(), leaf, child.as_fd(), FileType::Directory)?;
        Ok(Self(Arc::new(Store {
            parent,
            leaf: leaf.to_owned(),
            child,
            policy,
            claim: Mutex::new(Claim::default()),
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
            &self.0.leaf,
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
        let mut actual = Vec::new();
        file.take(64).read_to_end(&mut actual)?;
        if actual != super::PROFILE_CONTENT {
            return Err(Error::Storage);
        }
        Ok(())
    }
    fn initialize_profile(&self) -> Result<()> {
        let root = self.root();
        let name = format!(
            ".profile-{}-{}",
            std::process::id(),
            super::PROFILE_STAGE.fetch_add(1, Ordering::Relaxed)
        );
        let stage = OsStr::new(&name);
        let mut file = create_new_at(root, stage)?;
        file.write_all(super::PROFILE_CONTENT)?;
        file.sync_all()?;
        self.original()?;
        io::check_named(root, stage, file.as_fd(), FileType::RegularFile)?;
        if io::named(root, PROFILE.as_ref())?.is_some() {
            return Err(Error::Storage);
        }
        rustix::fs::renameat(root, stage, root, PROFILE).map_err(|_| Error::Storage)?;
        io::check_named(root, PROFILE.as_ref(), file.as_fd(), FileType::RegularFile)?;
        io::sync(root)
    }
}
