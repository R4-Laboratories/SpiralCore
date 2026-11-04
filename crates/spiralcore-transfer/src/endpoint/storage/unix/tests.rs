use super::*;
use std::{cell::RefCell, io::Write, path::PathBuf};

#[derive(Default)]
struct Trace {
    directories: Vec<PathBuf>,
    fail: Option<PathBuf>,
}
thread_local! { static TRACE: RefCell<Option<Trace>> = const { RefCell::new(None) }; }
pub(super) fn before_sync(path: &Path) -> Result<()> {
    TRACE.with(|trace| {
        let mut trace = trace.borrow_mut();
        if let Some(trace) = trace.as_mut() {
            trace.directories.push(path.to_path_buf());
            if trace.fail.as_deref() == Some(path) {
                return Err(Error::Storage);
            }
        }
        Ok(())
    })
}
struct Watch;
impl Watch {
    fn new(fail: Option<PathBuf>) -> Self {
        TRACE.with(|trace| {
            assert!(trace.borrow().is_none());
            *trace.borrow_mut() = Some(Trace {
                fail,
                ..Default::default()
            });
        });
        Self
    }
    fn directories(&self) -> Vec<PathBuf> {
        TRACE.with(|trace| trace.borrow().as_ref().unwrap().directories.clone())
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        TRACE.with(|trace| *trace.borrow_mut() = None);
    }
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "publication-barriers-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
    fn stage(&self, directory: &Path) -> PathBuf {
        let path = directory.join(".stage-owned");
        let mut file = create_new_file(&path).unwrap();
        file.write_all(b"exact durable fixture bytes").unwrap();
        file.sync_all().unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn same_parent_publication_retains_one_actual_changed_directory_barrier() {
    let fixture = Fixture::new();
    let namespace = fixture.0.join("namespace");
    private_dir(&namespace).unwrap();
    let stage = fixture.stage(&namespace);
    let destination = namespace.join("exact.object");
    let watch = Watch::new(None);
    publish(&stage, &destination, &fixture.0).unwrap();
    assert_eq!(
        fs::read(&destination).unwrap(),
        b"exact durable fixture bytes"
    );
    assert!(!stage.exists());
    assert_eq!(watch.directories(), vec![namespace]);
}

#[test]
fn cross_parent_publication_refuses_before_renaming_or_syncing() {
    let fixture = Fixture::new();
    let namespace = fixture.0.join("namespace");
    private_dir(&namespace).unwrap();
    let stage = fixture.stage(&fixture.0);
    let destination = namespace.join("exact.object");
    let watch = Watch::new(None);
    assert_eq!(
        publish(&stage, &destination, &fixture.0),
        Err(Error::Storage)
    );
    assert_eq!(fs::read(&stage).unwrap(), b"exact durable fixture bytes");
    assert!(!destination.exists());
    assert!(watch.directories().is_empty());
}

#[test]
fn changed_directory_barrier_failure_never_returns_a_publication_receipt() {
    let fixture = Fixture::new();
    let root = fixture.0.join("cache");
    let context = crate::wan::context([188; 32]);
    let mut endpoint = crate::endpoint::Endpoint::open_with_policy(
        &root,
        crate::endpoint::Limits::default(),
        crate::wan::BenchAuthority,
        crate::endpoint::ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let namespace = root.join(crate::hex(&context.namespace().unwrap()));
    let watch = Watch::new(Some(namespace.clone()));
    let bytes = b"publication must not acknowledge a failed directory barrier";
    let result = crate::client::transfer_batched(
        bytes,
        crate::protocol::Mode::Raw,
        None,
        [189; 32],
        false,
        |frames| {
            frames
                .iter()
                .map(|frame| endpoint.handle(&context, frame, 0))
                .collect()
        },
    );
    assert!(matches!(result, Err(Error::Storage)), "{result:?}");
    assert_eq!(watch.directories(), vec![namespace.clone()]);
    // Rename visibility alone is not the successful durability acknowledgment.
    assert!(
        namespace
            .join(format!("{}.object", crate::hex(&crate::object_id(bytes))))
            .exists()
    );
}

#[test]
fn quota_length_uses_private_nofollow_metadata_without_reading_contents() {
    let fixture = Fixture::new();
    let path = fixture.stage(&fixture.0);
    let expected = b"exact durable fixture bytes".len() as u64;
    assert_eq!(quota_file_len(&path).unwrap(), expected);

    let alias = fixture.0.join("alias.object");
    std::os::unix::fs::symlink(&path, &alias).unwrap();
    assert_eq!(quota_file_len(&alias), Err(Error::Storage));
    fs::remove_file(&alias).unwrap();
    fs::hard_link(&path, &alias).unwrap();
    assert_eq!(quota_file_len(&path), Err(Error::Storage));
    assert_eq!(quota_file_len(&alias), Err(Error::Storage));
    fs::remove_file(&alias).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(quota_file_len(&path), Err(Error::Storage));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(quota_file_len(&path).unwrap(), expected);
    // The quota lookup grants no content access. Do not claim a root process
    // must observe EACCES: assert denial only when this actual File open fails.
    if File::open(&path).is_err() {
        assert!(open_read(&path).is_err());
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let mut actual = Vec::new();
    std::io::Read::read_to_end(&mut open_read(&path).unwrap(), &mut actual).unwrap();
    assert_eq!(actual, b"exact durable fixture bytes");
}

#[test]
fn quota_full_scan_still_refuses_overflow_and_preserves_exact_readback() {
    let fixture = Fixture::new();
    let root = fixture.0.join("cache");
    let context = crate::wan::context([190; 32]);
    let mut endpoint = crate::endpoint::Endpoint::open_with_policy(
        &root,
        crate::endpoint::Limits::default(),
        crate::wan::BenchAuthority,
        crate::endpoint::ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let bytes = [71; 64];
    crate::client::transfer(
        &bytes,
        crate::protocol::Mode::Raw,
        None,
        [191; 32],
        |frame| endpoint.handle(&context, frame, 0),
    )
    .unwrap();
    let ns = context.namespace().unwrap();
    assert_eq!(endpoint.disk_usage(ns).unwrap(), (113, 113, 1));
    assert_eq!(
        endpoint
            .read_object(&context, crate::object_id(&bytes))
            .unwrap(),
        bytes
    );
    // A distinct namespace still contributes to the global full scan.
    let foreign = root.join(crate::hex(&[192; 32]));
    private_dir(&foreign).unwrap();
    let foreign_file = foreign.join("retained.object");
    create_new_file(&foreign_file)
        .unwrap()
        .write_all(&[5; 33])
        .unwrap();
    fs::set_permissions(&foreign_file, fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(endpoint.disk_usage(ns).unwrap(), (113, 146, 2));
    endpoint.limits.namespace_bytes = 256;
    endpoint.limits.cache_bytes = 200;
    let result = crate::client::transfer(
        &[72; 32],
        crate::protocol::Mode::Raw,
        None,
        [193; 32],
        |frame| endpoint.handle(&context, frame, 0),
    );
    assert!(matches!(result, Err(Error::Limit)), "{result:?}");
    assert_eq!(endpoint.disk_usage(ns).unwrap(), (113, 146, 2));
    assert_eq!(
        endpoint
            .read_object(&context, crate::object_id(&bytes))
            .unwrap(),
        bytes
    );
    fs::set_permissions(&foreign_file, fs::Permissions::from_mode(0o600)).unwrap();
}
