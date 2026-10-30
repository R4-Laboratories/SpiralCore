use spiralcore_transfer::{
    client,
    endpoint::{Authority, ColdGraphPolicy, Context, Endpoint, Limits},
    object_id,
    protocol::{Frame, Mode, Offer},
    shared_runtime::{self, OutputKind, Recipe, State},
    Error, Result,
};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-runtime-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}

fn frame(kind: u8, seq: u64, body: Vec<u8>) -> Frame {
    Frame {
        kind,
        seq,
        body,
        transfer: [9; 32],
    }
}
fn offer(checkpoint: &[u8], target: &[u8]) -> Frame {
    frame(
        1,
        0,
        Offer {
            mode: Mode::RuntimeV1,
            len: target.len() as u64,
            object: object_id(target),
            manifest: None,
            base: Some(object_id(checkpoint)),
        }
        .encode()
        .unwrap(),
    )
}
fn seeded(dir: &Temp, checkpoint: &[u8]) -> Endpoint<Allow> {
    let mut ep =
        Endpoint::open_with_policy(&dir.0, Limits::default(), Allow, ColdGraphPolicy::Disabled)
            .unwrap();
    client::transfer(checkpoint, Mode::Raw, None, [1; 32], |f| {
        ep.handle(&context(), f, 0)
    })
    .unwrap();
    ep
}

#[test]
fn checkpoint_must_be_authorized_present_and_within_replay_budget() {
    let base = State::seeded(48).unwrap().encode().unwrap();
    let target = shared_runtime::replay(&base, &recipe(), shared_runtime::MAX_OUTPUT).unwrap();
    let dir = Temp::new();
    let mut ep =
        Endpoint::open_with_policy(&dir.0, Limits::default(), Allow, ColdGraphPolicy::Disabled)
            .unwrap();
    let mut calls = 0;
    let err =
        client::transfer_runtime_batched(&base, &target, &recipe(), [2; 32], false, |frames| {
            calls += 1;
            frames.iter().map(|f| ep.handle(&context(), f, 0)).collect()
        })
        .unwrap_err();
    assert_eq!(err, Error::Missing);
    assert_eq!(calls, 1);
    let dir2 = Temp::new();
    let mut ep = seeded(&dir2, &base);
    for field in 0..4 {
        let mut other = context();
        match field {
            0 => other.policy_epoch += 1,
            1 => other.execution_epoch += 1,
            2 => other.recipient = "other".into(),
            _ => other.authorization_scope = "other".into(),
        }
        assert_eq!(
            ep.handle(&other, &offer(&base, &target), 1),
            Err(Error::Missing)
        );
    }
    let mut wrong = context();
    wrong.tenant = "wrong".into();
    assert_eq!(
        ep.handle(&wrong, &offer(&base, &target), 1),
        Err(Error::Unauthorized)
    );
    drop(ep);
    let limits = Limits {
        resident_bytes: 2 << 20,
        ..Limits::default()
    };
    let mut ep =
        Endpoint::open_with_policy(&dir2.0, limits, Allow, ColdGraphPolicy::Disabled).unwrap();
    assert_eq!(
        ep.handle(&context(), &offer(&base, &target), 1),
        Err(Error::Limit)
    );
}

