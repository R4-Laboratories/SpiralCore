use spiralcore_transfer::{
    endpoint::{Authority, Context, Endpoint, Limits},
    protocol::{Frame, Mode, Offer},
    Error, Result,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-endpoint-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn context() -> Context {
    Context {
        tenant: "t".into(),
        domain: "d".into(),
        sender: "s".into(),
        recipient: "r".into(),
        mailbox: "m".into(),
        capability: "c".into(),
        authorization_scope: "objects".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}
#[derive(Clone)]
struct Policy(Arc<AtomicBool>);
impl Authority for Policy {
    fn authorize(&self, c: &Context) -> Result<()> {
        if self.0.load(Ordering::SeqCst) && *c == context() {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
fn policy() -> Policy {
    Policy(Arc::new(AtomicBool::new(true)))
}
fn frame(kind: u8, seq: u64, body: Vec<u8>) -> Frame {
    Frame {
        kind,
        seq,
        body,
        transfer: [9; 32],
    }
}
fn raw(bytes: &[u8]) -> Vec<u8> {
    let mut b = vec![0];
    b.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    b.extend_from_slice(bytes);
    b
}

#[test]
fn sparse_copy_insert_is_durable_and_base_scoped() {
    use spiralcore_transfer::{client, delta, object_id};
    let t = Temp::new();
    let c = context();
    let base = vec![42; 65537];
    let target = [b"prefix".as_slice(), &base, b"\0"].concat();
    let mut ep = Endpoint::open(&t.0, Limits::default(), policy()).unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    let offer = Offer {
        mode: Mode::SparseV1,
        len: target.len() as u64,
        object: object_id(&target),
        manifest: None,
        base: Some(object_id(&base)),
    };
    assert_eq!(Offer::decode(&offer.encode().unwrap()).unwrap(), offer);
    assert_eq!(
        ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 1)
            .unwrap()
            .kind,
        101
    );
    let recipe = delta::encode(&base, &target).unwrap().unwrap();
    let count = recipe.frames.len();
    for (i, body) in recipe.frames.into_iter().enumerate() {
        ep.handle(&c, &frame(2, i as u64 + 1, body), 2).unwrap();
    }
    assert_eq!(ep.read_object(&c, offer.object), Err(Error::Unknown));
    assert_eq!(
        ep.handle(&c, &frame(3, count as u64 + 1, vec![]), 3)
            .unwrap()
            .kind,
        103
    );
    drop(ep);
    let mut ep = Endpoint::open(&t.0, Limits::default(), policy()).unwrap();
    assert_eq!(ep.read_object(&c, offer.object).unwrap(), target);
    let mut missing = offer.clone();
    missing.base = Some([0xff; 32]);
    missing.object = [0xab; 32];
    assert_eq!(
        ep.handle(&c, &frame(1, 0, missing.encode().unwrap()), 4),
        Err(Error::Missing)
    );
}

#[test]
fn retained_graph_is_used_and_eviction_is_recoverable() {
    use spiralcore_transfer::{client, dag::Graph};
    let t = Temp::new();
    let c = context();
    let base = vec![51; 4096];
    let mut target = base.clone();
    target[29] ^= 1;
    let graph = Graph::fold(&base, 262144).unwrap();
    let mut ep = Endpoint::open(&t.0, Limits::default(), policy()).unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    assert!(ep.graph_cache_stats().resident_bytes > 0);
    client::transfer(&target, Mode::Dag, Some(&graph), [2; 32], |f| {
        ep.handle(&c, f, 1)
    })
    .unwrap();
    assert_eq!(ep.graph_cache_stats().hits, 1);
    assert_eq!(
        ep.read_object(&c, spiralcore_transfer::object_id(&target))
            .unwrap(),
        target
    );
    drop(ep);
    let mut ep = Endpoint::open(
        &t.0,
        Limits {
            graph_cache_bytes: 0,
            ..Limits::default()
        },
        policy(),
    )
    .unwrap();
    target[31] ^= 2;
    client::transfer(&target, Mode::Dag, Some(&graph), [3; 32], |f| {
        ep.handle(&c, f, 2)
    })
    .unwrap();
    assert_eq!(ep.graph_cache_stats().hits, 0);
    assert_eq!(ep.graph_cache_stats().misses, 1);
    assert_eq!(ep.graph_cache_stats().resident_bytes, 0);
}

#[test]
fn sparse_replay_cancel_bad_digest_and_revocation_never_commit() {
    use spiralcore_transfer::{client, delta, object_id};
    let t = Temp::new();
    let c = context();
    let p = policy();
    let base = vec![19; 8192];
    let mut target = base.clone();
    target.insert(0, 8);
    let mut ep = Endpoint::open(&t.0, Limits::default(), p.clone()).unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    let offer = Offer {
        mode: Mode::SparseV1,
        len: target.len() as u64,
        object: object_id(&target),
        manifest: None,
        base: Some(object_id(&base)),
    };
    let body = delta::encode(&base, &target)
        .unwrap()
        .unwrap()
        .frames
        .remove(0);
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 1)
        .unwrap();
    let data = frame(2, 1, body.clone());
    let ack = ep.handle(&c, &data, 2).unwrap();
    assert_eq!(
        u64::from_le_bytes(ack.body.clone().try_into().unwrap()),
        target.len() as u64
    );
    assert_eq!(ep.handle(&c, &data, 3).unwrap(), ack);
    ep.handle(&c, &frame(6, 2, vec![]), 4).unwrap();
    assert_eq!(ep.read_object(&c, offer.object), Err(Error::Unknown));
    for (id, bad_digest) in [(2, true), (3, false)] {
        let mut claimed = offer.clone();
        if bad_digest {
            claimed.object = [0x88; 32];
        }
        let f = |kind, seq, body| Frame {
            kind,
            seq,
            body,
            transfer: [id; 32],
        };
        ep.handle(&c, &f(1, 0, claimed.encode().unwrap()), 5)
            .unwrap();
        ep.handle(&c, &f(2, 1, body.clone()), 6).unwrap();
        if !bad_digest {
            p.0.store(false, Ordering::SeqCst);
        }
        assert_eq!(
            ep.handle(&c, &f(3, 2, vec![]), 7),
            Err(if bad_digest {
                Error::Integrity
            } else {
                Error::Unauthorized
            })
        );
        p.0.store(true, Ordering::SeqCst);
        assert_eq!(ep.read_object(&c, claimed.object), Err(Error::Unknown));
    }
}

#[test]
fn sparse_needs_no_dag_and_rejects_unreserved_target() {
    use spiralcore_transfer::{client, delta, object_id};
    let t = Temp::new();
    let c = context();
    let base = vec![17; 8192];
    let mut ep = Endpoint::open(
        &t.0,
        Limits {
            max_nodes: 1,
            resident_bytes: 3 << 20,
            ..Limits::default()
        },
        policy(),
    )
    .unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    assert_eq!(ep.graph_cache_stats().resident_bytes, 0);
    let target = [base.as_slice(), b"\0"].concat();
    let mut offer = Offer {
        mode: Mode::SparseV1,
        len: target.len() as u64,
        object: object_id(&target),
        manifest: None,
        base: Some(object_id(&base)),
    };
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 1)
        .unwrap();
    ep.handle(
        &c,
        &frame(
            2,
            1,
            delta::encode(&base, &target)
                .unwrap()
                .unwrap()
                .frames
                .remove(0),
        ),
        2,
    )
    .unwrap();
    ep.handle(&c, &frame(3, 2, vec![]), 3).unwrap();
    assert_eq!(ep.graph_cache_stats().resident_bytes, 0);
    assert_eq!(ep.read_object(&c, offer.object).unwrap(), target);
    offer.len = 2 << 20;
    offer.object = [0x55; 32];
    let mut begin = frame(1, 0, offer.encode().unwrap());
    begin.transfer = [4; 32];
    assert_eq!(ep.handle(&c, &begin, 4), Err(Error::Limit));
}

#[test]
fn cached_graph_never_bypasses_disk_integrity_or_namespace() {
    use spiralcore_transfer::{client, dag::Graph, object_id};
    struct AnyScope;
    impl Authority for AnyScope {
        fn authorize(&self, _: &Context) -> Result<()> {
            Ok(())
        }
    }
    let t = Temp::new();
    let c = context();
    let base = vec![51; 4096];
    let graph = Graph::fold(&base, 262144).unwrap();
    let mut target = base.clone();
    target[0] ^= 1;
    let mut ep = Endpoint::open(&t.0, Limits::default(), AnyScope).unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    for change in 0..4 {
        let mut other = c.clone();
        match change {
            0 => other.recipient = "other".into(),
            1 => other.policy_epoch += 1,
            2 => other.execution_epoch += 1,
            _ => other.capability = "other".into(),
        }
        assert_eq!(
            client::transfer(&target, Mode::Dag, Some(&graph), [2; 32], |f| ep
                .handle(&other, f, 1))
            .unwrap_err(),
            Error::Missing
        );
    }
    let path =
        t.0.join(spiralcore_transfer::hex(&c.namespace().unwrap()))
            .join(format!(
                "{}.object",
                spiralcore_transfer::hex(&object_id(&base))
            ));
    let mut data = std::fs::read(&path).unwrap();
    *data.last_mut().unwrap() ^= 1;
    std::fs::write(path, data).unwrap();
    assert_eq!(
        client::transfer(&target, Mode::Dag, Some(&graph), [2; 32], |f| ep
            .handle(&c, f, 2))
        .unwrap_err(),
        Error::Integrity
    );
    assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
}

#[test]
fn pinned_graph_survives_eviction_pressure_then_releases_after_cancel() {
    use spiralcore_transfer::{client, dag::Graph};
    let t = Temp::new();
    let c = context();
    let a = vec![51; 4096];
    let b = vec![61; 4096];
    let mut ep = Endpoint::open(
        &t.0,
        Limits {
            graph_cache_entries: 1,
            ..Limits::default()
        },
        policy(),
    )
    .unwrap();
    client::transfer(&a, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    let ga = Graph::fold(&a, 262144).unwrap();
    let mut at = a.clone();
    at[0] ^= 1;
    let gt = Graph::fold(&at, 262144).unwrap();
    let offer = Offer {
        mode: Mode::Dag,
        len: at.len() as u64,
        object: gt.manifest.object,
        manifest: Some(gt.manifest),
        base: Some(ga.manifest.id()),
    };
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 1)
        .unwrap();
    client::transfer(&b, Mode::Raw, None, [2; 32], |f| ep.handle(&c, f, 2)).unwrap();
    assert_eq!(ep.graph_cache_stats().entries, 1);
    assert_eq!(ep.graph_cache_stats().evictions, 0);
    ep.handle(&c, &frame(6, 1, vec![]), 3).unwrap();
    client::transfer(&vec![71; 4096], Mode::Raw, None, [3; 32], |f| {
        ep.handle(&c, f, 4)
    })
    .unwrap();
    assert_eq!(ep.graph_cache_stats().evictions, 1);
    client::transfer(&at, Mode::Dag, Some(&ga), [4; 32], |f| ep.handle(&c, f, 5)).unwrap();
    assert_eq!(ep.graph_cache_stats().misses, 1);
}

#[test]
fn optional_graphs_are_evicted_for_verified_reads_and_published_resume() {
    use spiralcore_transfer::{client, object_id};
    for resume in [false, true] {
        let t = Temp::new();
        let c = context();
        let large = vec![37; 2 << 20];
        let mut ep = Endpoint::open(
            &t.0,
            Limits {
                max_nodes: 1,
                ..Limits::default()
            },
            policy(),
        )
        .unwrap();
        client::transfer(&large, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
        drop(ep);
        let mut ep = Endpoint::open(
            &t.0,
            Limits {
                resident_bytes: 5 << 20,
                ..Limits::default()
            },
            policy(),
        )
        .unwrap();
        let mut x = 77u64;
        let small: Vec<u8> = (0..65536)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        client::transfer(&small, Mode::Raw, None, [2; 32], |f| ep.handle(&c, f, 1)).unwrap();
        assert!(ep.graph_cache_stats().resident_bytes > 1 << 20);
        if resume {
            let offer = Offer::cold(&large, Mode::Raw);
            let mut body = offer.id().unwrap().to_vec();
            body.extend_from_slice(&offer.encode().unwrap());
            assert_eq!(ep.handle(&c, &frame(4, 0, body), 2).unwrap().kind, 103);
        } else {
            assert_eq!(ep.read_object(&c, object_id(&large)).unwrap(), large);
        }
        assert!(ep.graph_cache_stats().evictions > 0);
    }
}

#[test]
fn durable_cold_transfer_retry_and_restart() {
    let t = Temp::new();
    let p = policy();
    let c = context();
    let bytes = b"exact\0bytes\0";
    let offer = Offer::cold(bytes, Mode::Raw);
    let begin = frame(1, 0, offer.encode().unwrap());
    let mut ep = Endpoint::open(&t.0, Limits::default(), p.clone()).unwrap();
    assert_eq!(ep.handle(&c, &begin, 0).unwrap().kind, 101);
    let data = frame(2, 1, raw(bytes));
    let response = ep.handle(&c, &data, 1).unwrap();
    assert_eq!(ep.handle(&c, &data, 2).unwrap(), response);
    assert_eq!(ep.read_object(&c, offer.object), Err(Error::Unknown));
    assert_eq!(ep.handle(&c, &frame(3, 2, vec![]), 3).unwrap().kind, 103);
    assert_eq!(ep.read_object(&c, offer.object).unwrap(), bytes);
    let mut live_resume = offer.id().unwrap().to_vec();
    live_resume.extend_from_slice(&offer.encode().unwrap());
    assert_eq!(
        ep.handle(&c, &frame(4, 3, live_resume), 4).unwrap().kind,
        103
    );
    drop(ep);
    let mut reopened = Endpoint::open(&t.0, Limits::default(), p).unwrap();
    let mut resume = offer.id().unwrap().to_vec();
    resume.extend_from_slice(&offer.encode().unwrap());
    assert_eq!(
        reopened.handle(&c, &frame(4, 0, resume), 4).unwrap().kind,
        103
    );
}

#[test]
fn disk_quota_is_reserved_at_admission() {
    let t = Temp::new();
    let c = context();
    let limits = Limits {
        namespace_bytes: 100,
        cache_bytes: 100,
        ..Limits::default()
    };
    let mut ep = Endpoint::open(&t.0, limits, policy()).unwrap();
    let offer = Offer::cold(&[7; 32], Mode::Raw);
    assert_eq!(
        ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 0),
        Err(Error::Limit)
    );
}

#[test]
fn repair_exhaustion_is_terminal() {
    let t = Temp::new();
    let c = context();
    let g = spiralcore_transfer::dag::Graph::fold(b"abc", 100).unwrap();
    let offer = Offer {
        mode: Mode::Dag,
        len: 3,
        object: g.manifest.object,
        manifest: Some(g.manifest),
        base: None,
    };
    let mut ep = Endpoint::open(&t.0, Limits::default(), policy()).unwrap();
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 0)
        .unwrap();
    assert_eq!(ep.handle(&c, &frame(3, 1, vec![]), 1).unwrap().kind, 104);
    assert_eq!(ep.handle(&c, &frame(3, 2, vec![]), 2).unwrap().kind, 104);
    assert_eq!(ep.handle(&c, &frame(3, 3, vec![]), 3), Err(Error::Missing));
    assert_eq!(ep.handle(&c, &frame(5, 3, vec![]), 4), Err(Error::Missing));
}

#[test]
fn tiny_dag_cannot_admit_unreserved_unrelated_nodes() {
    let t = Temp::new();
    let c = context();
    let g = spiralcore_transfer::dag::Graph::fold(b"a", 100).unwrap();
    let offer = Offer {
        mode: Mode::Dag,
        len: 1,
        object: g.manifest.object,
        manifest: Some(g.manifest),
        base: None,
    };
    let mut ep = Endpoint::open(&t.0, Limits::default(), policy()).unwrap();
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 0)
        .unwrap();
    let records = (1..=3)
        .map(|n| spiralcore_transfer::dag::Node::Leaf([n; 32]).encode())
        .collect::<Vec<_>>();
    let mut body = vec![2];
    body.extend_from_slice(&(198u32).to_le_bytes());
    body.extend_from_slice(&3u16.to_le_bytes());
    for r in records {
        body.extend_from_slice(&r);
    }
    assert_eq!(ep.handle(&c, &frame(2, 1, body), 1), Err(Error::Limit));
}

