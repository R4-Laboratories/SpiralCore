use super::{io, OwnedStoreLease};
use crate::{hex, Error, Id, Result};
use rustix::fs::{FileType, Mode};
use std::{
    ffi::OsStr,
    fs::File,
    os::fd::{AsFd, BorrowedFd},
};

impl OwnedStoreLease {
    pub(super) fn walk(
        &self,
        scope: Option<Id>,
        mut visit: impl FnMut(Option<Id>, BorrowedFd<'_>, &OsStr) -> Result<()>,
    ) -> Result<usize> {
        if let Some(ns) = scope {
            self.member(ns)?;
        }
        self.original()?;
        let root = self.root();
        let mut remaining = self.0.policy.max_entries;
        // Never wait with the claim lock held, and refuse before opening iterator FDs.
        let mut work = self.0.work.try_lock().map_err(|error| match error {
            std::sync::TryLockError::WouldBlock => Error::Limit,
            std::sync::TryLockError::Poisoned(_) => Error::Storage,
        })?;
        let super::work::Buffers {
            root: root_buffer,
            child: child_buffer,
        } = &mut *work;
        root_buffer.scan(root, |name| {
            remaining = remaining.checked_sub(1).ok_or(Error::Limit)?;
            let stat = io::named(root, name)?.ok_or(Error::Storage)?;
            if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
                if scope.is_none() {
                    visit(None, root, name)?;
                }
                return Ok(());
            }
            let ns = crate::wan::parse_id(name.to_str().ok_or(Error::Storage)?)
                .map_err(|_| Error::Storage)?;
            self.member(ns)?;
            if scope.is_none() {
                visit(None, root, name)?;
            }
            if scope.is_some_and(|selected| selected != ns) {
                return Ok(());
            }
            let dir = io::open_dir(root, name)?;
            child_buffer.scan(dir.as_fd(), |child| {
                remaining = remaining.checked_sub(1).ok_or(Error::Limit)?;
                visit(Some(ns), dir.as_fd(), child)
            })?;
            io::check_named(root, name, dir.as_fd(), FileType::Directory)
        })?;
        self.original()?;
        Ok(self.0.policy.max_entries - remaining)
    }

    pub(crate) fn visit(
        &self,
        scope: Option<Id>,
        visitor: impl FnMut(Option<Id>, BorrowedFd<'_>, &OsStr) -> Result<()>,
    ) -> Result<()> {
        self.ready()?;
        self.walk(scope, visitor)?;
        self.ready()
    }
    pub(crate) fn check_room(&self, extra: usize) -> Result<()> {
        self.ready()?;
        let occupied = self.walk(None, |_, _, _| Ok(()))?;
        if extra > self.0.policy.max_entries - occupied {
            return Err(Error::Limit);
        }
        Ok(())
    }
    pub(crate) fn open_namespace(&self, ns: Id, create: bool) -> Result<File> {
        self.check_lane(ns)?;
        let root = self.root();
        let name = hex(&ns);
        match io::open_dir(root, name.as_ref()) {
            Err(Error::Unknown) if create => {
                self.check_room(1)?;
                rustix::fs::mkdirat(root, name.as_str(), Mode::from_raw_mode(0o700))
                    .map_err(|_| Error::Storage)?;
                let file = io::open_dir(root, name.as_ref())?;
                file.sync_all()?;
                io::sync(root)?;
                Ok(file)
            }
            result => result,
        }
    }
    pub(crate) fn check_namespace(&self, ns: Id, dir: BorrowedFd<'_>) -> Result<()> {
        self.check_lane(ns)?;
        io::check_named(self.root(), hex(&ns).as_ref(), dir, FileType::Directory)
    }
    pub(crate) fn sync_namespace(&self, ns: Id, dir: BorrowedFd<'_>) -> Result<()> {
        self.check_namespace(ns, dir)?;
        io::sync(dir)?;
        self.check_namespace(ns, dir)
    }
    pub(crate) fn sync_publication(
        &self,
        ns: Id,
        dir: BorrowedFd<'_>,
        name: &OsStr,
        file: &File,
    ) -> Result<()> {
        self.check_namespace(ns, dir)?;
        io::check_file_at(dir, name, file)?;
        file.sync_all()?;
        self.sync_published_name(ns, dir, name, file)
    }

    fn sync_published_name(
        &self,
        ns: Id,
        dir: BorrowedFd<'_>,
        name: &OsStr,
        file: &File,
    ) -> Result<()> {
        self.check_namespace(ns, dir)?;
        io::check_file_at(dir, name, file)?;
        #[cfg(test)]
        self.check_injected_publication_sync()?;
        io::sync(dir)?;
        io::sync(self.root())?;
        io::check_file_at(dir, name, file)?;
        self.check_namespace(ns, dir)
    }

    pub(crate) fn publish_at(
        &self,
        ns: Id,
        dir: BorrowedFd<'_>,
        stage: &OsStr,
        dest: &OsStr,
        staged: &File,
    ) -> Result<()> {
        self.check_namespace(ns, dir)?;
        io::check_named(dir, stage, staged.as_fd(), FileType::RegularFile)?;
        if io::named(dir, dest)?.is_some() {
            return Err(Error::Storage);
        }
        rustix::fs::renameat(dir, stage, dir, dest).map_err(|_| Error::Storage)?;
        // The caller synced the staged file before authorizing this rename.
        self.sync_published_name(ns, dir, dest, staged)
    }
}
