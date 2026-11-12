use super::*;
use std::{
    cell::RefCell,
    os::unix::fs::DirBuilderExt,
    sync::atomic::{AtomicU64, Ordering},
};
type FaultHook = fn(&'static str) -> Result<()>;
thread_local! { static HOOK: RefCell<Option<FaultHook>> = const { RefCell::new(None) }; }
pub(super) fn checkpoint(point: &'static str) -> Result<()> {
    HOOK.with(|hook| match *hook.borrow() {
        Some(callback) => callback(point),
        None => Ok(()),
    })
}
struct Fixture {
    base: PathBuf,
    root: PathBuf,
    key: InstallationKey,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "spiral-disk-barrier-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        Self {
            root: base.join("installation"),
            base,
            key: InstallationKey {
                installation: [71; 32],
                profile: [72; 32],
            },
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        HOOK.with(|h| *h.borrow_mut() = None);
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn request() -> EndpointDiskRequest {
    EndpointDiskRequest {
        tenant: [1; 32],
        session: [2; 32],
        parent_serial: 1,
        cache_bytes: 65536,
        staging_bytes: 65536,
    }
}
fn fail_publish(point: &'static str) -> Result<()> {
    if point == "publish-before-parent-sync" {
        Err(io::Error::other("injected disk parent sync failure").into())
    } else {
        Ok(())
    }
}
fn fail_remove(point: &'static str) -> Result<()> {
    if point == "remove-before-parent-sync" {
        Err(io::Error::other("injected disk removal sync failure").into())
    } else {
        Ok(())
    }
}
#[test]
fn postrename_sync_failure_keeps_creation_intent_and_blocks_generic_retirement() {
    let f = Fixture::new();
    let mut owner = InstallationOwner::initialize(&f.root, f.key).unwrap();
    let epoch = owner.begin().unwrap();
    HOOK.with(|h| *h.borrow_mut() = Some(fail_publish));
    let failed = owner.prepare_endpoint_disk(epoch, 1, request());
    assert!(
        failed.is_err(),
        "injected directory sync must fail preparation"
    );
    assert!(failed.err().unwrap().may_have_effects);
    HOOK.with(|h| *h.borrow_mut() = None);
    assert!(f.root.join(INVENTORY).exists());
    assert!(owner.finish(epoch).is_err());
    drop(owner);
    let mut owner = InstallationOwner::open(&f.root, f.key).unwrap();
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    owner.reconcile().unwrap();
}
#[test]
fn removal_sync_failure_retains_cleanup_identity_and_recovers_missing_directory() {
    let f = Fixture::new();
    let mut owner = InstallationOwner::initialize(&f.root, f.key).unwrap();
    let epoch = owner.begin().unwrap();
    let store = owner
        .prepare_endpoint_disk(epoch, 1, request())
        .ok()
        .unwrap();
    HOOK.with(|h| *h.borrow_mut() = Some(fail_remove));
    assert!(
        owner.cancel_unused_endpoint_disk(&store).is_err(),
        "injected removal sync must fail cleanup"
    );
    HOOK.with(|h| *h.borrow_mut() = None);
    assert!(!store.path.exists());
    assert_eq!(
        owner.storage.disk_snapshot(epoch).unwrap().entries[0].phase,
        3
    );
    assert!(owner.finish(epoch).is_err());
    drop(store);
    drop(owner);
    let mut owner = InstallationOwner::open(&f.root, f.key).unwrap();
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    owner.reconcile().unwrap();
}

fn crash_checkpoint(point: &'static str) -> Result<()> {
    use std::io::{Read, Write};
    if std::env::var("SPIRAL_DISK_TEST_POINT").ok().as_deref() != Some(point) {
        return Ok(());
    }
    println!("CHECKPOINT_READY");
    std::io::stdout().flush()?;
    let mut control = [0];
    let _ = std::io::stdin().read_exact(&mut control);
    // Child::wait may close stdin while the requested kill is being delivered.
    // EOF is never permission to continue the interrupted product operation.
    loop {
        std::thread::park();
    }
}
#[test]
fn publication_crash_helper() {
    let Some(base) = std::env::var_os("SPIRAL_DISK_TEST_BASE") else {
        return;
    };
    let root = PathBuf::from(base).join("installation");
    let key = InstallationKey {
        installation: [71; 32],
        profile: [72; 32],
    };
    let mut owner = InstallationOwner::initialize(&root, key).unwrap();
    let epoch = owner.begin().unwrap();
    HOOK.with(|h| *h.borrow_mut() = Some(crash_checkpoint));
    let store = owner
        .prepare_endpoint_disk(epoch, 1, request())
        .ok()
        .unwrap();
    owner.cancel_unused_endpoint_disk(&store).unwrap();
    panic!("requested checkpoint was never reached");
}
struct OriginalChild {
    child: std::process::Child,
    reader: Option<std::thread::JoinHandle<()>>,
}
impl Drop for OriginalChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
    }
}
#[test]
fn process_kills_at_publication_boundaries_do_not_skip_required_recovery() {
    use std::{
        io::{BufRead, BufReader},
        os::unix::process::ExitStatusExt,
        process::{Command, Stdio},
        sync::mpsc,
        time::Duration,
    };
    for (point, ambiguous, count) in [
        ("intent-file-written", false, 0),
        ("intent-file-synced", false, 0),
        ("publish-before-parent-sync", false, 1),
        ("creation-committed", false, 1),
        ("namespace-created", true, 0),
        ("active-committed", false, 1),
        ("cleanup-committed", false, 1),
        ("remove-before-parent-sync", false, 1),
        ("namespace-removal-committed", false, 1),
        ("inventory-empty-committed", false, 0),
        ("inventory-removed", false, 0),
    ] {
        let f = Fixture::new();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "installed_owner::endpoint_disks::tests::publication_crash_helper",
                "--nocapture",
            ])
            .env("SPIRAL_DISK_TEST_BASE", &f.base)
            .env("SPIRAL_DISK_TEST_POINT", point)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut original = OriginalChild {
            child,
            reader: None,
        };
        let mut output = BufReader::new(original.child.stdout.take().unwrap());
        let (tx, rx) = mpsc::sync_channel(1);
        original.reader = Some(std::thread::spawn(move || {
            let mut line = String::new();
            let ready = loop {
                line.clear();
                if output.read_line(&mut line).unwrap_or(0) == 0 {
                    break false;
                }
                if line.trim() == "CHECKPOINT_READY" {
                    break true;
                }
            };
            let _ = tx.send(ready);
            let _ = std::io::copy(&mut output, &mut std::io::sink());
        }));
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(10)),
            Ok(true),
            "{point}"
        );
        original.child.kill().unwrap();
        assert_eq!(
            original.child.wait().unwrap().signal(),
            Some(libc::SIGKILL),
            "{point}"
        );
        drop(original);
        let mut owner = InstallationOwner::open(&f.root, f.key).unwrap();
        if point != "inventory-removed" {
            assert!(owner.reconcile().is_err(), "{point}");
        }
        if ambiguous {
            assert!(owner.recover_endpoint_disks().is_err(), "{point}");
            assert!(owner.reconcile().is_err(), "{point}");
            assert!(
                fs::read_dir(&f.root).unwrap().any(|e| e
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .starts_with(PREFIX))
            );
        } else {
            assert_eq!(owner.recover_endpoint_disks().unwrap(), count, "{point}");
            owner.reconcile().unwrap();
            let epoch = owner.begin().unwrap();
            owner.finish(epoch).unwrap();
        }
    }
}

#[test]
fn directory_buffer_profile_rejects_unbounded_platform_allocation_inputs() {
    assert!(scan_limits(Some(0), false).is_err());
    assert!(scan_limits(Some(32768), false).is_ok());
    assert!(scan_limits(Some(32769), false).is_err());
    assert!(scan_limits(Some(u64::MAX), false).is_err());
    assert!(scan_limits(None, false).is_ok());
    assert!(scan_limits(None, true).is_err());
    assert!(scan_limits(Some(4096), true).is_err());
}