#[test]
fn two_attempts_publish_one_immutable_object() {
    let t = Temp::new();
    let c = context();
    let limits = Limits {
        max_publications: 1,
        ..Limits::default()
    };
    let mut ep = Endpoint::open(&t.0, limits, policy()).unwrap();
    let offer = Offer::cold(b"abc", Mode::Raw);
    for id in [1, 2] {
        let mut f = frame(1, 0, offer.encode().unwrap());
        f.transfer = [id; 32];
        ep.handle(&c, &f, 0).unwrap();
    }
    for id in [1, 2] {
        let mut f = frame(2, 1, raw(b"abc"));
        f.transfer = [id; 32];
        ep.handle(&c, &f, 1).unwrap();
        let mut f = frame(3, 2, vec![]);
        f.transfer = [id; 32];
        assert_eq!(ep.handle(&c, &f, 2).unwrap().kind, 103);
    }
}

#[test]
fn unowned_staging_is_never_removed() {
    let t = Temp::new();
    std::fs::create_dir_all(t.0.join("unrelated")).unwrap();
    let unrelated = t.0.join("unrelated/.stage-not-ours");
    std::fs::write(&unrelated, b"preserve").unwrap();
    assert!(Endpoint::open(&t.0, Limits::default(), policy()).is_err());
    assert_eq!(std::fs::read(unrelated).unwrap(), b"preserve");
}

