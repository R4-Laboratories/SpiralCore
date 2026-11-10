//! Safe mapping to the isolated Win32 file backend; no protocol or authority
//! implementation crosses the filesystem boundary.
use crate::{Error, Result};
use std::{fs::File, path::Path};
impl From<spiralcore_storage_windows::Error> for Error {
    fn from(error: spiralcore_storage_windows::Error) -> Self {
        match error {
            spiralcore_storage_windows::Error::Storage => Self::Storage,
            spiralcore_storage_windows::Error::Unsupported => Self::Unsupported,
            spiralcore_storage_windows::Error::Limit => Self::Limit,
        }
    }
}
pub(in super::super) fn private_file(path: &Path) -> Result<()> {
    Ok(spiralcore_storage_windows::private_file(path)?)
}
pub(in super::super) fn private_dir(path: &Path) -> Result<()> {
    Ok(spiralcore_storage_windows::private_dir(path)?)
}
pub(in super::super) fn open_read(path: &Path) -> Result<File> {
    Ok(spiralcore_storage_windows::open_read(path)?)
}
pub(in super::super) fn quota_file_len(path: &Path) -> Result<u64> {
    // Preserve the native backend's held-file ACL/reparse/sharing checks.
    Ok(open_read(path)?.metadata()?.len())
}
pub(in super::super) fn create_new_file(path: &Path) -> Result<File> {
    Ok(spiralcore_storage_windows::create_new_file(path)?)
}
pub(in super::super) fn open_owner(path: &Path) -> Result<File> {
    Ok(spiralcore_storage_windows::open_owner(path)?)
}
pub(in super::super) fn publish(stage: &Path, destination: &Path, root: &Path) -> Result<()> {
    Ok(spiralcore_storage_windows::publish(
        stage,
        destination,
        root,
    )?)
}
pub(in super::super) fn sync_existing(path: &Path) -> Result<()> {
    Ok(spiralcore_storage_windows::sync_existing(path)?)
}
pub(in super::super) fn remove_stage(path: &Path) -> Result<()> {
    Ok(spiralcore_storage_windows::remove_stage(path)?)
}
