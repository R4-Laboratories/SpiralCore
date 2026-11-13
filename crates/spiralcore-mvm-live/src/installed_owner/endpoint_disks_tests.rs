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
    mvm_core::util::test_env::isolated_lock_probe(
        probe_postrename_sync_failure_keeps_creation_intent_and_blocks_generic_retirement,
    );
}

fn probe_postrename_sync_failure_keeps_creation_intent_and_blocks_generic_retirement() {
    let f = Fixture::new();
    let mut owner = InstallationOwner::initialize(&f.root, f.key).unwrap();
    let epoch = owner.begin().unwrap();
    HOOK.with(|h| *h.borrow_mut() = Some(fail_publish));
    let failed = owner.prepare_endpoint_disk(epoch, 1, request());
    assert!(
        failed.is_err(),
        "injected directory sync must fail preparation"
    );
    let failure = failed.err().unwrap();
    assert!(failure.may_have_effects);
    assert!(
        matches!(failure.error, OwnerError::Io(e) if e.to_string() == "injected disk parent sync failure")
    );
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
    mvm_core::util::test_env::isolated_lock_probe(probe_removal_sync);
}

fn probe_removal_sync() {
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
    mvm_core::util::test_env::isolated_lock_probe(probe_publication_crashes);
}

fn probe_publication_crashes() {
    use std::{
        io::{BufRead, BufReader},
        os::unix::process::ExitStatusExt,
        process::{Command, Stdio},
        sync::mpsc,
        time::Duration,
    };
    let helper = format!(
        "{}::publication_crash_helper",
        module_path!().split_once("::").unwrap().1
    );
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
            .args(["--exact", &helper, "--nocapture"])
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

#[test]
fn live_retirement_removes_only_its_store_while_another_holder_remains() {
    mvm_core::util::test_env::isolated_lock_probe(probe_live_retirement);
}

fn probe_live_retirement() {
    use std::os::unix::fs::OpenOptionsExt;
    let f = Fixture::new();
    let mut owner = InstallationOwner::initialize(&f.root, f.key).unwrap();
    let epoch = owner.begin().unwrap();
    let mut store = owner
        .prepare_endpoint_disk(epoch, 1, request())
        .ok()
        .unwrap();
    owner.check_endpoint_disk(&store).unwrap();
    let binding = store.binding();
    assert_eq!(binding.key, f.key);
    assert_eq!(binding.generation, epoch.generation());
    assert_eq!(binding.reservation_serial, 1);
    assert_eq!(binding.session, request().session);
    let sibling = owner
        .prepare_endpoint_disk(
            epoch,
            2,
            EndpointDiskRequest {
                session: [3; 32],
                ..request()
            },
        )
        .ok()
        .unwrap();
    let holder = HolderLease::acquire(&f.root, f.key, epoch.generation()).unwrap();
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(store.path.join("cache-object"))
        .unwrap();
    file.write_all(b"original cache bytes").unwrap();
    file.sync_all().unwrap();
    drop(file);
    fs::write(f.base.join("audit-excluded"), b"retained audit").unwrap();
    owner.retire_endpoint_disk(&mut store).unwrap();
    assert!(!store.path.exists());
    assert!(sibling.path.is_dir());
    assert_eq!(
        fs::read(f.base.join("audit-excluded")).unwrap(),
        b"retained audit"
    );
    assert_eq!(owner.storage.disk_snapshot(epoch).unwrap().entries.len(), 1);
    owner.retire_endpoint_disk(&mut store).unwrap();
    owner.cancel_unused_endpoint_disk(&sibling).unwrap();
    assert!(
        owner.finish(epoch).is_err(),
        "live holder still fences global retirement"
    );
    drop(holder);
    // A failed global finish is intentionally fenced; use the original
    // process-exclusion recovery path rather than resetting that owner.
    drop(owner);
    let mut owner = InstallationOwner::open(&f.root, f.key).unwrap();
    owner.reconcile().unwrap();
}

fn fail_final_publish(point: &'static str) -> Result<()> {
    thread_local! { static COUNT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
    if point == "publish-before-parent-sync" {
        let n = COUNT.with(|c| {
            let n = c.get() + 1;
            c.set(n);
            n
        });
        if n == 2 {
            return Err(io::Error::other("injected final inventory sync failure").into());
        }
    }
    Ok(())
}
#[test]
fn live_cleanup_resumes_after_removal_or_final_publication_failure() {
    mvm_core::util::test_env::isolated_lock_probe(probe_live_cleanup);
}

fn probe_live_cleanup() {
    for hook in [fail_remove as FaultHook, fail_final_publish as FaultHook] {
        let f = Fixture::new();
        let mut owner = InstallationOwner::initialize(&f.root, f.key).unwrap();
        let epoch = owner.begin().unwrap();
        let mut store = owner
            .prepare_endpoint_disk(epoch, 1, request())
            .ok()
            .unwrap();
        HOOK.with(|h| *h.borrow_mut() = Some(hook));
        assert!(owner.retire_endpoint_disk(&mut store).is_err());
        HOOK.with(|h| *h.borrow_mut() = None);
        assert!(!store.path.exists());
        eprintln!(
            "retire diagnostic: nlink={} namespace_synced={} inventory_phases={:?}",
            store.directory.metadata().unwrap().nlink(),
            store.retirement == RetirementProgress::NamespaceSynced,
            owner
                .storage
                .disk_snapshot(epoch)
                .unwrap()
                .entries
                .iter()
                .map(|e| e.phase)
                .collect::<Vec<_>>()
        );
        owner.retire_endpoint_disk(&mut store).unwrap();
        owner.retire_endpoint_disk(&mut store).unwrap();
        assert!(!f.root.join(INVENTORY).exists());
        owner.finish(epoch).unwrap();
    }
}

#[test]
fn live_retirement_cannot_infer_progress_from_missing_active_or_cleanup_names() {
    mvm_core::util::test_env::isolated_lock_probe(
        probe_live_retirement_cannot_infer_progress_from_missing_active_or_cleanup_names,
    );
}

fn probe_live_retirement_cannot_infer_progress_from_missing_active_or_cleanup_names() {
    let f = Fixture::new();
    let mut owner = InstallationOwner::initialize(&f.root, f.key).unwrap();
    let epoch = owner.begin().unwrap();
    let mut store = owner
        .prepare_endpoint_disk(epoch, 1, request())
        .ok()
        .unwrap();
    fs::remove_dir(&store.path).unwrap();
    let before = fs::read(f.root.join(INVENTORY)).unwrap();
    assert!(owner.retire_endpoint_disk(&mut store).is_err());
    assert_eq!(fs::read(f.root.join(INVENTORY)).unwrap(), before);
    let mut snapshot = owner.storage.disk_snapshot(epoch).unwrap();
    snapshot.entries[0].phase = 3;
    owner.storage.publish_disks(&snapshot).unwrap();
    let before = fs::read(f.root.join(INVENTORY)).unwrap();
    assert!(
        owner.retire_endpoint_disk(&mut store).is_err(),
        "a durable intent alone is not this live store's unlink proof"
    );
    assert_eq!(fs::read(f.root.join(INVENTORY)).unwrap(), before);
    drop(store);
    drop(owner);
    let mut owner = InstallationOwner::open(&f.root, f.key).unwrap();
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    owner.reconcile().unwrap();
}

#[test]
fn live_retirement_missing_entry_refuses_until_original_remove_is_synced() {
    mvm_core::util::test_env::isolated_lock_probe(probe_missing_entry);
}

fn probe_missing_entry() {
    let f = Fixture::new();
    let mut owner = InstallationOwner::initialize(&f.root, f.key).unwrap();
    let epoch = owner.begin().unwrap();
    let mut store = owner
        .prepare_endpoint_disk(epoch, 1, request())
        .ok()
        .unwrap();
    HOOK.with(|h| *h.borrow_mut() = Some(fail_remove));
    assert!(owner.retire_endpoint_disk(&mut store).is_err());
    HOOK.with(|h| *h.borrow_mut() = None);
    let snapshot = owner.storage.disk_snapshot(epoch).unwrap();
    fs::remove_file(f.root.join(INVENTORY)).unwrap();
    assert!(owner.retire_endpoint_disk(&mut store).is_err());
    assert!(!f.root.join(INVENTORY).exists());
    owner.storage.publish_disks(&snapshot).unwrap();
    owner.retire_endpoint_disk(&mut store).unwrap();
    owner.finish(epoch).unwrap();
}

#[test]
fn live_retirement_does_not_adopt_a_recreated_namespace() {
    mvm_core::util::test_env::isolated_lock_probe(probe_recreated_namespace);
}

fn probe_recreated_namespace() {
    use std::os::unix::fs::OpenOptionsExt;
    let f = Fixture::new();
    let mut owner = InstallationOwner::initialize(&f.root, f.key).unwrap();
    let epoch = owner.begin().unwrap();
    let mut store = owner
        .prepare_endpoint_disk(epoch, 1, request())
        .ok()
        .unwrap();
    HOOK.with(|h| *h.borrow_mut() = Some(fail_remove));
    assert!(owner.retire_endpoint_disk(&mut store).is_err());
    HOOK.with(|h| *h.borrow_mut() = None);
    let before = fs::read(f.root.join(INVENTORY)).unwrap();
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&store.path)
        .unwrap();
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(store.path.join("foreign"))
        .unwrap();
    file.write_all(b"preserve replacement").unwrap();
    drop(file);
    assert!(owner.retire_endpoint_disk(&mut store).is_err());
    assert_eq!(fs::read(f.root.join(INVENTORY)).unwrap(), before);
    assert_eq!(
        fs::read(store.path.join("foreign")).unwrap(),
        b"preserve replacement"
    );
    fs::remove_file(store.path.join("foreign")).unwrap();
    fs::remove_dir(&store.path).unwrap();
    owner.retire_endpoint_disk(&mut store).unwrap();
    owner.finish(epoch).unwrap();
}