#[cfg(unix)]
#[test]
fn plaintext_cache_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let t = Temp::new();
    let c = context();
    let mut ep = Endpoint::open(&t.0, Limits::default(), policy()).unwrap();
    let offer = Offer::cold(b"abc", Mode::Raw);
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 0)
        .unwrap();
    ep.handle(&c, &frame(2, 1, raw(b"abc")), 1).unwrap();
    ep.handle(&c, &frame(3, 2, vec![]), 2).unwrap();
    let dir = t.0.join(spiralcore_transfer::hex(&c.namespace().unwrap()));
    let file = dir.join(format!(
        "{}.object",
        spiralcore_transfer::hex(&offer.object)
    ));
    for p in [&t.0, &dir, &file] {
        assert_eq!(
            std::fs::metadata(p).unwrap().permissions().mode() & 0o077,
            0
        );
    }
}

#[test]
fn authorization_is_rechecked_after_read() {
    use std::sync::atomic::AtomicIsize;
    struct Revoking(Arc<AtomicIsize>);
    impl Authority for Revoking {
        fn authorize(&self, _: &Context) -> Result<()> {
            if self.0.fetch_sub(1, Ordering::SeqCst) > 0 {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }
    }
    let t = Temp::new();
    let c = context();
    let remaining = Arc::new(AtomicIsize::new(100));
    let mut ep = Endpoint::open(&t.0, Limits::default(), Revoking(remaining.clone())).unwrap();
    let offer = Offer::cold(b"abc", Mode::Raw);
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 0)
        .unwrap();
    ep.handle(&c, &frame(2, 1, raw(b"abc")), 1).unwrap();
    ep.handle(&c, &frame(3, 2, vec![]), 2).unwrap();
    remaining.store(1, Ordering::SeqCst);
    assert_eq!(ep.read_object(&c, offer.object), Err(Error::Unauthorized));
}

