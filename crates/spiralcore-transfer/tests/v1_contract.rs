use spiralcore_transfer::{
    endpoint::{Authority, Context, Limits},
    object_id,
    protocol::{Frame, Mode, Offer},
    v1::{Capabilities, FrameEndpoint, ObjectEndpoint, ObjectMode, ObjectSender},
    Error, Result,
};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
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
struct Policy(bool);
impl Authority for Policy {
    fn authorize(&self, _: &Context) -> Result<()> {
        if self.0 {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-v1-{}-{}",
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
fn profile(mask: u16, cap: u64) -> spiralcore_transfer::v1::NegotiatedProfile {
    let mut c = Capabilities::full();
    c.codec_mask = mask;
    c.max_object_bytes = cap;
    c.negotiate(&Capabilities::full()).unwrap()
}
#[test]
fn capabilities_validate_before_intersection() {
    let full = Capabilities::full();
    assert_eq!(full.negotiate(&full).unwrap().capabilities(), full);
    for bad in [
        Capabilities { major: 2, ..full },
        Capabilities { minor: 1, ..full },
        Capabilities { profile: 2, ..full },
        Capabilities {
            codec_mask: 1025,
            ..full
        },
        Capabilities {
            codec_mask: 2,
            ..full
        },
    ] {
        assert_eq!(bad.negotiate(&full), Err(Error::Unsupported));
    }
    for bad in [
        Capabilities {
            max_frame_bytes: 65535,
            ..full
        },
        Capabilities {
            max_object_bytes: 0,
            ..full
        },
        Capabilities {
            max_object_bytes: (64 << 20) + 1,
            ..full
        },
    ] {
        assert_eq!(bad.negotiate(&full), Err(Error::Limit));
    }
    assert_eq!(profile(3, 10).capabilities().codec_mask, 3);
    assert_eq!(profile(3, 10).capabilities().max_object_bytes, 10);
}
#[test]
fn endpoint_checks_authority_then_begin_and_resume_caps() {
    let t = Temp::new();
    let c = context();
    let mut ep =
        ObjectEndpoint::open(&t.0, Limits::default(), Policy(true), profile(1, 4)).unwrap();
    for kind in [1, 4] {
        let offer = Offer::cold(b"hi", Mode::Zstd);
        let mut body = if kind == 4 { vec![0; 32] } else { vec![] };
        body.extend(offer.encode().unwrap());
        assert_eq!(
            ep.handle(
                &c,
                &Frame {
                    kind,
                    transfer: [1; 32],
                    seq: 0,
                    body
                },
                0
            ),
            Err(Error::Unsupported)
        );
    }
    let f = Frame {
        kind: 1,
        transfer: [1; 32],
        seq: 0,
        body: Offer::cold(b"12345", Mode::Raw).encode().unwrap(),
    };
    assert_eq!(ep.handle(&c, &f, 0), Err(Error::Limit));
    let u = Temp::new();
    let mut denied =
        ObjectEndpoint::open(&u.0, Limits::default(), Policy(false), profile(1, 4)).unwrap();
    let oversized = Frame {
        body: vec![0; 65536],
        ..f
    };
    assert_eq!(denied.handle(&c, &oversized, 0), Err(Error::Unauthorized));
}
#[test]
fn exact_durable_reopen_and_interrupted_not_published() {
    let t = Temp::new();
    let c = context();
    let p = profile(1023, 64 << 20);
    let bytes = [vec![42; 65537], vec![0, 0, 0]].concat();
    let id = object_id(&bytes);
    let mut ep = ObjectEndpoint::open(&t.0, Limits::default(), Policy(true), p).unwrap();
    let mut sender = ObjectSender::new_with_profile(ObjectMode::Raw, &c, None, p).unwrap();
    sender
        .send(&c, bytes.clone(), [3; 32], false, |fs| {
            fs.iter()
                .map(|f| FrameEndpoint::handle(&mut ep, &c, f, 0))
                .collect()
        })
        .unwrap();
    assert_eq!(ep.read_object(&c, id).unwrap(), bytes);
    drop(ep);
    let mut ep = ObjectEndpoint::open(&t.0, Limits::default(), Policy(true), p).unwrap();
    assert_eq!(ep.read_object(&c, id).unwrap(), bytes);
    let pending = b"never committed";
    ep.handle(
        &c,
        &Frame {
            kind: 1,
            transfer: [4; 32],
            seq: 0,
            body: Offer::cold(pending, Mode::Raw).encode().unwrap(),
        },
        1,
    )
    .unwrap();
    assert_eq!(ep.read_object(&c, object_id(pending)), Err(Error::Unknown));
}
#[test]
fn sender_rejects_ineligible_before_callback_and_limits_replies() {
    let c = context();
    let mut calls = 0;
    let mut sender =
        ObjectSender::new_with_profile(ObjectMode::Whole, &c, None, profile(1, 1024)).unwrap();
    assert_eq!(
        sender
            .send(&c, vec![42; 100], [5; 32], false, |_| {
                calls += 1;
                Ok(vec![])
            })
            .unwrap_err(),
        Error::Unsupported
    );
    assert_eq!(calls, 0);
    let mut sender =
        ObjectSender::new_with_profile(ObjectMode::Raw, &c, None, profile(1, 4)).unwrap();
    assert_eq!(
        sender
            .seed(&c, vec![0; 5], [6; 32], |_| {
                calls += 1;
                Ok(vec![])
            })
            .unwrap_err(),
        Error::Limit
    );
    assert_eq!(calls, 0);
    assert_eq!(
        sender
            .send(&c, vec![0; 1], [7; 32], false, |fs| Ok(vec![
                fs[0].reply(101, vec![0; 65536])
            ]))
            .unwrap_err(),
        Error::Limit
    );
}
#[test]
fn legacy_wire_golden_is_unchanged() {
    let f = Frame {
        kind: 6,
        transfer: [0; 32],
        seq: 1,
        body: vec![0xaa],
    };
    let encoded = f.encode().unwrap();
    let mut golden = b"SCMOBJ01\x06\0\0\0".to_vec();
    golden.extend([0; 32]);
    golden.extend([1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0xaa]);
    assert_eq!(encoded, golden);
    assert_eq!(
        object_id(b""),
        [
            0x02, 0x9e, 0xca, 0x22, 0xf5, 0xc8, 0xac, 0x72, 0xd8, 0x77, 0xf0, 0xf5, 0x98, 0x11,
            0xba, 0x12, 0x4a, 0x2f, 0xce, 0x3d, 0xcd, 0x65, 0x8d, 0xb5, 0x54, 0x8c, 0xbc, 0x5b,
            0x91, 0x0c, 0x4b, 0x46
        ]
    );
}
fn prediction() -> spiralcore_transfer::shared_transfer::PreparedPrediction {
    use spiralcore_transfer::{
        shared_runtime::{OutputKind, Recipe, State},
        shared_transfer::PreparedPrediction,
    };
    PreparedPrediction::new(
        &State::seeded(48).unwrap().encode().unwrap(),
        &Recipe {
            steps: 81,
            payload: 1.25,
            output: OutputKind::Trace,
        },
    )
    .unwrap()
}
#[test]
fn every_seed_constructor_enforces_the_same_object_cap_without_carrier() {
    use spiralcore_transfer::{schema, session::SelectionPolicy};
    let c = context();
    let schema = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut warm =
        ObjectSender::new_with_profile(ObjectMode::Warm, &c, Some(&schema), profile(1, 1)).unwrap();
    let mut runtime = ObjectSender::with_runtime_profile(&c, prediction(), profile(1, 1)).unwrap();
    let mut combined = ObjectSender::with_combined_policy_and_profile(
        &c,
        prediction(),
        &schema,
        SelectionPolicy::Exhaustive,
        profile(1, 1),
    )
    .unwrap();
    let mut calls = 0;
    for sender in [&mut warm, &mut combined] {
        assert_eq!(
            sender
                .seed_schema(&c, [8; 32], |_| {
                    calls += 1;
                    Ok(vec![])
                })
                .unwrap_err(),
            Error::Limit
        );
    }
    for sender in [&mut runtime, &mut combined] {
        assert_eq!(
            sender
                .seed_checkpoint(&c, [9; 32], |_| {
                    calls += 1;
                    Ok(vec![])
                })
                .unwrap_err(),
            Error::Limit
        );
    }
    for sender in [&mut warm, &mut runtime, &mut combined] {
        assert_eq!(
            sender
                .send(&c, vec![0; 2], [10; 32], false, |_| {
                    calls += 1;
                    Ok(vec![])
                })
                .unwrap_err(),
            Error::Limit
        );
    }
    assert_eq!(calls, 0);
}
#[test]
fn narrowed_runtime_and_combined_selected_offer_never_reaches_carrier() {
    use spiralcore_transfer::{schema, session::SelectionPolicy};
    let t = Temp::new();
    let c = context();
    let schema = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut ep =
        ObjectEndpoint::open(&t.0, Limits::default(), Policy(true), profile(1, 64 << 20)).unwrap();
    for combined in [false, true] {
        let p = prediction();
        let target = p.prediction().to_vec();
        let mut sender = if combined {
            ObjectSender::with_combined_policy_and_profile(
                &c,
                p,
                &schema,
                SelectionPolicy::Exhaustive,
                profile(1, 64 << 20),
            )
            .unwrap()
        } else {
            ObjectSender::with_runtime_profile(&c, p, profile(1, 64 << 20)).unwrap()
        };
        sender
            .seed_checkpoint(&c, [11 + combined as u8; 32], |fs| {
                let mut replies = Vec::new();
                for f in fs {
                    let reply = ep.handle(&c, f, 0)?;
                    let terminal = matches!(reply.kind, 103 | 127);
                    replies.push(reply);
                    if terminal {
                        break;
                    }
                }
                Ok(replies)
            })
            .unwrap();
        let mut calls = 0;
        assert_eq!(
            sender
                .send(&c, target, [14 + combined as u8; 32], false, |_| {
                    calls += 1;
                    Ok(vec![])
                })
                .unwrap_err(),
            Error::Unsupported
        );
        assert_eq!(calls, 0);
    }
}
#[test]
fn narrowed_warm_selected_offer_never_reaches_carrier() {
    use spiralcore_transfer::schema;
    let t = Temp::new();
    let c = context();
    let schema = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut ep =
        ObjectEndpoint::open(&t.0, Limits::default(), Policy(true), profile(1, 64 << 20)).unwrap();
    let mut sender =
        ObjectSender::new_with_profile(ObjectMode::Warm, &c, Some(&schema), profile(1, 64 << 20))
            .unwrap();
    sender
        .seed_schema(&c, [17; 32], |fs| {
            fs.iter().map(|f| ep.handle(&c, f, 0)).collect()
        })
        .unwrap();
    let target = b"{\"value\":123}\n".repeat(1000);
    let mut calls = 0;
    assert_eq!(
        sender
            .send(&c, target, [18; 32], false, |_| {
                calls += 1;
                Ok(vec![])
            })
            .unwrap_err(),
        Error::Unsupported
    );
    assert_eq!(calls, 0);
}
