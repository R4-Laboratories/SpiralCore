//! Bounds for the Linux process and filesystem that actually host preparation.
use anyhow::{Result, ensure};
use std::fs::File;
use std::io::{self, Read};
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

/// Retained original MVM root and scratch. The trusted installer controls
/// mount topology and launch paths throughout the operation.
pub(super) struct StartupBounds {
    home: File,
    scratch: File,
    home_path: PathBuf,
    scratch_path: PathBuf,
}

impl StartupBounds {
    pub(super) fn capture(home: &Path, scratch: &Path, memory: u64, disk: u64) -> Result<Self> {
        let value = Self {
            home: private_directory(home)?,
            scratch: private_directory(scratch)?,
            home_path: home.into(),
            scratch_path: scratch.into(),
        };
        value.check(home, scratch, memory, disk)?;
        Ok(value)
    }

    pub(super) fn check(&self, home: &Path, scratch: &Path, memory: u64, disk: u64) -> Result<()> {
        ensure!(
            home == self.home_path && scratch == self.scratch_path,
            "startup paths changed"
        );
        reject_submounts(home, scratch)?;
        let original = self.home.metadata()?;
        for (held, path) in [(&self.home, home), (&self.scratch, scratch)] {
            let current = private_directory(path)?.metadata()?;
            let retained = held.metadata()?;
            ensure!(
                current.dev() == original.dev()
                    && current.dev() == retained.dev()
                    && current.ino() == retained.ino(),
                "original startup filesystem changed"
            );
        }
        let mut limits = MaybeUninit::uninit();
        // SAFETY: getrlimit writes one rlimit in the supplied storage on success.
        if unsafe { libc::getrlimit(libc::RLIMIT_AS, limits.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: getrlimit initialized both fields.
        let limits = unsafe { limits.assume_init() };
        ensure!(
            limits.rlim_max != libc::RLIM_INFINITY && limits.rlim_max <= memory,
            "controller address space exceeds original paid workspace"
        );
        ensure!(
            filesystem_bytes(&self.home)? <= disk,
            "backing filesystem exceeds original paid storage"
        );
        Ok(())
    }

    pub(super) fn check_generation(&self, directory: &File) -> Result<()> {
        ensure!(
            directory.metadata()?.dev() == self.home.metadata()?.dev(),
            "original kernel generation is on another filesystem"
        );
        Ok(())
    }
}

fn private_directory(path: &Path) -> Result<File> {
    ensure!(
        path.is_absolute() && path.canonicalize()? == path,
        "original startup path must be absolute and canonical"
    );
    let directory = File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(path)?;
    let metadata = directory.metadata()?;
    mvm_core::private_fs::require_current_owner(&metadata)?;
    ensure!(
        metadata.mode() & 0o7777 == 0o700,
        "startup directory must be private"
    );
    Ok(directory)
}

fn filesystem_bytes(directory: &File) -> Result<u64> {
    let mut info = MaybeUninit::uninit();
    // SAFETY: fstatfs initializes info on success and the descriptor remains live.
    if unsafe { libc::fstatfs(directory.as_raw_fd(), info.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: fstatfs returned success.
    let info = unsafe { info.assume_init() };
    // A memory-backed or union filesystem needs separate RAM/layer accounting.
    ensure!(
        matches!(info.f_type, libc::EXT4_SUPER_MAGIC | libc::XFS_SUPER_MAGIC),
        "startup requires a local disk-backed filesystem"
    );
    let block_size = u64::try_from(info.f_bsize)?;
    ensure!(
        block_size > 0 && info.f_blocks > 0,
        "empty filesystem geometry"
    );
    info.f_blocks
        .checked_mul(block_size)
        .ok_or_else(|| anyhow::anyhow!("filesystem extent overflow"))
}

fn reject_submounts(home: &Path, scratch: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt;
    // Refuse unrepresentable path forms rather than decoding procfs escapes.
    let roots = [home.as_os_str().as_bytes(), scratch.as_os_str().as_bytes()];
    ensure!(
        roots
            .iter()
            .all(|root| !root.iter().any(|c| c.is_ascii_whitespace() || *c == b'\\')),
        "startup paths must be representable in the mount table"
    );
    let mut table = Vec::new();
    const MAX: u64 = 256 * 1024;
    File::open("/proc/self/mountinfo")?
        .take(MAX + 1)
        .read_to_end(&mut table)?;
    ensure!(
        !table.is_empty() && table.len() as u64 <= MAX,
        "cannot verify startup mount table"
    );
    for line in table.split(|c| *c == b'\n').filter(|line| !line.is_empty()) {
        let path = line
            .split(|c| *c == b' ')
            .nth(4)
            .ok_or_else(|| anyhow::anyhow!("missing mount path"))?;
        ensure!(
            !roots.iter().any(|root| path
                .strip_prefix(*root)
                .is_some_and(|rest| rest.starts_with(b"/"))),
            "nested mount in original startup namespace"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    use std::os::unix::process::CommandExt;

    #[test]
    fn retains_original_directories_and_observes_real_process_and_filesystem_limits() {
        const SELECTOR: &str = "installed_parent::startup_bounds::tests::retains_original_directories_and_observes_real_process_and_filesystem_limits";
        const MAX_AS: u64 = 1024 * 1024 * 1024;
        const FLAG: &str = "SPIRALCORE_TEST_FUNDED_STARTUP_CHILD";
        if std::env::var_os(FLAG).is_none() {
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args(["--exact", SELECTOR, "--nocapture"])
                .env(FLAG, "1");
            // SAFETY: only setrlimit in the single child between fork and exec.
            unsafe {
                command.pre_exec(|| {
                    let limits = libc::rlimit {
                        rlim_cur: MAX_AS,
                        rlim_max: MAX_AS,
                    };
                    if libc::setrlimit(libc::RLIMIT_AS, &limits) == 0 {
                        Ok(())
                    } else {
                        Err(io::Error::last_os_error())
                    }
                });
            }
            assert!(command.status().unwrap().success());
            return;
        }
        // The operator chooses the actual disk-backed test root (ordinary Linux
        // /tmp can be memory-backed). All artifacts remain private and temporary.
        let parent = std::env::var_os("SPIRALCORE_TEST_DISK")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        let temp = tempfile::tempdir_in(parent).unwrap();
        let home = temp.path().join("home");
        let scratch = temp.path().join("scratch");
        for path in [&home, &scratch] {
            std::fs::DirBuilder::new().mode(0o700).create(path).unwrap();
        }
        let capacity = filesystem_bytes(&File::open(&home).unwrap()).unwrap();
        assert!(StartupBounds::capture(&home, &scratch, MAX_AS - 1, capacity).is_err());
        assert!(StartupBounds::capture(&home, &scratch, MAX_AS, capacity - 1).is_err());
        let held = StartupBounds::capture(&home, &scratch, MAX_AS, capacity).unwrap();
        let old = temp.path().join("old-scratch");
        std::fs::rename(&scratch, &old).unwrap();
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&scratch)
            .unwrap();
        assert!(held.check(&home, &scratch, MAX_AS, capacity).is_err());
        std::fs::remove_dir(&scratch).unwrap();
        std::fs::rename(&old, &scratch).unwrap();
        held.check(&home, &scratch, MAX_AS, capacity).unwrap();
    }
}