#[test]
fn published_resume_respects_other_active_reservations() {
    let t = Temp::new();
    let c = context();
    let limits = Limits {
        max_nodes: 1,
        resident_bytes: (4 << 20) + 2512,
        ..Limits::default()
    };
    let mut ep = Endpoint::open(&t.0, limits, policy()).unwrap();
    let bytes = vec![7; 4096];
    let offer = Offer::cold(&bytes, Mode::Raw);
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 0)
        .unwrap();
    ep.handle(&c, &frame(2, 1, raw(&bytes)), 1).unwrap();
    ep.handle(&c, &frame(3, 2, vec![]), 2).unwrap();
    let second = Offer::cold(&vec![8; 1 << 20], Mode::Raw);
    let mut b = frame(1, 0, second.encode().unwrap());
    b.transfer = [8; 32];
    ep.handle(&c, &b, 3).unwrap();
    let mut resume = offer.id().unwrap().to_vec();
    resume.extend_from_slice(&offer.encode().unwrap());
    assert_eq!(ep.handle(&c, &frame(4, 3, resume), 4), Err(Error::Limit));
}

#[test]
fn object_identity_cannot_shrink_an_existing_disk_reservation() {
    let t = Temp::new();
    let c = context();
    let mut ep = Endpoint::open(&t.0, Limits::default(), policy()).unwrap();
    let mut offer = Offer::cold(&vec![7; 8192], Mode::Raw);
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 0)
        .unwrap();
    offer.len = 0;
    let mut spoof = frame(1, 0, offer.encode().unwrap());
    spoof.transfer = [8; 32];
    assert_eq!(ep.handle(&c, &spoof, 1), Err(Error::Integrity));
}

