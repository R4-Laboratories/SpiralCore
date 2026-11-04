//! Native private-file and publication primitives for the single endpoint.
use crate::Result;
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
#[cfg(unix)]
pub(super) use unix::*;
#[cfg(windows)]
pub(super) use windows::*;
#[cfg(not(any(unix, windows)))]
mod unsupported {
    use crate::{Error, Result};
    use std::{fs::File, path::Path};
    pub(in super::super) fn private_file(_: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    pub(in super::super) fn private_dir(_: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    pub(in super::super) fn open_read(_: &Path) -> Result<File> {
        Err(Error::Unsupported)
    }
    pub(in super::super) fn create_new_file(_: &Path) -> Result<File> {
        Err(Error::Unsupported)
    }
    pub(in super::super) fn open_owner(_: &Path) -> Result<File> {
        Err(Error::Unsupported)
    }
    pub(in super::super) fn publish(_: &Path, _: &Path, _: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    pub(in super::super) fn sync_existing(_: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    pub(in super::super) fn remove_stage(_: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
}
#[cfg(not(any(unix, windows)))]
pub(super) use unsupported::*;

static PROFILE_STAGE: AtomicU64 = AtomicU64::new(0);

pub(super) fn initialize_profile(root: &Path, bytes: &[u8]) -> Result<()> {
    let stage = root.join(format!(
        ".profile-{}-{}",
        std::process::id(),
        PROFILE_STAGE.fetch_add(1, Ordering::Relaxed)
    ));
    let destination = root.join(".spiralcore-profile-v1");
    let mut created = false;
    let result = (|| {
        let mut file: File = create_new_file(&stage)?;
        created = true;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        publish(&stage, &destination, root)?;
        let mut actual = Vec::new();
        open_read(&destination)?.take(64).read_to_end(&mut actual)?;
        if actual != bytes {
            return Err(crate::Error::Integrity);
        }
        Ok(())
    })();
    if result.is_err() && created {
        let _ = remove_stage(&stage);
    }
    result
}
