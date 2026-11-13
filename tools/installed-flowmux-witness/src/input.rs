use anyhow::{Context, Result, ensure};
use mvm_core::transcript::secure_cleanup::CaptureDirectory;
use rustix::fs::{Mode, OFlags};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Seek},
    net::{Ipv4Addr, SocketAddrV4},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
};
use tempfile::NamedTempFile;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivePin {
    pub decoded_bytes: u64,
    pub sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub issuer_id: String,
    pub allowed_ip: Ipv4Addr,
    pub port: u16,
    pub server_name: String,
    pub archive: ArchivePin,
}

impl Profile {
    pub fn check(&self) -> Result<()> {
        ensure!(
            !self.issuer_id.is_empty() && self.issuer_id.len() <= 128,
            "issuer ID required"
        );
        ensure!(
            self.port == 38443,
            "port differs from the sealed guest role"
        );
        ensure!(
            self.archive.decoded_bytes > 0
                && self.archive.decoded_bytes <= mvm_contract::plan::bundle::MAX_BUNDLE_TOTAL_BYTES,
            "archive extent outside bundle limit"
        );
        ensure!(
            self.archive.sha256.len() == 64
                && self
                    .archive
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "expected a lowercase SHA-256"
        );
        Ok(())
    }
    pub fn address(&self) -> SocketAddrV4 {
        SocketAddrV4::new(self.allowed_ip, self.port)
    }
    pub fn matches_archive(&self, sha: &str, extent: u64) -> Result<()> {
        ensure!(
            self.archive.sha256 == sha && self.archive.decoded_bytes == extent,
            "archive differs from selected digest or extent"
        );
        Ok(())
    }
}

pub fn explicit_home() -> Result<PathBuf> {
    let home = PathBuf::from(std::env::var_os("MVM_HOME").context("dedicated MVM_HOME required")?);
    ensure!(
        home.is_absolute()
            && !home.components().any(|p| matches!(p, Component::ParentDir))
            && home == mvm_core::config::mvm_home_strict()?,
        "expected an explicit absolute original MVM home"
    );
    Ok(home)
}

/// Restrict temporary archive and endpoint data to the original operator-funded filesystem.
pub fn funded_scratch(home: &Path) -> Result<PathBuf> {
    let path =
        PathBuf::from(std::env::var_os("TMPDIR").context("explicit private TMPDIR required")?);
    ensure!(
        path.is_absolute() && path != home,
        "a separate absolute scratch directory is required"
    );
    let open = |p: &Path| -> Result<std::fs::Metadata> {
        let f = File::from(rustix::fs::open(
            p,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )?);
        let meta = f.metadata()?;
        ensure!(
            meta.uid() == rustix::process::geteuid().as_raw() && meta.mode() & 0o077 == 0,
            "expected private owned directory"
        );
        Ok(meta)
    };
    let parent = open(home)?;
    let tmp = open(&path)?;
    ensure!(
        parent.dev() == tmp.dev() && parent.ino() != tmp.ino(),
        "scratch must be separate on the original funded filesystem"
    );
    Ok(path)
}

pub fn parse_inputs(args: impl IntoIterator<Item = OsString>) -> Result<PathBuf> {
    let mut args = args.into_iter();
    ensure!(
        args.next().as_deref() == Some(std::ffi::OsStr::new("--inputs")),
        "expected --inputs ABSOLUTE_ROLE_DIRECTORY"
    );
    let dir = PathBuf::from(args.next().context("missing input directory")?);
    ensure!(
        args.next().is_none()
            && dir.is_absolute()
            && dir.file_name().is_some()
            && !dir.components().any(|c| matches!(c, Component::ParentDir)),
        "expected one absolute role directory"
    );
    Ok(dir)
}

pub fn staged(path: &Path) -> Result<CaptureDirectory> {
    Ok(CaptureDirectory::open(
        path.parent().context("missing trusted root")?,
        Path::new(path.file_name().context("missing role directory")?),
    )?)
}

pub fn protected_json<T: DeserializeOwned>(path: &Path, cap: u64) -> Result<T> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags((OFlags::NOFOLLOW | OFlags::NONBLOCK).bits() as i32)
        .open(path)?;
    let n = protected_size(&f, cap)?;
    let mut buf = Vec::new();
    f.take(n + 1).read_to_end(&mut buf)?;
    ensure!(buf.len() as u64 == n, "handoff changed length");
    Ok(serde_json::from_slice(&buf)?)
}

fn protected_size(f: &File, cap: u64) -> Result<u64> {
    let m = f.metadata()?;
    ensure!(
        m.is_file()
            && m.nlink() == 1
            && m.mode() & 0o7777 == 0o600
            && m.uid() == rustix::process::geteuid().as_raw()
            && m.len() <= cap,
        "expected bounded owner-only single-link regular file"
    );
    Ok(m.len())
}

/// Copy only the explicitly funded archive extent to an owned tempfile before path-based registry verification.
pub fn capture_archive(
    role: &Path,
    name: &str,
    pin: &ArchivePin,
    scratch: &Path,
) -> Result<NamedTempFile> {
    ensure!(
        matches!(
            Path::new(name).components().collect::<Vec<_>>().as_slice(),
            [Component::Normal(_)]
        ),
        "expected a single archive leaf"
    );
    let dir = rustix::fs::open(
        role,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
    )?;
    let dir = File::from(dir);
    let m = dir.metadata()?;
    ensure!(
        m.uid() == rustix::process::geteuid().as_raw() && m.mode() & 0o077 == 0,
        "private input directory required"
    );
    let file = File::from(rustix::fs::openat(
        &dir,
        name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOFOLLOW,
        Mode::empty(),
    )?);
    ensure!(
        protected_size(&file, pin.decoded_bytes)? == pin.decoded_bytes,
        "archive extent mismatch"
    );
    let mut src = file.take(pin.decoded_bytes);
    let mut staging = NamedTempFile::new_in(scratch)?;
    ensure!(
        std::io::copy(&mut src, &mut staging)? == pin.decoded_bytes
            && src.into_inner().read(&mut [0; 1])? == 0,
        "archive changed extent"
    );
    staging.as_file().sync_all()?;
    staging
        .as_file()
        .set_permissions(std::fs::Permissions::from_mode(0o400))?;
    staging.as_file_mut().rewind()?;
    ensure!(
        mvm_core::crypto::image_verify::sha256_reader(staging.as_file())? == pin.sha256,
        "archive digest mismatch"
    );
    Ok(staging)
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
