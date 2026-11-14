#![cfg(any(target_os = "linux", target_os = "macos"))]

use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    v1::{
        Capabilities, ObjectEndpoint, ObjectMode, ObjectSender, OwnedLaneNamespaces,
        OwnedStoreLease, OwnedStorePolicy,
    },
    wan,
};
use std::{ffi::OsStr, fs, os::unix::fs::DirBuilderExt, sync::Arc};

use spiralcore_transfer::{
    endpoint::{Context, Limits},
    protocol::Frame,
    Error, Id,
};
use std::{io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};

struct Fixture {
    parent: PathBuf,
    child: PathBuf,
    lease: OwnedStoreLease,
    context: Context,
    limits: Limits,
}
impl Fixture {
    fn new(name: &str, limits: Limits) -> Self {
        let parent = std::env::temp_dir().join(format!(
            "sc-owned-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let child = parent.join("payload-1");
        for path in [&parent, &child] {
            fs::DirBuilder::new().mode(0o700).create(path).unwrap();
        }
        let context = wan::context([7; 32]);
        let lanes = OwnedLaneNamespaces::One(context.namespace().unwrap());
        let lease = OwnedStoreLease::capture(
            Arc::new(fs::File::open(&parent).unwrap()),
            OsStr::new("payload-1"),
            Arc::new(fs::File::open(&child).unwrap()),
            OwnedStorePolicy::small_v1(&limits, 102, lanes).unwrap(),
        )
        .unwrap();
        Self {
            parent,
            child,
            lease,
            context,
            limits,
        }
    }
    fn open(&self) -> spiralcore_transfer::Result<ObjectEndpoint<wan::BenchAuthority>> {
        let caps = Capabilities::small_v1();
        ObjectEndpoint::open_at_with_resource_profile(
            &self.lease,
            self.limits.clone(),
            wan::BenchAuthority,
            caps.negotiate(&caps)?,
            ResourceProfile::SmallV1,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}
fn send(
    endpoint: &mut ObjectEndpoint<wan::BenchAuthority>,
    context: &Context,
    body: &[u8],
    transfer: Id,
    warm: bool,
    mut before: impl FnMut(&Frame),
) -> spiralcore_transfer::Result<Id> {
    let profile = endpoint.profile();
    let mut sender = ObjectSender::new_with_resource_profile(
        ObjectMode::Raw,
        context,
        None,
        profile,
        ResourceProfile::SmallV1,
    )?;
    let mut request_bytes = 0;
    let mut reply_bytes = 0;
    let clock = std::time::Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let stats = sender.send(context, body.to_vec(), transfer, warm, |frames| {
        frames
            .iter()
            .map(|frame| {
                before(frame);
                request_bytes += frame.encode()?.len();
                let reply = endpoint.handle(context, frame, 0)?;
                reply_bytes += reply.encode()?.len();
                Ok(reply)
            })
            .collect()
    })?;
    assert_eq!(stats.object, spiralcore_transfer::object_id(body));
    assert_eq!(endpoint.read_object(context, stats.object)?, body);
    eprintln!("OWNED_V1 warm={warm} body={} request={request_bytes} reply={reply_bytes} wall_ns={} process_cpu_ns={}",body.len(),clock.elapsed().as_nanos(),cpu.elapsed().as_nanos());
    Ok(stats.object)
}

#[test]
fn original_directory_v1_roundtrips_trailing_nul() {
    let mut f = Fixture::new("roundtrip", ResourceProfile::SmallV1.receiver_limits());
    let retained = f.lease.clone();
    let mut endpoint = f.open().unwrap();
    assert!(f.open().is_err());
    let old = f.parent.clone();
    f.parent = f.parent.with_extension("moved");
    fs::rename(&old, &f.parent).unwrap();
    fs::DirBuilder::new().mode(0o700).create(&old).unwrap();
    fs::write(old.join("sentinel"), b"untouched").unwrap();
    for (index, warm) in [false, true].into_iter().enumerate() {
        send(
            &mut endpoint,
            &f.context,
            b"exact\0original\0\0",
            [index as u8 + 1; 32],
            warm,
            |_| {},
        )
        .unwrap();
    }
    assert_eq!(fs::read_dir(&old).unwrap().count(), 1);
    assert_eq!(fs::read(old.join("sentinel")).unwrap(), b"untouched");
    fs::remove_dir_all(old).unwrap();
    drop((endpoint, retained));
    f.lease.retire_contents().unwrap();
}

#[test]
fn commit_rechecks_quota_after_begin_and_preserves_prior_object() {
    let mut limits = ResourceProfile::SmallV1.receiver_limits();
    limits.max_publications = 2;
    let f = Fixture::new("quota", limits);
    let mut endpoint = f.open().unwrap();
    let ns = f
        .child
        .join(spiralcore_transfer::hex(&f.context.namespace().unwrap()));
    let original = b"accepted\0";
    let id = send(&mut endpoint, &f.context, original, [1; 32], false, |_| {}).unwrap();
    let foreign = ns.join("unknown-stem.object");
    let new = b"declined\0";
    let mut began = false;
    let result = send(&mut endpoint, &f.context, new, [2; 32], false, |frame| {
        if frame.kind == 1 {
            began = true;
        }
        if frame.kind == 3 {
            assert!(began);
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&foreign)
                .unwrap();
            file.write_all(b"counted").unwrap();
        }
    });
    assert_eq!(result, Err(Error::Limit));
    assert_eq!(fs::read(foreign).unwrap(), b"counted");
    assert_eq!(endpoint.read_object(&f.context, id).unwrap(), original);
    assert_eq!(
        endpoint.read_object(&f.context, spiralcore_transfer::object_id(new)),
        Err(Error::Unknown)
    );
}

const CHILD_MODE: &str = "SPIRALCORE_OWNED_PUBLICATION_INTERRUPT";
const CHILD_ROOT: &str = "SPIRALCORE_OWNED_PUBLICATION_PARENT";
const INTERRUPTED_BODY: &[u8] = b"interrupted original\0\0";

// Only the independently executed child owns this endpoint. The caller's
// normal current-authority checkpoint observes the actual durable boundary.
struct ExitAtPublication {
    namespace: PathBuf,
    target: PathBuf,
    after_rename: bool,
}
impl spiralcore_transfer::endpoint::Authority for ExitAtPublication {
    fn authorize(&self, context: &Context) -> spiralcore_transfer::Result<()> {
        wan::BenchAuthority.authorize(context)?;
        let exit = if self.after_rename {
            self.target.is_file()
        } else {
            match fs::read_dir(&self.namespace) {
                Ok(mut entries) => entries.any(|e| {
                    e.unwrap()
                        .file_name()
                        .as_encoded_bytes()
                        .starts_with(b".stage-")
                }),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
                Err(e) => return Err(e.into()),
            }
        };
        if exit {
            std::process::exit(74);
        }
        Ok(())
    }
}

#[test]
fn process_exit_recovers_only_the_original_stage_or_verified_publication() {
    let resources = ResourceProfile::SmallV1;
    let limits = resources.receiver_limits();
    if let Some(phase) = std::env::var_os(CHILD_MODE) {
        let parent = PathBuf::from(std::env::var_os(CHILD_ROOT).unwrap());
        let child = parent.join("payload-1");
        let c = wan::context([7; 32]);
        let ns = c.namespace().unwrap();
        let lease = OwnedStoreLease::capture(
            Arc::new(fs::File::open(&parent).unwrap()),
            "payload-1".as_ref(),
            Arc::new(fs::File::open(&child).unwrap()),
            OwnedStorePolicy::small_v1(&limits, 102, OwnedLaneNamespaces::One(ns)).unwrap(),
        )
        .unwrap();
        let profile = Capabilities::small_v1()
            .negotiate(&Capabilities::small_v1())
            .unwrap();
        let namespace = child.join(spiralcore_transfer::hex(&ns));
        let id = spiralcore_transfer::object_id(INTERRUPTED_BODY);
        let target = namespace.join(format!("{}.object", spiralcore_transfer::hex(&id)));
        let authority = ExitAtPublication {
            namespace,
            target,
            after_rename: phase == "published",
        };
        let mut endpoint = ObjectEndpoint::open_at_with_resource_profile(
            &lease, limits, authority, profile, resources,
        )
        .unwrap();
        let mut sender =
            ObjectSender::new_with_resource_profile(ObjectMode::Raw, &c, None, profile, resources)
                .unwrap();
        sender
            .send(&c, INTERRUPTED_BODY.to_vec(), [4; 32], false, |frames| {
                frames.iter().map(|f| endpoint.handle(&c, f, 0)).collect()
            })
            .unwrap();
        panic!("the selected publication boundary was never observed");
    }
    for phase in ["staged", "published"] {
        let f = Fixture::new(phase, limits.clone());
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "process_exit_recovers_only_the_original_stage_or_verified_publication",
            ])
            .env(CHILD_MODE, phase)
            .env(CHILD_ROOT, &f.parent)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(74));
        let ns = f
            .child
            .join(spiralcore_transfer::hex(&f.context.namespace().unwrap()));
        let names = || {
            fs::read_dir(&ns)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect::<Vec<_>>()
        };
        let id = spiralcore_transfer::object_id(INTERRUPTED_BODY);
        assert_eq!(names().len(), 1);
        if phase == "staged" {
            assert!(names()[0].as_encoded_bytes().starts_with(b".stage-"));
        }
        let mut recovered = f.open().unwrap();
        if phase == "staged" {
            assert_eq!(recovered.read_object(&f.context, id), Err(Error::Unknown));
            assert!(names().is_empty());
        } else {
            assert_eq!(
                recovered.read_object(&f.context, id).unwrap(),
                INTERRUPTED_BODY
            );
        }
    }
}

