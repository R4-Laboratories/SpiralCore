use super::{OwnedLaneNamespaces, OwnedStorePolicy};
use crate::{resource_profile::ResourceProfile, Error};

#[test]
fn small_v1_funds_overflow_lookahead() {
    let limits = ResourceProfile::SmallV1.receiver_limits();
    let lane = OwnedLaneNamespaces::One([1; 32]);
    assert!(matches!(
        OwnedStorePolicy::small_v1(&limits, 101, lane),
        Err(Error::Limit)
    ));
    assert!(OwnedStorePolicy::small_v1(&limits, 102, lane).is_ok());
    assert!(matches!(
        OwnedStorePolicy::small_v1(&limits, 102, OwnedLaneNamespaces::Two([1; 32], [1; 32])),
        Err(Error::Limit)
    ));
}
use super::OwnedStoreLease;
use crate::endpoint::Limits;
use std::{
    ffi::OsStr,
    fs::{self, File},
    os::{
        fd::AsFd,
        unix::fs::{DirBuilderExt, PermissionsExt},
    },
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

struct Store {
    parent: PathBuf,
    limits: Limits,
    lease: OwnedStoreLease,
}
impl Store {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let parent = std::env::temp_dir().join(format!(
            "sc-owned-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&parent).unwrap();
        fs::DirBuilder::new()
            .mode(0o700)
            .create(parent.join("store"))
            .unwrap();
        let limits = ResourceProfile::SmallV1.receiver_limits();
        let policy =
            OwnedStorePolicy::small_v1(&limits, 102, OwnedLaneNamespaces::One([1; 32])).unwrap();
        let lease = OwnedStoreLease::capture(
            Arc::new(File::open(&parent).unwrap()),
            OsStr::new("store"),
            Arc::new(File::open(parent.join("store")).unwrap()),
            policy,
        )
        .unwrap();
        Self {
            parent,
            limits,
            lease,
        }
    }
    fn claim(&self) {
        self.lease
            .claim(&self.limits, ResourceProfile::SmallV1)
            .unwrap();
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}

#[test]
fn capture_and_claim_keep_the_original_directory_and_lock() {
    let mut store = Store::new();
    let root = store.parent.join("store");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    store.claim();
    assert!(matches!(
        store.lease.claim(&store.limits, ResourceProfile::SmallV1),
        Err(Error::Storage)
    ));
    let contender = File::open(root.join(".owner")).unwrap();
    assert!(matches!(
        contender.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    let extra = store.lease.clone();
    assert!(store.lease.retire_contents().is_err());
    drop(extra);
    fs::rename(&root, store.parent.join("original")).unwrap();
    fs::create_dir(&root).unwrap();
    assert!(store.lease.check_lane([1; 32]).is_err());
    assert!(store.lease.retire_contents().is_err());
    assert!(store.parent.join("original/.owner").exists());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
}

#[test]
fn walk_charges_unknown_names_and_refuses_unenrolled_lanes() {
    let store = Store::new();
    store.claim();
    let root = store.parent.join("store");
    for i in 0..99 {
        File::create(root.join(format!(".other-{i}"))).unwrap();
    }
    assert!(store.lease.check_room(0).is_ok());
    assert_eq!(store.lease.check_room(1), Err(Error::Limit));
    File::create(root.join("one-extra")).unwrap();
    assert_eq!(store.lease.check_room(0), Err(Error::Limit));
    assert!(store.lease.open_namespace([2; 32], true).is_err());
    assert!(!root.join(crate::hex(&[2; 32])).exists());
}

#[test]
fn checked_opens_refuse_fifo_symlink_and_hardlink() {
    let store = Store::new();
    store.claim();
    let ns = store.lease.open_namespace([1; 32], true).unwrap();
    let root = store.parent.join("store").join(crate::hex(&[1; 32]));
    assert!(std::process::Command::new("/usr/bin/mkfifo")
        .arg(root.join("fifo"))
        .status()
        .unwrap()
        .success());
    for name in ["fifo", "link", "second"] {
        if name == "link" {
            std::os::unix::fs::symlink(root.join("fifo"), root.join(name)).unwrap();
        }
        if name == "second" {
            let file = File::create(root.join("first")).unwrap();
            file.set_permissions(fs::Permissions::from_mode(0o600))
                .unwrap();
            fs::hard_link(root.join("first"), root.join(name)).unwrap();
        }
        assert!(matches!(
            super::open_read_at(ns.as_fd(), OsStr::new(name)),
            Err(Error::Storage)
        ));
    }
}

fn private_content(path: impl AsRef<std::path::Path>, bytes: &[u8]) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

#[test]
fn invalid_original_marker_refuses_before_locking_owner() {
    let store = Store::new();
    let root = store.parent.join("store");
    private_content(root.join(".owner"), b"");
    private_content(root.join(".spiralcore-profile-v1"), b"wrong marker");
    let original = File::open(root.join(".owner")).unwrap();
    assert!(store
        .lease
        .claim(&store.limits, ResourceProfile::SmallV1)
        .is_err());
    assert!(
        original.try_lock().is_ok(),
        "invalid initialized marker must be detected before lock"
    );
    assert!(store.lease.0.claim.lock().unwrap().owner.is_none());
}

#[test]
fn failed_lock_retains_opened_fd_without_granting_exclusion() {
    let store = Store::new();
    let root = store.parent.join("store");
    private_content(root.join(".owner"), b"");
    private_content(
        root.join(".spiralcore-profile-v1"),
        super::super::PROFILE_CONTENT,
    );
    let other = File::open(root.join(".owner")).unwrap();
    other.try_lock().unwrap();
    assert!(store
        .lease
        .claim(&store.limits, ResourceProfile::SmallV1)
        .is_err());
    let state = store.lease.0.claim.lock().unwrap();
    assert!(state.started && !state.locked && !state.ready);
    assert!(state.owner.is_some());
    drop(state);
    drop(other);
    assert!(store
        .lease
        .claim(&store.limits, ResourceProfile::SmallV1)
        .is_err());
}

#[test]
fn failed_locked_recovery_preserves_stages_and_original_lock() {
    let store = Store::new();
    let root = store.parent.join("store");
    private_content(root.join(".owner"), b"");
    private_content(
        root.join(".spiralcore-profile-v1"),
        super::super::PROFILE_CONTENT,
    );
    let dir = root.join(crate::hex(&[1; 32]));
    fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    for i in 0..33 {
        private_content(dir.join(format!(".stage-2-{i}")), b"x");
    }
    assert_eq!(
        store.lease.claim(&store.limits, ResourceProfile::SmallV1),
        Err(Error::Limit)
    );
    assert_eq!(fs::read_dir(dir).unwrap().count(), 33);
    assert!(matches!(
        File::open(root.join(".owner")).unwrap().try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    assert!(store.lease.check_lane([1; 32]).is_err());
}

#[test]
fn held_parent_rename_and_clean_retirement_preserve_substitute() {
    let mut store = Store::new();
    store.claim();
    let moved = store.parent.with_extension("moved");
    fs::rename(&store.parent, &moved).unwrap();
    let substitute = store.parent.clone();
    store.parent = moved;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&substitute)
        .unwrap();
    private_content(substitute.join("keep"), b"sentinel");
    let dir = store.lease.open_namespace([1; 32], true).unwrap();
    let ns = crate::hex(&[1; 32]);
    let original_root = store.parent.join("store");
    private_content(original_root.join(ns).join(".stage-10-5"), b"incomplete");
    assert!(dir.sync_all().is_ok());
    let previous_owner = File::open(original_root.join(".owner")).unwrap();
    store.lease.retire_contents().unwrap();
    assert_eq!(fs::read_dir(original_root).unwrap().count(), 0);
    assert_eq!(fs::read(substitute.join("keep")).unwrap(), b"sentinel");
    assert!(matches!(
        previous_owner.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    fs::remove_file(substitute.join("keep")).unwrap();
    fs::remove_dir(substitute).unwrap();
}

#[test]
fn retirement_refuses_unknown_file_before_deleting_known_data() {
    let mut store = Store::new();
    store.claim();
    drop(store.lease.open_namespace([1; 32], true).unwrap());
    let root = store.parent.join("store");
    let dir = root.join(crate::hex(&[1; 32]));
    private_content(dir.join(".stage-1-2"), b"prior bytes\0");
    private_content(dir.join("foreign"), b"keep");
    assert_eq!(store.lease.retire_contents(), Err(Error::Storage));
    assert_eq!(fs::read(dir.join(".stage-1-2")).unwrap(), b"prior bytes\0");
    assert_eq!(fs::read(dir.join("foreign")).unwrap(), b"keep");
    assert!(root.join(".owner").exists());
    assert!(store.lease.check_lane([1; 32]).is_err());
}

#[test]
fn stage_substitution_refuses_publish_and_cleanup() {
    let store = Store::new();
    store.claim();
    let ns = [1; 32];
    let dir = store.lease.open_namespace(ns, true).unwrap();
    let name = OsStr::new(".stage-4-7");
    let original = super::create_new_at(dir.as_fd(), name).unwrap();
    let path = store.parent.join("store").join(crate::hex(&ns));
    fs::rename(path.join(name), path.join("moved")).unwrap();
    private_content(path.join(name), b"substitute");
    assert_eq!(
        store
            .lease
            .publish_at(ns, dir.as_fd(), name, "out.object".as_ref(), &original),
        Err(Error::Storage)
    );
    assert_eq!(
        super::unlink_checked_at(dir.as_fd(), name, &original),
        Err(Error::Storage)
    );
    assert_eq!(fs::read(path.join(name)).unwrap(), b"substitute");
    assert!(!path.join("out.object").exists());
}

#[test]
fn wrong_root_fd_and_unsafe_leaf_are_refused_without_storage_effects() {
    let store = Store::new();
    let parent = Arc::new(File::open(&store.parent).unwrap());
    let child = Arc::new(File::open(store.parent.join("store")).unwrap());
    for leaf in [".", "..", "/", "store/../store", "absent"] {
        assert!(OwnedStoreLease::capture(
            parent.clone(),
            leaf.as_ref(),
            child.clone(),
            store.lease.0.policy.clone()
        )
        .is_err());
    }
    private_content(store.parent.join("regular"), b"keep");
    assert!(OwnedStoreLease::capture(
        parent,
        "regular".as_ref(),
        Arc::new(File::open(store.parent.join("regular")).unwrap()),
        store.lease.0.policy.clone()
    )
    .is_err());
    assert_eq!(fs::read_dir(store.parent.join("store")).unwrap().count(), 0);
}

#[test]
fn opened_foreign_lane_blocks_retirement_before_any_unlink() {
    let mut store = Store::new();
    store.claim();
    let root = store.parent.join("store");
    let foreign = root.join(crate::hex(&[2; 32]));
    fs::DirBuilder::new().mode(0o700).create(&foreign).unwrap();
    private_content(foreign.join("keep"), b"foreign");
    assert_eq!(store.lease.check_room(0), Err(Error::Unauthorized));
    assert_eq!(store.lease.retire_contents(), Err(Error::Unauthorized));
    assert_eq!(fs::read(foreign.join("keep")).unwrap(), b"foreign");
    assert!(root.join(".owner").exists());
}

#[test]
fn oversized_components_are_refused_before_os_path_conversion() {
    let name = "s".repeat(4096);
    assert_eq!(super::io::component(OsStr::new(&name)), Err(Error::Storage));
}

#[cfg(target_os = "linux")]
#[test]
fn linux_mount_identity_uses_the_held_descriptors() {
    let temp = Store::new();
    let original = File::open(&temp.parent).unwrap();
    let reopened = File::open(temp.parent.join("store")).unwrap();
    let foreign = File::open("/proc").unwrap();
    assert_eq!(
        super::io::check_mount(original.as_fd(), reopened.as_fd()),
        Ok(())
    );
    assert_eq!(
        super::io::check_mount(original.as_fd(), foreign.as_fd()),
        Err(Error::Storage)
    );
}

#[test]
fn checked_walk_holds_four_transient_fds_at_file_verification() {
    const CHILD: &str = "SC_OWNED_FD_EXACT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact","endpoint::storage::owned::tests::checked_walk_holds_four_transient_fds_at_file_verification","--nocapture"])
            .env(CHILD,"1").output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("test result: ok. 1 passed"));
        assert!(stdout.contains("OWNED_SCAN_FD_DELTA=4"));
        return;
    }
    let store = Store::new();
    store.claim();
    let ns = [1; 32];
    let dir = store.lease.open_namespace(ns, true).unwrap();
    drop(super::create_new_at(dir.as_fd(), "entry".as_ref()).unwrap());
    drop(dir);
    let count = || {
        #[cfg(target_os = "linux")]
        let observer = "/proc/self/fd";
        #[cfg(target_os = "macos")]
        let observer = "/dev/fd";
        fs::read_dir(observer)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_str().unwrap().parse::<usize>().is_ok())
            .count()
            - 1
    };
    let baseline = count();
    let mut at_check = None;
    store
        .lease
        .visit(None, |parent, dir, name| {
            if parent == Some(ns) && name == "entry" {
                let file = super::open_read_at(dir, name)?;
                at_check = Some(count() - baseline);
                drop(file);
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(at_check, Some(4));
    assert_eq!(count(), baseline);
    println!("OWNED_SCAN_FD_DELTA=4");
}

fn two_publications_and_stage(root: &std::path::Path) -> (PathBuf, Vec<PathBuf>) {
    let dir = root.join(crate::hex(&[1; 32]));
    fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    let mut files = Vec::new();
    for body in [b"A\0", b"B\0"] {
        let id = crate::object_id(body);
        let path = dir.join(format!("{}.object", crate::hex(&id)));
        let mut bytes = b"SCPUB001".to_vec();
        bytes.extend_from_slice(&(body.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&id);
        bytes.push(0);
        bytes.extend_from_slice(body);
        private_content(&path, &bytes);
        files.push(path);
    }
    files.push(dir.join(".stage-1-2"));
    private_content(files.last().unwrap(), b"retained stage");
    (dir, files)
}

#[test]
fn excess_publications_refuse_recovery_and_retirement_before_unlink() {
    for recovery in [true, false] {
        let mut store = Store::new();
        store.limits.max_publications = 1;
        Arc::get_mut(&mut store.lease.0).unwrap().policy =
            OwnedStorePolicy::small_v1(&store.limits, 102, OwnedLaneNamespaces::One([1; 32]))
                .unwrap();
        let root = store.parent.join("store");
        if recovery {
            private_content(root.join(super::OWNER), b"");
            private_content(root.join(super::PROFILE), super::super::PROFILE_CONTENT);
        } else {
            store.claim();
        }
        let (_, files) = two_publications_and_stage(&root);
        let saved = files
            .iter()
            .map(|p| fs::read(p).unwrap())
            .collect::<Vec<_>>();
        let result = if recovery {
            store.lease.claim(&store.limits, ResourceProfile::SmallV1)
        } else {
            store.lease.retire_contents()
        };
        assert_eq!(result, Err(Error::Limit));
        for (path, bytes) in files.iter().zip(saved) {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        assert!(root.join(super::OWNER).exists());
    }
}

#[test]
fn missing_initialized_profile_preserves_remaining_contents() {
    let mut store = Store::new();
    store.claim();
    let root = store.parent.join("store");
    let (_, files) = two_publications_and_stage(&root);
    fs::remove_file(root.join(super::PROFILE)).unwrap();
    assert_eq!(store.lease.retire_contents(), Err(Error::Storage));
    assert!(root.join(super::OWNER).exists());
    assert!(files.iter().all(|path| path.is_file()));
}

#[test]
fn post_unlink_sync_failure_quarantines_the_original_locked_store() {
    let mut store = Store::new();
    store.claim();
    let root = store.parent.join("store");
    private_content(root.join(".profile-1-1"), b"interrupted creation");
    let contender = File::open(root.join(super::OWNER)).unwrap();
    let mut failures = 0;
    let outcome = store.lease.retire_with_sync(|_| {
        failures += 1;
        Err(Error::Storage)
    });
    assert_eq!(outcome, Err(Error::Storage));
    assert_eq!(failures, 1);
    assert!(
        !root.join(".profile-1-1").exists(),
        "failure follows the real unlink"
    );
    assert!(root.join(super::PROFILE).exists());
    assert!(matches!(
        contender.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    assert_eq!(store.lease.retire_contents(), Err(Error::Storage));
    assert!(store.lease.check_lane([1; 32]).is_err());
}
