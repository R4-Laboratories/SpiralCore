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
pub(in super::super) fn quota_file_len(path: &Path) -> Result<u64> {
    // Accounting does not grant content access. Keep the same private-file
    // checks without opening every retained object just to obtain its length.
    let metadata = fs::symlink_metadata(path)?;
    check_file(&metadata)?;
    Ok(metadata.len())
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
fn check_directory(metadata: &fs::Metadata) -> Result<()> {
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(Error::Storage);
    }
    Ok(())
}
pub(in super::super) fn private_dir(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            check_directory(&metadata)?;
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
fn open_existing_private_directory(path: &Path) -> Result<File> {
    let named = fs::symlink_metadata(path)?;
    check_directory(&named)?;
    let directory = File::open(path)?;
    let held = directory.metadata()?;
    check_directory(&held)?;
    if (named.dev(), named.ino()) != (held.dev(), held.ino()) {
        return Err(Error::Storage);
    }
    Ok(directory)
}
pub(in super::super) fn publish(stage: &Path, destination: &Path, _root: &Path) -> Result<()> {
    let source = open_existing_private_directory(stage.parent().ok_or(Error::Storage)?)?;
    let parent = destination.parent().ok_or(Error::Storage)?;
    let target = open_existing_private_directory(parent)?;
    let (source_meta, target_meta) = (source.metadata()?, target.metadata()?);
    // Both existing constructors stage inside the destination directory. Prove
    // actual parent identity before mutation; unsupported cross-parent moves
    // must not silently omit a modified source-directory durability barrier.
    if (source_meta.dev(), source_meta.ino()) != (target_meta.dev(), target_meta.ino()) {
        return Err(Error::Storage);
    }
    fs::rename(stage, destination)?;
    sync_directory(&target, parent)?;
    Ok(())
}
fn sync_directory(directory: &File, path: &Path) -> Result<()> {
    #[cfg(test)]
    tests::before_sync(path)?;
    #[cfg(not(test))]
    let _ = path;
    directory.sync_all()?;
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

#[cfg(test)]
#[path = "unix/tests.rs"]
mod tests;
