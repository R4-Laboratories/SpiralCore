use anyhow::{Context, Result, ensure};
use mvm_core::private_fs::require_current_owner;
use std::{
    ffi::CString,
    fs::{self, File, Metadata, OpenOptions, Permissions},
    io::{self, Write},
    os::{
        fd::AsRawFd,
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        },
    },
    path::Path,
};

fn identity(meta: &Metadata) -> (u64, u64, u32) {
    (meta.dev(), meta.ino(), meta.uid())
}

fn private_dir(path: &Path) -> Result<File> {
    ensure!(
        path.as_os_str() == path.canonicalize()?.as_os_str(),
        "aliased directory"
    );
    let dir = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(path)?;
    let meta = dir.metadata()?;
    require_current_owner(&meta)?;
    ensure!(meta.mode() & 0o7777 == 0o700, "directory must be private");
    for ancestor in path.ancestors().skip(1) {
        let m = fs::symlink_metadata(ancestor)?;
        ensure!(
            m.is_dir()
                && (m.uid() == 0 || m.uid() == meta.uid())
                && (m.mode() & 0o022 == 0 || m.mode() & 0o1000 != 0),
            "directory has an unprotected ancestor"
        );
    }
    Ok(dir)
}

fn leaf(path: &Path) -> Result<CString> {
    let name = path.file_name().context("ready file name required")?;
    Ok(CString::new(name.as_bytes())?)
}

pub(super) fn finish(
    working: tempfile::TempDir,
    ready: Option<&Path>,
    archive_sha256: &str,
) -> Result<()> {
    let Some(ready) = ready else { return Ok(()) };
    let parent = ready.parent().context("ready parent required")?;
    let dest = leaf(ready)?;
    ensure!(
        parent
            .join(ready.file_name().unwrap_or_default())
            .as_os_str()
            == ready.as_os_str(),
        "aliased ready path"
    );
    let dir = private_dir(parent)?;
    let original_dir = identity(&dir.metadata()?);
    let root = working.path();
    ensure!(
        !parent.starts_with(root),
        "ready file belongs outside retained workspace"
    );
    let root_file = private_dir(root)?;
    let root_meta = root_file.metadata()?;
    let record = serde_json::to_vec(&serde_json::json!({
        "root": root.to_str().context("non-UTF-8 workspace")?,
        "dev":root_meta.dev(),"ino":root_meta.ino(),"uid":root_meta.uid(),
        "archive_sha256":archive_sha256
    }))?;
    ensure!(record.len() <= 8192, "ready record exceeds limit");
    let mut stage = tempfile::Builder::new()
        .permissions(Permissions::from_mode(0o600))
        .tempfile_in(parent)?;
    let original = stage.as_file().metadata()?;
    ensure!(
        original.is_file()
            && original.nlink() == 1
            && original.uid() == root_meta.uid()
            && original.mode() & 0o7777 == 0o600,
        "invalid ready file permissions"
    );
    stage.write_all(&record)?;
    stage.as_file().sync_all()?;
    let src = leaf(stage.path())?;
    ensure!(
        identity(&private_dir(parent)?.metadata()?) == original_dir
            && identity(&fs::symlink_metadata(stage.path())?) == identity(&original),
        "staged ready file changed"
    );
    let fd = dir.as_raw_fd();
    // SAFETY: both names are NUL-terminated single components of the held parent.
    // RENAME_NOREPLACE refuses any occupied destination, including a dangling symlink.
    if unsafe { libc::renameat2(fd, src.as_ptr(), fd, dest.as_ptr(), libc::RENAME_NOREPLACE) } != 0
    {
        return Err(io::Error::last_os_error().into());
    }
    stage.disable_cleanup(true); // The original temporary name is no longer ours.
    dir.sync_all()?;
    ensure!(
        identity(&private_dir(parent)?.metadata()?) == original_dir
            && identity(&fs::symlink_metadata(ready)?) == identity(&original)
            && identity(&private_dir(root)?.metadata()?) == identity(&root_meta),
        "published handoff identity changed"
    );
    let _retained = working.keep();
    Ok(())
}
