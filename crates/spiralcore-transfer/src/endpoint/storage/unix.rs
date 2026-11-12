use crate::{Error, Result};
use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};

fn check_file(metadata: &fs::Metadata) -> Result<()> {
    if !metadata.is_file() || metadata.nlink() != 1 || metadata.permissions().mode() & 0o077 != 0 {
        return Err(Error::Storage);
    }
    Ok(())
}
pub(in super::super) fn private_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(Error::Storage);
    }
    check_file(&metadata)
}
pub(in super::super) fn open_read(path: &Path) -> Result<File> {
    private_file(path)?;
    let file = File::open(path)?;
    check_file(&file.metadata()?)?;
    Ok(file)
}
pub(in super::super) fn create_new_file(path: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?)
}
pub(in super::super) fn open_owner(path: &Path) -> Result<File> {
    if fs::symlink_metadata(path).is_ok() {
        private_file(path)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)?;
    check_file(&file.metadata()?)?;
    Ok(file)
}
pub(in super::super) fn lock_owner(file: &File, path: &Path, initialized: bool) -> Result<()> {
    match file.try_lock() {
        Ok(()) => Ok(()),
        Err(std::fs::TryLockError::WouldBlock) if initialized => lock_reopened_owner(file, path),
        Err(_) => Err(Error::Storage),
    }
}

pub(in super::super) fn lock_reopened_owner(file: &File, path: &Path) -> Result<()> {
    lock_reopened_owner_with(file, path, std::time::Instant::now, std::thread::sleep)
}

pub(in super::super) fn lock_reopened_owner_with(
    file: &File,
    path: &Path,
    now: impl Fn() -> std::time::Instant,
    mut wait: impl FnMut(std::time::Duration),
) -> Result<()> {
    use std::time::Duration;

    // A concurrent fork/exec briefly retains the old open file description.
    // Keep the kernel lock authoritative. Only an initialized cache may wait.
    let deadline = now() + Duration::from_millis(50);
    loop {
        let remaining = deadline.saturating_duration_since(now());
        if remaining.is_zero() {
            return Err(Error::Storage);
        }
        wait(remaining.min(Duration::from_millis(2)));
        if now() >= deadline {
            return Err(Error::Storage);
        }
        match file.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) => continue,
            Err(_) => return Err(Error::Storage),
        }
    }

    let named = fs::symlink_metadata(path)?;
    let locked = file.metadata()?;
    check_file(&named)?;
    if (named.dev(), named.ino()) != (locked.dev(), locked.ino()) {
        return Err(Error::Storage);
    }
    let root = path.parent().ok_or(Error::Storage)?;
    let directory = fs::symlink_metadata(root)?;
    if !directory.is_dir() || directory.permissions().mode() & 0o077 != 0 {
        return Err(Error::Storage);
    }
    super::check_profile(&root.join(".spiralcore-profile-v1"))?;
    if now() >= deadline {
        return Err(Error::Storage);
    }
    Ok(())
}
pub(in super::super) fn private_dir(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_dir()
                || metadata.file_type().is_symlink()
                || metadata.permissions().mode() & 0o077 != 0
            {
                return Err(Error::Storage);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            if !parent.exists() {
                private_dir(parent)?;
            }
            fs::DirBuilder::new().mode(0o700).create(path)?;
            File::open(path)?.sync_all()?;
            File::open(parent)?.sync_all()?;
        }
        Err(_) => return Err(Error::Storage),
    }
    Ok(())
}
pub(in super::super) fn publish(stage: &Path, destination: &Path, root: &Path) -> Result<()> {
    fs::rename(stage, destination)?;
    File::open(destination.parent().ok_or(Error::Storage)?)?.sync_all()?;
    File::open(root)?.sync_all()?;
    Ok(())
}
pub(in super::super) fn sync_existing(path: &Path) -> Result<()> {
    File::open(path.parent().ok_or(Error::Storage)?)?.sync_all()?;
    Ok(())
}
pub(in super::super) fn remove_stage(path: &Path) -> Result<()> {
    private_file(path)?;
    fs::remove_file(path)?;
    Ok(())
}
