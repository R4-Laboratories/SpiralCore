use super::{io, OwnedStoreLease, OWNER, PROFILE};
use crate::{resource_profile::ResourceProfile, Error, Id, Result};
use rustix::fs::{AtFlags, FileType, Stat};
use std::{
    ffi::{OsStr, OsString},
    os::fd::{AsFd, BorrowedFd},
    sync::Arc,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Item {
    Data,
    Namespace,
    Marker,
    Owner,
}

struct Victim {
    parent: Option<Id>,
    name: OsString,
    stat: Stat,
    kind: Item,
}

fn profile_stage(name: &str) -> bool {
    let Some(rest) = name.strip_prefix(".profile-") else {
        return false;
    };
    let Some((pid, seq)) = rest.split_once('-') else {
        return false;
    };
    !pid.is_empty()
        && !seq.is_empty()
        && [pid, seq]
            .iter()
            .all(|s| s.bytes().all(|b| b.is_ascii_digit()))
}
fn stage(parent: Option<Id>, name: &OsStr) -> bool {
    name.to_str().is_some_and(|s| {
        if parent.is_some() {
            crate::endpoint::owned_stage(s)
        } else {
            profile_stage(s)
        }
    })
}

impl OwnedStoreLease {
    fn classify(
        &self,
        parent: Option<Id>,
        dir: BorrowedFd<'_>,
        name: &OsStr,
        verify: bool,
    ) -> Result<(Item, Stat)> {
        let current = io::named(dir, name)?.ok_or(Error::Storage)?;
        if parent.is_none() && FileType::from_raw_mode(current.st_mode) == FileType::Directory {
            let directory = io::open_dir(dir, name)?;
            if !io::same(&current, &io::held(directory.as_fd())?) {
                return Err(Error::Storage);
            }
            return Ok((Item::Namespace, current));
        }
        io::private(&current, FileType::RegularFile)?;
        if current.st_dev != io::held(dir)?.st_dev {
            return Err(Error::Storage);
        }
        let kind = if stage(parent, name) {
            if verify {
                let stage = io::open_read_at(dir, name)?;
                if !io::same(&current, &io::held(stage.as_fd())?) {
                    return Err(Error::Storage);
                }
            }
            Item::Data
        } else if parent.is_none() && name == OWNER {
            Item::Owner
        } else if parent.is_none() && name == PROFILE {
            if verify {
                self.check_profile()?;
            }
            Item::Marker
        } else {
            if parent.is_none() {
                return Err(Error::Storage);
            }
            let stem = name
                .to_str()
                .and_then(|s| s.strip_suffix(".object"))
                .ok_or(Error::Storage)?;
            let object = crate::wan::parse_id(stem).map_err(|_| Error::Storage)?;
            if verify {
                let file = io::open_read_at(dir, name)?;
                if !io::same(&current, &io::held(file.as_fd())?) {
                    return Err(Error::Storage);
                }
                crate::endpoint::verify_owned_file(
                    object,
                    file,
                    ResourceProfile::SmallV1.max_artifact_len() as u64,
                )?;
            }
            Item::Data
        };
        Ok((kind, current))
    }

    fn next_victim(&self, stages_only: bool) -> Result<Option<Victim>> {
        let mut next: Option<Victim> = None;
        self.walk(None, |parent, dir, name| {
            if stages_only && !stage(parent, name) {
                return Ok(());
            }
            let (kind, stat) = self.classify(parent, dir, name, false)?;
            if next.as_ref().is_none_or(|candidate| kind < candidate.kind) {
                next = Some(Victim {
                    parent,
                    name: name.to_owned(),
                    stat,
                    kind,
                });
            }
            Ok(())
        })?;
        Ok(next)
    }

    fn remove_victim(
        &self,
        v: &Victim,
        locked: &std::fs::File,
        sync: &mut impl FnMut(BorrowedFd<'_>) -> Result<()>,
    ) -> Result<()> {
        self.original()?;
        io::check_named(
            self.root(),
            OWNER.as_ref(),
            locked.as_fd(),
            FileType::RegularFile,
        )?;
        let dir = v
            .parent
            .map(|ns| io::open_dir(self.root(), crate::hex(&ns).as_ref()))
            .transpose()?;
        let parent = dir.as_ref().map(AsFd::as_fd).unwrap_or(self.root());
        let (kind, actual) = self.classify(v.parent, parent, &v.name, true)?;
        if kind != v.kind || !io::same(&v.stat, &actual) {
            return Err(Error::Storage);
        }
        let flags = if kind == Item::Namespace {
            AtFlags::REMOVEDIR
        } else {
            AtFlags::empty()
        };
        rustix::fs::unlinkat(parent, v.name.as_os_str(), flags).map_err(|_| Error::Storage)?;
        sync(parent)
    }

    fn clean(
        &self,
        stages_only: bool,
        locked: &std::fs::File,
        mut sync: impl FnMut(BorrowedFd<'_>) -> Result<()>,
    ) -> Result<()> {
        // Complete preflight precedes even the first removal.
        self.check_profile()?;
        let mut publications = 0usize;
        let mut object_stages = 0usize;
        let mut profile_stages = 0usize;
        let total = self.walk(None, |parent, dir, name| {
            if parent.is_some()
                && std::path::Path::new(name).extension() == Some(OsStr::new("object"))
            {
                io::quota_len_at(dir, name)?;
                publications += 1;
                if publications > self.0.policy.limits.max_publications {
                    return Err(Error::Limit);
                }
            }
            if stage(parent, name) {
                if parent.is_some() {
                    object_stages += 1;
                    if object_stages > self.0.policy.limits.max_states {
                        return Err(Error::Limit);
                    }
                } else {
                    profile_stages += 1;
                    if profile_stages > 1 {
                        return Err(Error::Limit);
                    }
                }
            }
            if !stages_only || stage(parent, name) {
                self.classify(parent, dir, name, true)?;
            }
            Ok(())
        })?;
        for _ in 0..total {
            let Some(v) = self.next_victim(stages_only)? else {
                return Ok(());
            };
            self.remove_victim(&v, locked, &mut sync)?;
        }
        if self.next_victim(stages_only)?.is_some() {
            return Err(Error::Storage);
        }
        Ok(())
    }

    pub(super) fn recover_stages(&self, locked: &std::fs::File) -> Result<()> {
        self.clean(true, locked, io::sync)
    }
    /// Remove verified codec contents while retaining the original locked inode.
    /// The caller holds this lease through its final parent-relative removal and sync.
    pub fn retire_contents(&mut self) -> Result<()> {
        self.retire_with_sync(io::sync)
    }
    pub(super) fn retire_with_sync(
        &mut self,
        sync: impl FnMut(BorrowedFd<'_>) -> Result<()>,
    ) -> Result<()> {
        if Arc::get_mut(&mut self.0).is_none() {
            return Err(Error::Storage);
        }
        let mut state = self.0.claim.lock().map_err(|_| Error::Storage)?;
        if !state.ready || !state.locked || state.retired {
            return Err(Error::Storage);
        }
        state.ready = false;
        self.original()?;
        let owner = state.owner.as_ref().ok_or(Error::Storage)?;
        io::check_named(
            self.root(),
            OWNER.as_ref(),
            owner.as_fd(),
            FileType::RegularFile,
        )?;
        self.clean(false, owner, sync)?;
        self.original()?;
        state.retired = true;
        Ok(())
    }
}
