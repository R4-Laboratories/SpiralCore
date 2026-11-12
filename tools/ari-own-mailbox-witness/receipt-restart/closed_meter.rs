// Test-only collector for an exclusively owned, parked VM directory.
// The caller has joined the original helper before collection.
use std::fs::{self, File, OpenOptions};
use std::io::{Error, ErrorKind, Read, Result};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::time::Instant;

#[path = "spiralcore_guest_stream.rs"]
mod stream;

pub(super) const NAME: &str = stream::NAME;

pub(super) fn collect(parked: &Path, archive: &Path) -> Result<String> {
    let archive_parent = archive.parent().ok_or(ErrorKind::InvalidInput)?;
    let original = fs::symlink_metadata(parked)?;
    let owner = fs::symlink_metadata(archive_parent)?;
    if !original.is_dir()
        || !owner.is_dir()
        || original.uid() != owner.uid()
        || original.mode() & 0o777 != 0o700
        || owner.mode() & 0o777 != 0o700
    {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "expected private owned directories",
        ));
    }
    let source = parked.join(stream::NAME);
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&source)?;
    let meta = file.metadata()?;
    assert!(meta.is_file() && meta.nlink() == 1 && meta.mode() & 0o777 == 0o600);
    assert_eq!(meta.uid(), original.uid());
    let mut raw = String::new();
    (&mut file).take(32_769).read_to_string(&mut raw)?;
    assert!(raw.len() <= 32_768, "oversized recorder");
    let Some((exchanges, read, written, true)) = stream::completed(&raw) else {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "missing or invalid closed session",
        ));
    };
    if exchanges == 0 || read == 0 || written == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "missing measured exchange",
        ));
    }
    // Creating a fresh private directory makes collection no-clobber. In this
    // single-owner fixture no actor can insert another file into it.
    fs::DirBuilder::new().mode(0o700).create(archive)?;
    let dest = archive.join(stream::NAME);
    fs::rename(&source, &dest)?;
    let archived = fs::symlink_metadata(&dest)?;
    assert_eq!((archived.dev(), archived.ino()), (meta.dev(), meta.ino()));
    file.sync_all()?;
    File::open(archive)?.sync_all()?;
    File::open(archive_parent)?.sync_all()?;
    File::open(parked)?.sync_all()?;
    eprintln!("METER_SUMMARY {exchanges} {read} {written}");
    eprintln!("METER_START\n{raw}METER_END");
    Ok(raw)
}

pub(super) struct Timing(Instant, u64);
impl Timing {
    pub(super) fn start() -> Self {
        Self(Instant::now(), thread_us())
    }
    pub(super) fn record(self, role: &str) {
        eprintln!(
            "METER_TIME {role} {} {}",
            self.0.elapsed().as_micros(),
            thread_us() - self.1
        );
    }
}

fn thread_us() -> u64 {
    let mut ts = std::mem::MaybeUninit::uninit();
    // SAFETY: the output buffer is valid and read only after a successful call.
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, ts.as_mut_ptr()) },
        0
    );
    let ts = unsafe { ts.assume_init() };
    ts.tv_sec as u64 * 1_000_000 + ts.tv_nsec as u64 / 1_000
}

#[cfg(test)]
mod collection_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn missing_footer_and_occupied_slot_preserve_the_original() {
        use std::os::unix::fs::PermissionsExt;
        let mut builder = tempfile::Builder::new();
        builder.permissions(fs::Permissions::from_mode(0o700));
        let parked = builder.tempdir().unwrap();
        let home = builder.tempdir().unwrap();
        let slot = home.path().join("first-meter");
        let path = parked.path().join(NAME);
        let mut source = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        writeln!(source, "{}\nbegin 1\nend 1 10 15 0 0 false", stream::SCOPE).unwrap();
        let prefix = fs::read(&path).unwrap();
        assert_eq!(
            collect(parked.path(), &slot).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        assert_eq!(fs::read(&path).unwrap(), prefix);
        fs::set_permissions(home.path(), fs::Permissions::from_mode(0o770)).unwrap();
        assert_eq!(
            collect(parked.path(), &slot).unwrap_err().kind(),
            ErrorKind::PermissionDenied
        );
        fs::set_permissions(home.path(), fs::Permissions::from_mode(0o700)).unwrap();
        assert!(!slot.exists());

        writeln!(source, "finish 1").unwrap();
        drop(source);
        fs::create_dir(&slot).unwrap();
        fs::write(slot.join(NAME), b"earlier session").unwrap();
        let closed = fs::read(&path).unwrap();
        assert_eq!(
            collect(parked.path(), &slot).unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&path).unwrap(), closed);
        assert_eq!(fs::read(slot.join(NAME)).unwrap(), b"earlier session");
    }
}