#[test]
fn setup_truncations_versions_digest_and_resource_lies_are_rejected() {
    let base = State::seeded(48).unwrap().encode().unwrap();
    let target = shared_runtime::replay(&base, &recipe(), shared_runtime::MAX_OUTPUT).unwrap();
    let bodies = spiralcore_transfer::shared_transfer::prepare(&base, &target, &recipe())
        .unwrap()
        .unwrap();
    let good = &bodies[0];
    assert_eq!(
        spiralcore_transfer::shared_transfer::replay_frame(&base, good).unwrap(),
        target
    );
    let mut bad: Vec<Vec<u8>> = (0..good.len()).map(|n| good[..n].to_vec()).collect();
    for pos in [0, 1, 5, 7, good.len() - 1] {
        let mut b = good.clone();
        b[pos] ^= 0x80;
        bad.push(b);
    }
    let mut trailing = good.clone();
    trailing.push(0);
    bad.push(trailing);
    let mut claimed_length = good.clone();
    let offset = good.len() - 36;
    claimed_length[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    bad.push(claimed_length);
    for b in bad {
        assert!(spiralcore_transfer::shared_transfer::replay_frame(&base, &b).is_err());
    }
    let other = State::seeded(49).unwrap().encode().unwrap();
    assert!(spiralcore_transfer::shared_transfer::replay_frame(&other, good).is_err());
}

#[test]
fn malformed_setup_missing_setup_and_failed_final_digest_never_publish() {
    let base = State::seeded(48).unwrap().encode().unwrap();
    let target = shared_runtime::replay(&base, &recipe(), shared_runtime::MAX_OUTPUT).unwrap();
    let bodies = spiralcore_transfer::shared_transfer::prepare(&base, &target, &recipe())
        .unwrap()
        .unwrap();
    for case in 0..4 {
        let dir = Temp::new();
        let mut ep = seeded(&dir, &base);
        let c = context();
        let mut offered = target.clone();
        if case == 3 {
            offered[0] ^= 1;
        }
        ep.handle(&c, &offer(&base, &offered), 1).unwrap();
        if case == 0 {
            assert_eq!(
                ep.handle(&c, &frame(3, 1, vec![]), 2),
                Err(Error::Malformed)
            );
        } else if case == 1 {
            assert!(ep.handle(&c, &frame(2, 1, bodies[1].clone()), 2).is_err());
        } else if case == 2 {
            let mut bad = bodies[0].clone();
            *bad.last_mut().unwrap() ^= 1;
            assert_eq!(ep.handle(&c, &frame(2, 1, bad), 2), Err(Error::Integrity));
        } else {
            for (i, body) in bodies.iter().enumerate() {
                ep.handle(&c, &frame(2, i as u64 + 1, body.clone()), 2)
                    .unwrap();
            }
            assert_eq!(
                ep.handle(&c, &frame(3, bodies.len() as u64 + 1, vec![]), 3),
                Err(Error::Integrity)
            );
        }
        assert_eq!(ep.read_object(&c, object_id(&offered)), Err(Error::Unknown));
        drop(ep);
        let mut ep =
            Endpoint::open_with_policy(&dir.0, Limits::default(), Allow, ColdGraphPolicy::Disabled)
                .unwrap();
        assert_eq!(ep.read_object(&c, object_id(&offered)), Err(Error::Unknown));
    }
}

#[test]
fn replay_is_idempotent_and_cancel_discards_output() {
    let base = State::seeded(48).unwrap().encode().unwrap();
    let target = shared_runtime::replay(&base, &recipe(), shared_runtime::MAX_OUTPUT).unwrap();
    let bodies = spiralcore_transfer::shared_transfer::prepare(&base, &target, &recipe())
        .unwrap()
        .unwrap();
    let dir = Temp::new();
    let mut ep = seeded(&dir, &base);
    let c = context();
    let begin = offer(&base, &target);
    let ready = ep.handle(&c, &begin, 1).unwrap();
    assert_eq!(ep.handle(&c, &begin, 1).unwrap(), ready);
    let setup = frame(2, 1, bodies[0].clone());
    let ack = ep.handle(&c, &setup, 2).unwrap();
    assert_eq!(ack.body, 0u64.to_le_bytes());
    assert_eq!(ep.handle(&c, &setup, 2).unwrap(), ack);
    ep.handle(&c, &frame(6, 2, vec![]), 3).unwrap();
    assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
}

#[test]
fn different_lengths_and_final_checkpoint_use_exact_residuals() {
    let base = State::seeded(48).unwrap().encode().unwrap();
    for output in [OutputKind::Trace, OutputKind::FinalCheckpoint] {
        let recipe = Recipe { output, ..recipe() };
        let target = shared_runtime::replay(&base, &recipe, shared_runtime::MAX_OUTPUT).unwrap();
        for bytes in [
            &target[1..],
            target.as_slice(),
            &[target.as_slice(), &[0, 0, 7, 0]].concat(),
        ] {
            let frames = spiralcore_transfer::shared_transfer::prepare(&base, bytes, &recipe)
                .unwrap()
                .expect("these known replay cases must produce an exact residual");
            let prediction =
                spiralcore_transfer::shared_transfer::replay_frame(&base, &frames[0]).unwrap();
            let mut decoded = Vec::new();
            let mut ops = 0;
            for body in &frames[1..] {
                spiralcore_transfer::delta::apply(
                    &prediction,
                    &mut decoded,
                    bytes.len(),
                    &mut ops,
                    body,
                )
                .unwrap();
            }
            assert_eq!(decoded, bytes);
        }
    }
}

#[test]
fn revoked_authority_and_changed_epoch_cannot_finish_an_admitted_replay() {
    use std::sync::{atomic::AtomicBool, Arc};
    struct Gate(Arc<AtomicBool>);
    impl Authority for Gate {
        fn authorize(&self, _: &Context) -> Result<()> {
            if self.0.load(Ordering::SeqCst) {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }
    }
    let allowed = Arc::new(AtomicBool::new(true));
    let dir = Temp::new();
    let mut ep = Endpoint::open_with_policy(
        &dir.0,
        Limits::default(),
        Gate(allowed.clone()),
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let c = context();
    let base = State::seeded(48).unwrap().encode().unwrap();
    let target = shared_runtime::replay(&base, &recipe(), shared_runtime::MAX_OUTPUT).unwrap();
    client::transfer(&base, Mode::Raw, None, [1; 32], |f| ep.handle(&c, f, 0)).unwrap();
    let bodies = spiralcore_transfer::shared_transfer::prepare(&base, &target, &recipe())
        .unwrap()
        .unwrap();
    ep.handle(&c, &offer(&base, &target), 1).unwrap();
    ep.handle(&c, &frame(2, 1, bodies[0].clone()), 2).unwrap();
    let mut newer = c.clone();
    newer.execution_epoch += 1;
    assert_eq!(
        ep.handle(&newer, &frame(2, 2, bodies[1].clone()), 3),
        Err(Error::Unknown)
    );
    allowed.store(false, Ordering::SeqCst);
    assert_eq!(
        ep.handle(&c, &frame(2, 2, bodies[1].clone()), 3),
        Err(Error::Unauthorized)
    );
    allowed.store(true, Ordering::SeqCst);
    assert_eq!(ep.read_object(&c, object_id(&target)), Err(Error::Unknown));
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Allow;
impl Authority for Allow {
    fn authorize(&self, c: &Context) -> Result<()> {
        if c.tenant == "runtime-test" {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
fn context() -> Context {
    Context {
        tenant: "runtime-test".into(),
        domain: "public".into(),
        sender: "s".into(),
        recipient: "r".into(),
        mailbox: "m".into(),
        capability: "c".into(),
        authorization_scope: "runtime".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}
fn recipe() -> Recipe {
    Recipe {
        steps: 81,
        payload: 1.25,
        output: OutputKind::Trace,
    }
}

#[test]
fn runtime_recipe_crosses_transfer_boundary_and_reopens_exact_bytes() {
    let dir = Temp::new();
    let c = context();
    let checkpoint = State::seeded(48).unwrap().encode().unwrap();
    let mut target =
        shared_runtime::replay(&checkpoint, &recipe(), shared_runtime::MAX_OUTPUT).unwrap();
    target[101] ^= 1;
    target.extend_from_slice(&[0, 0, 7, 0]);
    let mut receiver =
        Endpoint::open_with_policy(&dir.0, Limits::default(), Allow, ColdGraphPolicy::Disabled)
            .unwrap();
    client::transfer_batched(&checkpoint, Mode::Raw, None, [1; 32], false, |frames| {
        frames.iter().map(|f| receiver.handle(&c, f, 1)).collect()
    })
    .unwrap();
    let result = client::transfer_runtime_batched(
        &checkpoint,
        &target,
        &recipe(),
        [2; 32],
        false,
        |frames| frames.iter().map(|f| receiver.handle(&c, f, 2)).collect(),
    )
    .unwrap();
    assert_eq!(result.transfer.mode, Mode::RuntimeV1);
    assert!(result.transfer.request_bytes + result.transfer.response_bytes < 1024);
    assert_eq!(
        receiver.read_object(&c, object_id(&target)).unwrap(),
        target
    );
    drop(receiver);
    let mut reopened =
        Endpoint::open_with_policy(&dir.0, Limits::default(), Allow, ColdGraphPolicy::Disabled)
            .unwrap();
    assert_eq!(
        reopened.read_object(&c, object_id(&target)).unwrap(),
        target
    );
}
