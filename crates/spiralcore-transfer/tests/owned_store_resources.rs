#![cfg(any(target_os = "linux", target_os = "macos"))]

// Kernel observation of this one process. The single temporary enumeration FD
// is included by the kernel and subtracted before comparing lifecycle counts.
use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    v1::{Capabilities, ObjectEndpoint, OwnedLaneNamespaces, OwnedStoreLease, OwnedStorePolicy},
    wan,
};
use std::{fs, os::unix::fs::DirBuilderExt, sync::Arc};

fn fds() -> usize {
    #[cfg(target_os = "linux")]
    let dir = "/proc/self/fd";
    #[cfg(target_os = "macos")]
    let dir = "/dev/fd";
    let entries = fs::read_dir(dir)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    entries
        .iter()
        .filter(|e| e.file_name().to_str().unwrap().parse::<u32>().is_ok())
        .count()
        - 1
}

#[test]
fn maximal_admitted_owned_store_lifecycle() {
    let clock = std::time::Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let baseline = fds();
    let root = std::env::temp_dir().join(format!(
        "sc-owned-resources-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    let parent = Arc::new(fs::File::open(&root).unwrap());
    assert_eq!(fds(), baseline + 1);
    let resource = ResourceProfile::SmallV1;
    let limits = resource.receiver_limits();
    let profile = Capabilities::small_v1()
        .negotiate(&Capabilities::small_v1())
        .unwrap();
    let mut leases = Vec::new();
    let mut endpoints = Vec::new();
    for i in 0..2usize {
        let leaf = format!("store-{i}");
        let child = root.join(&leaf);
        fs::DirBuilder::new().mode(0o700).create(&child).unwrap();
        let c = wan::context([1 + i as u8; 32]);
        let lease = OwnedStoreLease::capture(
            parent.clone(),
            leaf.as_ref(),
            Arc::new(fs::File::open(child).unwrap()),
            OwnedStorePolicy::small_v1(
                &limits,
                102,
                OwnedLaneNamespaces::One(c.namespace().unwrap()),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(fds(), baseline + 2 + 2 * i);
        let retained = lease.clone();
        assert_eq!(fds(), baseline + 2 + 2 * i);
        let endpoint = ObjectEndpoint::open_at_with_resource_profile(
            &lease,
            limits.clone(),
            wan::BenchAuthority,
            profile,
            resource,
        )
        .unwrap();
        assert_eq!(fds(), baseline + 3 + 2 * i);
        endpoints.push(endpoint);
        leases.push(retained);
    }
    let peak_persistent = fds();
    assert_eq!(peak_persistent, baseline + 5);
    assert!(leases[0].retire_contents().is_err());
    drop(endpoints);
    assert_eq!(fds(), peak_persistent);
    for i in 0..2usize {
        let owner = fs::File::open(root.join(format!("store-{i}/.owner"))).unwrap();
        assert!(matches!(
            owner.try_lock(),
            Err(fs::TryLockError::WouldBlock)
        ));
    }
    for mut lease in leases {
        lease.retire_contents().unwrap();
    }
    assert_eq!(fds(), baseline + 1);
    drop(parent);
    assert_eq!(fds(), baseline);
    fs::remove_dir_all(root).unwrap();
    eprintln!("OWNED_RESOURCE stores=2 base_fds={baseline} persistent_fds={peak_persistent} final_fds={} observer_fds=1 wall_ns={} process_cpu_ns={}",fds(),clock.elapsed().as_nanos(),cpu.elapsed().as_nanos());
}