#[test]
fn negotiated_profile_and_captured_disk_limits_must_match_before_claim() {
    let f = Fixture::new("limits", ResourceProfile::SmallV1.receiver_limits());
    let general = Capabilities::full()
        .negotiate(&Capabilities::full())
        .unwrap();
    assert!(matches!(
        ObjectEndpoint::open_at_with_resource_profile(
            &f.lease,
            f.limits.clone(),
            wan::BenchAuthority,
            general,
            ResourceProfile::General
        ),
        Err(Error::Unsupported)
    ));
    let mut requested = f.limits.clone();
    requested.namespace_bytes -= 1;
    let profile = Capabilities::small_v1()
        .negotiate(&Capabilities::small_v1())
        .unwrap();
    assert!(matches!(
        ObjectEndpoint::open_at_with_resource_profile(
            &f.lease,
            requested,
            wan::BenchAuthority,
            profile,
            ResourceProfile::SmallV1
        ),
        Err(Error::Limit)
    ));
    assert_eq!(fs::read_dir(&f.child).unwrap().count(), 0);
    let mut ep = f.open().unwrap();
    assert_eq!(
        ep.read_object(&wan::context([99; 32]), [0; 32]),
        Err(Error::Unauthorized)
    );
}

#[test]
fn ordered_scan_requires_an_actual_directory_for_a_namespace_name() {
    use spiralcore_transfer::v1::{OrderedBinding, OrderedObjectEndpoint};
    let f = Fixture::new("non-directory", ResourceProfile::SmallV1.receiver_limits());
    let ep = f.open().unwrap();
    let ns = f.context.namespace().unwrap();
    let hostile = f.child.join(spiralcore_transfer::hex(&ns));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&hostile)
        .unwrap();
    file.write_all(b"unowned record").unwrap();
    file.sync_all().unwrap();
    let binding = OrderedBinding::new(&f.context, ns, 0, b"app", b"artifact").unwrap();
    assert!(matches!(
        OrderedObjectEndpoint::new(ep, binding, &f.context),
        Err(Error::Storage)
    ));
    assert_eq!(fs::read(hostile).unwrap(), b"unowned record");
    let lock = fs::File::open(f.child.join(".owner")).unwrap();
    assert!(matches!(lock.try_lock(), Err(fs::TryLockError::WouldBlock)));
}