#[test]
fn terminal_reply_ledgers_cannot_outgrow_quota() {
    let t = Temp::new();
    let c = context();
    let limits = Limits {
        resident_bytes: 2_200_000,
        ..Limits::default()
    };
    let mut ep = Endpoint::open(&t.0, limits, policy()).unwrap();
    let offer = Offer::cold(b"a", Mode::Raw);
    for id in 0..3 {
        let mut f = frame(1, 0, offer.encode().unwrap());
        f.transfer = [id; 32];
        ep.handle(&c, &f, 0).unwrap();
        let mut cancel = frame(6, 1, vec![]);
        cancel.transfer = [id; 32];
        ep.handle(&c, &cancel, 1).unwrap();
    }
    let mut refused = false;
    'outer: for id in 0..3 {
        for seq in 2..4000 {
            let mut status = frame(5, seq, vec![]);
            status.transfer = [id; 32];
            match ep.handle(&c, &status, 2) {
                Ok(_) => {}
                Err(Error::Limit) => {
                    refused = true;
                    break 'outer;
                }
                Err(e) => panic!("unexpected {e:?}"),
            }
        }
    }
    assert!(
        refused,
        "terminal reply retention must consume the configured resident quota"
    );
}

#[test]
fn authorization_is_checked_at_publication() {
    use std::sync::atomic::AtomicIsize;
    struct Revoking(Arc<AtomicIsize>);
    impl Authority for Revoking {
        fn authorize(&self, _: &Context) -> Result<()> {
            if self.0.fetch_sub(1, Ordering::SeqCst) > 0 {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }
    }
    let t = Temp::new();
    let c = context();
    let remaining = Arc::new(AtomicIsize::new(100));
    let mut ep = Endpoint::open(&t.0, Limits::default(), Revoking(remaining.clone())).unwrap();
    let offer = Offer::cold(b"abc", Mode::Raw);
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 0)
        .unwrap();
    ep.handle(&c, &frame(2, 1, raw(b"abc")), 1).unwrap();
    remaining.store(2, Ordering::SeqCst);
    assert_eq!(
        ep.handle(&c, &frame(3, 2, vec![]), 2),
        Err(Error::Unauthorized)
    );
    remaining.store(100, Ordering::SeqCst);
    assert_eq!(ep.read_object(&c, offer.object), Err(Error::Unknown));
}

