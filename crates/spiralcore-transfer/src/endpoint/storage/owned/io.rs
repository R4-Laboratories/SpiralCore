use crate::{Error, Result};
use rustix::{
    fs::{self, AtFlags, FileType, Mode, OFlags, Stat},
    io::Errno,
};
use std::{
    ffi::OsStr,
    fs::File,
    os::{
        fd::{AsFd, BorrowedFd},
        unix::ffi::OsStrExt,
    },
};

pub(super) fn component(name: &OsStr) -> Result<()> {
    let bytes = name.as_bytes();
    // Darwin dirent.d_name accommodates 1023 bytes plus its NUL terminator.
    // Linux SmallV1 uses a 255-byte component ceiling at the owned boundary.
    let max_bytes = if cfg!(target_os = "macos") { 1023 } else { 255 };
    if bytes.len() > max_bytes
        || bytes.is_empty()
        || bytes == b"."
        || bytes == b".."
        || bytes.contains(&b'/')
        || bytes.contains(&0)
    {
        return Err(Error::Storage);
    }
    Ok(())
}

pub(super) fn held(fd: BorrowedFd<'_>) -> Result<Stat> {
    fs::fstat(fd).map_err(|_| Error::Storage)
}
pub(super) fn named(dir: BorrowedFd<'_>, name: &OsStr) -> Result<Option<Stat>> {
    component(name)?;
    match fs::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(s) => Ok(Some(s)),
        Err(Errno::NOENT) => Ok(None),
        Err(_) => Err(Error::Storage),
    }
}

pub(super) fn same(a: &Stat, b: &Stat) -> bool {
    a.st_dev == b.st_dev && a.st_ino == b.st_ino
}

pub(super) fn private(s: &Stat, kind: FileType) -> Result<()> {
    if FileType::from_raw_mode(s.st_mode) != kind
        || s.st_mode & 0o077 != 0
        || s.st_uid != rustix::process::geteuid().as_raw()
        || s.st_nlink == 0
        || (kind == FileType::RegularFile && s.st_nlink != 1)
    {
        Err(Error::Storage)
    } else {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn apfs(dir: BorrowedFd<'_>) -> Result<rustix::fs::Fsid> {
    let fs = fs::fstatfs(dir).map_err(|_| Error::Storage)?;
    let flags = fs.f_flags;
    if flags & (libc::MNT_LOCAL as u32) == 0
        || flags & (libc::MNT_UNION as u32) != 0
        || !fs.f_fstypename.starts_with(&[97, 112, 102, 115, 0])
    {
        return Err(Error::Storage);
    }
    Ok(fs.f_fsid)
}

#[cfg(target_os = "linux")]
fn mount_id(fd: BorrowedFd<'_>) -> Result<u64> {
    let mask = fs::StatxFlags::MNT_ID;
    let stat = fs::statx(fd, "", AtFlags::EMPTY_PATH, mask).map_err(|err| {
        if err == Errno::NOSYS || err == Errno::OPNOTSUPP {
            Error::Unsupported
        } else {
            Error::Storage
        }
    })?;
    if stat.stx_mask & mask.bits() == 0 {
        return Err(Error::Unsupported);
    }
    Ok(stat.stx_mnt_id)
}

pub(super) fn check_mount(a: BorrowedFd<'_>, b: BorrowedFd<'_>) -> Result<()> {
    #[cfg(target_os = "linux")]
    let matched = mount_id(a)? == mount_id(b)?;
    #[cfg(target_os = "macos")]
    let matched = apfs(a)? == apfs(b)?;
    if matched {
        Ok(())
    } else {
        Err(Error::Storage)
    }
}

pub(super) fn check_named(
    dir: BorrowedFd<'_>,
    name: &OsStr,
    fd: BorrowedFd<'_>,
    kind: FileType,
) -> Result<()> {
    check_mount(dir, fd)?;
    let actual = held(fd)?;
    private(&actual, kind)?;
    let current = named(dir, name)?.ok_or(Error::Storage)?;
    if !same(&current, &actual) || actual.st_dev != held(dir)?.st_dev {
        return Err(Error::Storage);
    }
    private(&current, kind)
}

fn flags() -> OFlags {
    OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC
}
pub(super) fn open_raw(dir: BorrowedFd<'_>, name: &OsStr, extra: OFlags) -> Result<File> {
    component(name)?;
    fs::openat(dir, name, flags() | extra, Mode::from_raw_mode(0o600))
        .map(File::from)
        .map_err(|e| {
            if e == Errno::NOENT {
                Error::Unknown
            } else {
                Error::Storage
            }
        })
}

pub(crate) fn open_read_at(dir: BorrowedFd<'_>, name: &OsStr) -> Result<File> {
    let expected = regular_stat(dir, name)?;
    let file = open_raw(dir, name, OFlags::RDONLY)?;
    if !same(&expected, &held(file.as_fd())?) {
        return Err(Error::Storage);
    }
    check_named(dir, name, file.as_fd(), FileType::RegularFile)?;
    Ok(file)
}
pub(crate) fn create_new_at(dir: BorrowedFd<'_>, name: &OsStr) -> Result<File> {
    let file = open_raw(dir, name, OFlags::RDWR | OFlags::CREATE | OFlags::EXCL)?;
    check_named(dir, name, file.as_fd(), FileType::RegularFile)?;
    Ok(file)
}
pub(super) fn regular_stat(dir: BorrowedFd<'_>, name: &OsStr) -> Result<Stat> {
    let s = named(dir, name)?.ok_or(Error::Unknown)?;
    private(&s, FileType::RegularFile)?;
    if s.st_dev != held(dir)?.st_dev {
        return Err(Error::Storage);
    }
    Ok(s)
}
pub(crate) fn quota_len_at(dir: BorrowedFd<'_>, name: &OsStr) -> Result<u64> {
    u64::try_from(regular_stat(dir, name)?.st_size).map_err(|_| Error::Storage)
}
pub(crate) fn check_file_at(dir: BorrowedFd<'_>, name: &OsStr, file: &File) -> Result<()> {
    check_named(dir, name, file.as_fd(), FileType::RegularFile)
}
pub(crate) fn unlink_checked_at(dir: BorrowedFd<'_>, name: &OsStr, expected: &File) -> Result<()> {
    check_file_at(dir, name, expected)?;
    fs::unlinkat(dir, name, AtFlags::empty()).map_err(|_| Error::Storage)?;
    sync(dir)
}
pub(super) fn sync(dir: BorrowedFd<'_>) -> Result<()> {
    fs::fsync(dir).map_err(|_| Error::Storage)
}
pub(super) fn open_dir(dir: BorrowedFd<'_>, name: &OsStr) -> Result<File> {
    let file = open_raw(dir, name, OFlags::RDONLY | OFlags::DIRECTORY)?;
    check_named(dir, name, file.as_fd(), FileType::Directory)?;
    Ok(file)
}
pub(super) fn iterator(fd: BorrowedFd<'_>) -> Result<fs::Dir> {
    private(&held(fd)?, FileType::Directory)?;
    let original = fs::openat(
        fd,
        ".",
        flags() | OFlags::RDONLY | OFlags::DIRECTORY,
        Mode::empty(),
    )
    .map_err(|_| Error::Storage)?;
    if !same(&held(fd)?, &held(original.as_fd())?) {
        return Err(Error::Storage);
    }
    check_mount(fd, original.as_fd())?;
    fs::Dir::new(original).map_err(|_| Error::Storage)
}
