use anyhow::{Context, Result, ensure};
use mvm_core::private_fs::require_current_owner;
use std::{
    ffi::CString,
    fs::{self, File, OpenOptions, Permissions},
    io::{self, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        },
    },
    path::Path,
};

pub(super) fn private_dir(path: &Path) -> Result<File> {
    let canonical = fs::canonicalize(path)?;
    ensure!(
        path.as_os_str() == canonical.as_os_str(),
        "expected an absolute, unaliased directory"
    );
    let dir = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)?;
    let meta = dir.metadata()?;
    require_current_owner(&meta)?;
    ensure!(meta.mode() & 0o7777 == 0o700, "directory is not private");
    // The caller's directory entry must also be protected from different users.
    for ancestor in path.ancestors().skip(1) {
        let m = fs::symlink_metadata(ancestor)?;
        ensure!(
            m.is_dir() && (m.uid() == 0 || m.uid() == meta.uid()),
            "unsafe directory ancestor"
        );
        ensure!(
            m.mode() & 0o022 == 0 || m.mode() & 0o1000 != 0,
            "writable nonsticky ancestor"
        );
    }
    Ok(dir)
}

pub(super) fn scratch(parent: &Path) -> Result<tempfile::TempDir> {
    Ok(tempfile::Builder::new()
        .permissions(Permissions::from_mode(0o700))
        .tempdir_in(parent)?)
}

pub(super) struct Destination<'a> {
    output: &'a Path,
    parent: &'a Path,
    dir: File,
}

impl<'a> Destination<'a> {
    pub(super) fn fresh(output: &'a Path) -> Result<Self> {
        let parent = output.parent().context("output parent required")?;
        let name = output.file_name().context("output leaf required")?;
        ensure!(
            parent.join(name).as_os_str() == output.as_os_str(),
            "output must be unaliased"
        );
        let dir = private_dir(parent).context("output parent must be private")?;
        match fs::symlink_metadata(output) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
            Ok(_) => anyhow::bail!("output already exists"),
        }
        Ok(Self {
            output,
            parent,
            dir,
        })
    }
    pub(super) fn path(&self) -> &Path {
        self.parent
    }

    pub(super) fn publish(self, packet: tempfile::TempDir) -> Result<()> {
        let original = self.dir.metadata()?;
        let now = private_dir(self.parent)?.metadata()?;
        ensure!(
            (original.dev(), original.ino()) == (now.dev(), now.ino()),
            "output parent changed"
        );
        File::open(packet.path())?.sync_all()?;
        let leaf = |p: &Path| -> Result<CString> {
            Ok(CString::new(
                p.file_name().context("expected packet leaf")?.as_bytes(),
            )?)
        };
        let source = leaf(packet.path())?;
        let dest = leaf(self.output)?;
        let dir = self.dir.as_raw_fd();
        // SAFETY: both names are live NUL-terminated single components of the held
        // original parent. The kernel consumes the strings synchronously.
        if unsafe {
            libc::renameat2(
                dir,
                source.as_ptr(),
                dir,
                dest.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        } != 0
        {
            return Err(io::Error::last_os_error().into());
        }
        // TempDir's old name no longer belongs to us after the move.
        let _ = packet.keep();
        self.dir
            .sync_all()
            .context("packet published; parent sync failed")
    }
}

pub(super) fn install_guest(input: &Path, digest: &str, tree: &Path, guest: &str) -> Result<()> {
    use mvm_build::guest_elf::validate_static_guest_elf;
    use mvm_core::{arch::GuestArch, crypto::image_verify::sha256_file};
    let input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(input)?;
    let metadata = input.metadata()?;
    // Bound staging and the SDK's slice-based static-ELF check to the signed
    // guest memory envelope, even if the input grows after being opened.
    let cap = super::package::MEMORY_MIB as u64 * 1024 * 1024;
    ensure!(
        metadata.is_file() && metadata.len() > 0 && metadata.len() <= cap,
        "invalid ELF extent"
    );
    let dirs = [
        tree.to_owned(),
        tree.join("etc"),
        tree.join("usr"),
        tree.join("usr/lib"),
        tree.join("usr/local"),
        tree.join("usr/local/bin"),
    ];
    for dir in dirs {
        fs::create_dir(&dir)?;
        fs::set_permissions(&dir, Permissions::from_mode(0o755))?;
    }
    let staged = tree.join(&guest[1..]);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&staged)?;
    ensure!(
        io::copy(&mut input.take(cap + 1), &mut file)? == metadata.len(),
        "changed ELF extent"
    );
    file.set_permissions(Permissions::from_mode(0o555))?;
    file.sync_all()?;
    drop(file);
    ensure!(
        sha256_file(&staged)? == digest,
        "staged ELF SHA-256 differs"
    );
    validate_static_guest_elf(&fs::read(&staged)?, &staged, GuestArch::X86_64)?;
    Ok(())
}

pub(super) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
