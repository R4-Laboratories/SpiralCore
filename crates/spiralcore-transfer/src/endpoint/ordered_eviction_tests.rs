//! The accepted single-victim path must read the complete persisted victim exactly once.
use super::*;
use std::cell::Cell;

thread_local! {
    pub(super) static HEADER_READS: Cell<usize> = const { Cell::new(0) };
    pub(super) static FULL_READS: Cell<usize> = const { Cell::new(0) };
}

#[test]
fn an_ordered_read_uses_the_current_private_file_from_header_through_eof() {
    let root = metadata_tests::Fixture::new();
    let resources = crate::resource_profile::ResourceProfile::SmallV1;
    let context = crate::wan::context([7; 32]);
    let ns = context.namespace().unwrap();
    let mut endpoint = Endpoint::open_with_resource_profile(
        &root.0,
        resources.receiver_limits(),
        crate::wan::BenchAuthority,
        resources,
    )
    .unwrap();
    let body = [b"original publication".as_slice(), &[0; 17]].concat();
    let offer = Offer::cold(&body, Mode::Raw);
    endpoint.publish(&context, ns, &offer, &body, None).unwrap();
    HEADER_READS.with(|count| count.set(0));
    assert_eq!(
        endpoint.ordered_cache_read(&context, offer.object).unwrap(),
        (body.clone(), body.len() as u64 + 49)
    );
    assert_eq!(HEADER_READS.with(Cell::get), 1, "one fresh checked header");
    let path = endpoint.path(ns, offer.object);
    let mut bytes = fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    fs::write(&path, bytes).unwrap();
    assert_eq!(
        endpoint.ordered_cache_read(&context, offer.object),
        Err(Error::Integrity)
    );
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

#[test]
fn application_commit_returns_one_independent_disk_readback() {
    use crate::resource_profile::ResourceProfile;
    use crate::v1::{
        Capabilities, ObjectEndpoint, ObjectMode, ObjectSender, OrderedBinding, OrderedLane,
        OrderedObjectEndpoint,
    };
    let context = crate::wan::context([7; 32]);
    let root = metadata_tests::Fixture::new();
    let r = ResourceProfile::SmallV1;
    let caps = Capabilities::small_v1();
    let profile = caps.negotiate(&caps).unwrap();
    let endpoint = ObjectEndpoint::open_with_resource_profile(
        &root.0,
        r.receiver_limits(),
        crate::wan::BenchAuthority,
        profile,
        r,
    )
    .unwrap();
    let binding = OrderedBinding::new(
        &context,
        context.namespace().unwrap(),
        0,
        b"application\0",
        b"artifact\0",
    )
    .unwrap();
    let mut endpoint = OrderedObjectEndpoint::new(endpoint, binding.clone(), &context).unwrap();
    let mut sender =
        ObjectSender::new_with_resource_profile(ObjectMode::Whole, &context, None, profile, r)
            .unwrap();
    let expected = [b"independent exact bytes".as_slice(), &[0; 17]].concat();
    let mut recovered = None;
    let mut terminal = None;
    sender
        .send(
            &context,
            expected.clone(),
            binding.transfer_id(OrderedLane::Application, 0),
            false,
            |frames| {
                frames
                    .iter()
                    .map(|f| {
                        FULL_READS.with(|n| n.set(0));
                        let (reply, bytes) = endpoint.handle_application(0, &context, f, 1)?;
                        if reply.kind == 103 {
                            assert_eq!(
                                FULL_READS.with(Cell::get),
                                1,
                                "one fresh disk read after publication"
                            );
                            assert!(recovered.is_none());
                            recovered = bytes;
                            terminal = Some(f.clone());
                        } else {
                            assert!(bytes.is_none());
                        }
                        Ok(reply)
                    })
                    .collect()
            },
        )
        .unwrap();
    assert_eq!(recovered.unwrap(), expected);
    let id = object_id(&expected);
    let path = root
        .0
        .join(hex(&context.namespace().unwrap()))
        .join(format!("{}.object", hex(&id)));
    let mut contents = fs::read(&path).unwrap();
    *contents.last_mut().unwrap() ^= 1;
    fs::write(path, contents).unwrap();
    assert!(matches!(
        endpoint.handle_application(0, &context, &terminal.unwrap(), 2),
        Err(Error::Integrity)
    ));
    assert_eq!(endpoint.read_object(&context, id), Err(Error::Storage));
}
