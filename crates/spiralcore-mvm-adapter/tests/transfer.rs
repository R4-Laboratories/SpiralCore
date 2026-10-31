#![cfg(feature = "conformance")]
mod common;

use spiralcore_mvm_adapter::conformance::Harness;
use spiralcore_transfer::{
    Error, client, object_id,
    protocol::Mode,
    schema,
    schema_selection::PreparedSelector,
    session::{RuntimeSession, WarmSession},
    shared_runtime::{OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
    wan,
};

#[test]
fn persistent_warm_mixed_objects_keep_selected_frames_and_reopen_exactly() {
    let store = common::Store::new();
    let mut harness = Harness::new(&store.0).unwrap();
    let context = harness.context().clone();
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut session = WarmSession::new(&context, &artifact).unwrap();
    let mut previous = common::random(1 << 20, 17);
    session
        .seed_base(&context, previous.clone(), [1; 32], |frames| {
            harness.exchange(frames)
        })
        .unwrap();
    let mut expected_objects = vec![previous.clone()];
    for position in 0..5 {
        let target = match position {
            1 => common::random(previous.len(), 91),
            3 => [previous.as_slice(), b"\0append\0"].concat(),
            4 => previous[..previous.len() - 17].to_vec(),
            _ => common::edited(&previous),
        };
        let transfer_id = [10 + position; 32];
        let plan = PreparedSelector::new(&artifact)
            .unwrap()
            .prepare_warm(&target, Some(&previous), false, false)
            .unwrap();
        let mut reference = Vec::new();
        assert!(matches!(
            client::transfer_warm_selection_batched(plan, transfer_id, |frames| {
                reference.extend_from_slice(frames);
                Err(Error::Storage)
            }),
            Err(Error::Storage)
        ));
        let mut actual = Vec::new();
        let mut equivalent_http_body = 0;
        let before = harness.counters();
        let sent = session
            .send(&context, target.clone(), transfer_id, false, |frames| {
                actual.extend_from_slice(frames);
                equivalent_http_body += wan::encode_batch(frames)?.len() as u64;
                let replies = harness.exchange(frames)?;
                equivalent_http_body += wan::encode_batch(&replies)?.len() as u64 + 16;
                Ok(replies)
            })
            .unwrap();
        assert_eq!(actual, reference);
        assert_eq!(sent.transfer.object, object_id(&target));
        assert_eq!(harness.read_object(object_id(&target)).unwrap(), target);
        if position == 0 || position == 2 {
            assert_eq!(sent.transfer.mode, Mode::SparseV1);
            // Historical HTTP-body accounting is preserved by identical frames.
            // Canonical MVM envelope bytes are measured separately, never called600.
            assert_eq!(equivalent_http_body, 600);
            assert!(harness.counters().envelope_bytes_sent - before.envelope_bytes_sent > 600);
        }
        previous = target.clone();
        expected_objects.push(target);
    }
    harness.reopen().unwrap();
    for expected in expected_objects {
        assert_eq!(harness.read_object(object_id(&expected)).unwrap(), expected);
    }
}

#[test]
fn lost_published_reply_does_not_promote_sender_and_recovery_reopens_committed_bytes() {
    let store = common::Store::new();
    let mut harness = Harness::new(&store.0).unwrap();
    let context = harness.context().clone();
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut session = WarmSession::new(&context, &artifact).unwrap();
    let base = common::random(128 * 1024, 31);
    session
        .seed_base(&context, base.clone(), [1; 32], |frames| {
            harness.exchange(frames)
        })
        .unwrap();
    let target = common::edited(&base);
    let result = session.send(&context, target.clone(), [2; 32], false, |frames| {
        let replies = harness.exchange(frames)?;
        if replies.iter().any(|reply| reply.kind == 103) {
            return Err(Error::Storage);
        }
        Ok(replies)
    });
    assert!(matches!(result, Err(Error::Storage)));
    assert!(!session.base_ready());
    assert_eq!(session.base_id(), Some(object_id(&base)));
    harness.reopen().unwrap();
    assert_eq!(harness.read_object(object_id(&target)).unwrap(), target);
    session
        .recover(&context, target.clone(), [3; 32], |frames| {
            harness.exchange(frames)
        })
        .unwrap();
    assert!(session.base_ready());
    assert_eq!(session.base_id(), Some(object_id(&target)));
}

#[test]
fn existing_ternary_kuramoto_ca_checkpoint_and_residual_cross_canonical_envelopes() {
    let store = common::Store::new();
    let mut harness = Harness::new(&store.0).unwrap();
    let context = harness.context().clone();
    let checkpoint = State::seeded(48).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 81,
        payload: 1.25,
        output: OutputKind::Trace,
    };
    let prediction = PreparedPrediction::new(&checkpoint, &recipe).unwrap();
    let generated = prediction.prediction().to_vec();
    let mut session = RuntimeSession::new(&context, prediction).unwrap();
    assert!(matches!(
        session.send(&context, generated.clone(), [1; 32], false, |_| {
            panic!("an unacknowledged checkpoint must not send")
        }),
        Err(Error::Missing)
    ));
    session
        .seed_checkpoint(&context, [2; 32], |frames| harness.exchange(frames))
        .unwrap();
    let edited = common::edited(&generated);
    for (index, target) in [generated.clone(), edited.clone()].into_iter().enumerate() {
        let sent = session
            .send(
                &context,
                target.clone(),
                [3 + index as u8; 32],
                false,
                |frames| harness.exchange(frames),
            )
            .unwrap();
        assert_eq!(sent.transfer.mode, Mode::RuntimeV1);
        assert_eq!(sent.transfer.object, object_id(&target));
    }
    harness.reopen().unwrap();
    for expected in [checkpoint, generated, edited] {
        assert_eq!(harness.read_object(object_id(&expected)).unwrap(), expected);
    }
}
