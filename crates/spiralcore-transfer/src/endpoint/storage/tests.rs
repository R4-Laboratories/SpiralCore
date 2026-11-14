use super::*;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Allow;
impl Authority for Allow {
    fn authorize(&self, _: &Context) -> Result<()> {
        Ok(())
    }
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self::at(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
    }
    fn at(timestamp: u128) -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-storage-{}-{}-{}",
            std::process::id(),
            timestamp,
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
        )))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn private_store_refuses_hardlinked_profile_without_resetting_it() {
    let coarse_clock = Fixture::at(1);
    let fixture = Fixture::at(1);
    assert_ne!(
        coarse_clock.0, fixture.0,
        "same-clock parallel fixtures must not share ownership"
    );
    drop(Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap());
    let profile = fixture.0.join(".spiralcore-profile-v1");
    let original = fs::read(&profile).unwrap();
    fs::hard_link(&profile, fixture.0.join("untrusted-alias")).unwrap();
    assert!(matches!(
        Endpoint::open(&fixture.0, Limits::default(), Allow),
        Err(Error::Storage)
    ));
    assert_eq!(fs::read(profile).unwrap(), original);
}

fn context() -> Context {
    Context {
        tenant: "storage-test".into(),
        domain: "local".into(),
        sender: "sender".into(),
        recipient: "receiver".into(),
        mailbox: "objects".into(),
        capability: "fixture".into(),
        authorization_scope: "exact-storage".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}

#[cfg(unix)]
#[test]
fn reopen_waits_for_a_transient_duplicate_of_the_original_lock() {
    let fixture = Fixture::new();
    let first = Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap();
    // dup and fork share the same open file description and its flock.
    let inherited = match &first.root {
        StoreRoot::Path { _owner, .. } => _owner.try_clone().unwrap(),
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        StoreRoot::Owned(_) => panic!("expected a Path endpoint"),
    };
    drop(first);
    let path = fixture.0.join(".owner");
    let next = storage::open_owner(&path).unwrap();
    assert!(matches!(
        next.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));

    // Release the genuine inherited lock at the first retry. Advance the poll
    // clock explicitly so a descheduled test process cannot consume the window.
    let clock = std::cell::Cell::new(std::time::Instant::now());
    let mut inherited = Some(inherited);
    storage::lock_reopened_owner_with(
        &next,
        &path,
        || clock.get(),
        |interval| {
            assert_eq!(interval, std::time::Duration::from_millis(2));
            drop(inherited.take().expect("original duplicate was still held"));
            clock.set(clock.get() + interval);
        },
    )
    .unwrap();
    assert!(inherited.is_none());
    assert!(matches!(
        Endpoint::open(&fixture.0, Limits::default(), Allow),
        Err(Error::Storage)
    ));
    drop(next);
    Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap();
}

#[cfg(unix)]
#[test]
fn contended_handoff_revalidates_the_profile_and_named_owner() {
    let fixture = Fixture::new();
    drop(Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap());
    let marker = fixture.0.join(".spiralcore-profile-v1");
    let path = fixture.0.join(".owner");
    let file = storage::open_owner(&path).unwrap();
    fs::write(&marker, b"unexpected profile").unwrap();
    assert_eq!(
        storage::lock_reopened_owner(&file, &path),
        Err(Error::Storage)
    );
    drop(file);
    assert_eq!(fs::read(&marker).unwrap(), b"unexpected profile");

    fs::write(&marker, storage::PROFILE_CONTENT).unwrap();
    let old = storage::open_owner(&path).unwrap();
    fs::rename(&path, fixture.0.join("previous")).unwrap();
    let replacement = storage::open_owner(&path).unwrap();
    replacement.try_lock().unwrap();
    assert_eq!(
        storage::lock_reopened_owner(&old, &path),
        Err(Error::Storage)
    );
    drop(old);
    assert!(matches!(
        storage::open_owner(&path).unwrap().try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
}

#[test]
fn published_bytes_survive_restart_and_fresh_read_rejects_corruption() {
    let fixture = Fixture::new();
    let context = context();
    let bytes = b"private exact storage\0readback";
    let id = object_id(bytes);
    let mut endpoint = Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap();
    assert!(matches!(
        Endpoint::open(&fixture.0, Limits::default(), Allow),
        Err(Error::Storage)
    ));
    crate::client::transfer(bytes, Mode::Raw, None, [0x24; 32], |frame| {
        endpoint.handle(&context, frame, 0)
    })
    .unwrap();
    assert_eq!(endpoint.read_object(&context, id).unwrap(), bytes);
    drop(endpoint);
    let mut reopened = Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap();
    assert_eq!(reopened.read_object(&context, id).unwrap(), bytes);
    let publication = fixture
        .0
        .join(hex(&context.namespace().unwrap()))
        .join(format!("{}.object", hex(&id)));
    let mut record = fs::read(&publication).unwrap();
    *record.last_mut().unwrap() ^= 1;
    fs::write(publication, record).unwrap();
    assert_eq!(reopened.read_object(&context, id), Err(Error::Integrity));
}

#[test]
fn staging_collection_refuses_hardlinks_without_removing_either_name() {
    let fixture = Fixture::new();
    drop(Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap());
    let namespace = fixture.0.join(hex(&context().namespace().unwrap()));
    storage::private_dir(&namespace).unwrap();
    let stage = namespace.join(".stage-123-0");
    let alias = fixture.0.join("stage-alias");
    drop(storage::create_new_file(&stage).unwrap());
    fs::hard_link(&stage, &alias).unwrap();
    assert!(matches!(
        Endpoint::open(&fixture.0, Limits::default(), Allow),
        Err(Error::Storage)
    ));
    assert!(stage.exists() && alias.exists());
}

#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn original_drive_absolute_syntax_refuses_aliases_without_store_mutation() {
        use std::path::{Component, Prefix};
        let fixture = Fixture::new();
        drop(Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap());
        let marker = fixture.0.join(".spiralcore-profile-v1");
        let original = fs::read(&marker).unwrap();
        let root_relative: PathBuf = fixture.0.components().skip(1).collect();
        let drive = match fixture.0.components().next().unwrap() {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => panic!("fixture must use a local drive"),
            },
            _ => panic!("fixture must be absolute"),
        };
        // Existing nonempty working directories cannot be initialized by the
        // old endpoint either. No global chdir or outside-fixture creation.
        for path in [
            PathBuf::from("."),
            PathBuf::from(format!("{}:", char::from(drive))),
            root_relative,
        ] {
            assert!(
                matches!(
                    Endpoint::open(&path, Limits::default(), Allow),
                    Err(Error::Unsupported)
                ),
                "original input must name its drive and root before IO"
            );
        }
        for suffix in [
            r"missing\..\child",
            r"candidate.",
            r"candidate ",
            r"missing.\child",
            r"missing \child",
        ] {
            assert!(Endpoint::open(&fixture.0.join(suffix), Limits::default(), Allow).is_err());
        }
        assert!(!fixture.0.join("child").exists());
        assert!(!fixture.0.join("candidate").exists());
        assert!(!fixture.0.join("missing").exists());
        assert_eq!(fs::read(marker).unwrap(), original);
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 2);
    }
}
