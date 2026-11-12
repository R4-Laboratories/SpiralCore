//! The accepted single-victim path must read the complete persisted victim exactly once.
use super::*;
use std::cell::Cell;

thread_local! {
    pub(super) static FULL_READS: Cell<usize> = const { Cell::new(0) };
}

#[test]
fn single_and_multiple_victim_validation_precedes_removal() {
    let root = metadata_tests::Fixture::new();
    let resources = crate::resource_profile::ResourceProfile::SmallV1;
    let c = crate::wan::context([7; 32]);
    let ns = c.namespace().unwrap();
    let mut endpoint = Endpoint::open_with_resource_profile(
        &root.0,
        resources.receiver_limits(),
        crate::wan::BenchAuthority,
        resources,
    )
    .unwrap();
    let bytes = [0x73; 16384];
    let offer = Offer::cold(&bytes, Mode::Raw);
    let victim = OrderedEviction {
        id: offer.object,
        len: offer.len,
        disk: offer.len + 49,
    };
    endpoint.publish(&c, ns, &offer, &bytes, None).unwrap();
    FULL_READS.with(|count| count.set(0));
    endpoint
        .apply_ordered_evictions(&c, std::slice::from_ref(&victim))
        .unwrap();
    let reads = FULL_READS.with(Cell::get);
    eprintln!(
        "ORDERED_SINGLE_EVICTION bytes={} full_reads={reads}",
        bytes.len()
    );
    assert_eq!(reads, 1);
    assert!(!endpoint.path(ns, victim.id).exists());

    endpoint.publish(&c, ns, &offer, &bytes, None).unwrap();
    let bad_charge = OrderedEviction {
        disk: victim.disk + 32,
        ..victim
    };
    assert_eq!(
        endpoint.apply_ordered_evictions(&c, &[bad_charge]),
        Err(Error::Storage)
    );
    assert_eq!(endpoint.read_object(&c, victim.id).unwrap(), bytes);

    let second = Offer::cold(b"another original", Mode::Raw);
    endpoint
        .publish(&c, ns, &second, b"another original", None)
        .unwrap();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(endpoint.path(ns, second.object))
        .unwrap();
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(49)).unwrap();
    file.write_all(b"x").unwrap();
    file.sync_all().unwrap();
    drop(file);
    let poisoned = OrderedEviction {
        id: second.object,
        len: second.len,
        disk: second.len + 49,
    };
    assert_eq!(
        endpoint.apply_ordered_evictions(&c, std::slice::from_ref(&poisoned)),
        Err(Error::Storage)
    );
    assert!(endpoint.path(ns, second.object).exists());
    assert_eq!(
        endpoint.apply_ordered_evictions(&c, &[victim, poisoned]),
        Err(Error::Storage)
    );
    assert_eq!(endpoint.read_object(&c, offer.object).unwrap(), bytes);
    assert!(endpoint.path(ns, second.object).exists());
}