#[test]
fn auth_conflict_and_corruption_do_not_publish() {
    let t = Temp::new();
    let p = policy();
    let c = context();
    let offer = Offer::cold(b"abc", Mode::Raw);
    let mut ep = Endpoint::open(&t.0, Limits::default(), p.clone()).unwrap();
    let begin = frame(1, 0, offer.encode().unwrap());
    let mut wrong = c.clone();
    wrong.recipient = "other".into();
    assert_eq!(ep.handle(&wrong, &begin, 0), Err(Error::Unauthorized));
    ep.handle(&c, &begin, 0).unwrap();
    ep.handle(&c, &frame(2, 1, raw(b"bad")), 1).unwrap();
    assert_eq!(
        ep.handle(&c, &frame(3, 2, vec![]), 2),
        Err(Error::Integrity)
    );
    assert_eq!(ep.read_object(&c, offer.object), Err(Error::Unknown));
    let mut second = begin.clone();
    second.transfer = [8; 32];
    ep.handle(&c, &second, 3).unwrap();
    let mut d = frame(2, 1, raw(b"abc"));
    d.transfer = second.transfer;
    ep.handle(&c, &d, 4).unwrap();
    d.body = raw(b"def");
    assert_eq!(ep.handle(&c, &d, 5), Err(Error::Sequence));
    p.0.store(false, Ordering::SeqCst);
    let mut f = frame(3, 2, vec![]);
    f.transfer = second.transfer;
    assert_eq!(ep.handle(&c, &f, 6), Err(Error::Unauthorized));
}

#[test]
fn writer_quota_cancel_and_expiry_are_bounded() {
    let t = Temp::new();
    let c = context();
    let p = policy();
    let mut ep = Endpoint::open(&t.0, Limits::default(), p.clone()).unwrap();
    assert!(Endpoint::open(&t.0, Limits::default(), p).is_err());
    let offer = Offer::cold(b"abc", Mode::Raw);
    for n in 0..4 {
        let mut b = frame(1, 0, offer.encode().unwrap());
        b.transfer = [n; 32];
        ep.handle(&c, &b, 0).unwrap();
    }
    assert_eq!(
        ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 1),
        Err(Error::Limit)
    );
    let mut cancel = frame(6, 1, vec![]);
    cancel.transfer = [0; 32];
    assert_eq!(ep.handle(&c, &cancel, 2).unwrap().kind, 105);
    ep.handle(&c, &frame(1, 0, offer.encode().unwrap()), 3)
        .unwrap();
    assert_eq!(ep.handle(&c, &frame(5, 1, vec![]), 40), Err(Error::Expired));
}
