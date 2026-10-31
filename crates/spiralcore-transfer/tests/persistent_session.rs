use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Context, Endpoint},
    hex, object_id,
    protocol::{Frame, Mode},
    schema,
    schema_selection::PreparedSelector,
    session::{RuntimeSession, WarmSession},
    shared_runtime::{OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
    wan::{self, BenchAuthority},
    Error, Id, Result,
};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Store(std::path::PathBuf);
impl Store {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-session-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
    fn open(&self) -> Endpoint<BenchAuthority> {
        Endpoint::open_with_policy(
            &self.0,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap()
    }
    fn publication(&self, context: &Context, object: Id) -> std::path::PathBuf {
        self.0
            .join(hex(&context.namespace().unwrap()))
            .join(format!("{}.object", hex(&object)))
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn random(len: usize, mut state: u64) -> Vec<u8> {
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}
fn edit(base: &[u8]) -> Vec<u8> {
    let mut target = base.to_vec();
    let middle = target.len() / 2;
    for byte in &mut target[middle..middle + 32] {
        *byte ^= 0xa5;
    }
    target
}
fn exchange(
    endpoint: &mut Endpoint<BenchAuthority>,
    context: &Context,
    frames: &[Frame],
    body_bytes: &mut u64,
) -> Result<Vec<Frame>> {
    *body_bytes += wan::encode_batch(frames)?.len() as u64;
    let mut replies = Vec::new();
    for frame in frames {
        let reply = endpoint
            .handle(context, frame, 0)
            .unwrap_or_else(|e| frame.reply(127, (e as u16).to_le_bytes().to_vec()));
        let terminal = matches!(reply.kind, 103 | 127);
        replies.push(reply);
        if terminal {
            break;
        }
    }
    *body_bytes += wan::encode_batch(&replies)?.len() as u64 + 16;
    Ok(replies)
}
fn artifact() -> Vec<u8> {
    schema::fit(&[b"{\"value\":0}\n"]).unwrap()
}

#[test]
fn rolling_owned_base_advances_only_on_ack_and_matches_existing_sender_frames() {
    let store = Store::new();
    let mut endpoint = store.open();
    let context = wan::context([1; 32]);
    let artifact = artifact();
    let mut session = WarmSession::new(&context, &artifact).unwrap();
    let base = random(1 << 20, 17);
    let pointer = base.as_ptr();
    let mut previous = base.clone();
    let mut bytes = 0;
    session
        .seed_base(&context, base, [1; 32], |frames| {
            exchange(&mut endpoint, &context, frames, &mut bytes)
        })
        .unwrap();
    assert_eq!(session.base_id(), Some(object_id(&previous)));
    assert_eq!(session.base_bytes().unwrap().as_ptr(), pointer);
    assert!(session.base_ready());
    let mut expected_objects = vec![previous.clone()];
    for index in 0..5 {
        let target = match index {
            1 => random(previous.len(), 91),
            3 => [previous.as_slice(), b"\0append\0"].concat(),
            4 => previous[..previous.len() - 17].to_vec(),
            _ => edit(&previous),
        };
        let expected = target.clone();
        let pointer = target.as_ptr();
        let transfer_id = [10 + index; 32];
        let plan = PreparedSelector::new(&artifact)
            .unwrap()
            .prepare_warm(&target, Some(&previous), false, false)
            .unwrap();
        let choice = format!("{:?}", plan.selection());
        let mut reference = Vec::new();
        assert!(matches!(
            client::transfer_warm_selection_batched(plan, transfer_id, |frames| {
                reference.extend_from_slice(frames);
                Err(Error::Storage)
            }),
            Err(Error::Storage)
        ));
        let mut actual = Vec::new();
        let mut bytes = 0;
        let sent = session
            .send(&context, target, transfer_id, false, |frames| {
                actual.extend_from_slice(frames);
                exchange(&mut endpoint, &context, frames, &mut bytes)
            })
            .unwrap();
        assert_eq!(format!("{:?}", sent.selection), choice);
        assert_eq!(actual, reference);
        assert_eq!(sent.transfer.object, object_id(&expected));
        if index == 0 || index == 2 {
            assert_eq!(sent.transfer.mode, Mode::SparseV1);
            assert_eq!(bytes, 600);
        }
        assert_eq!(session.base_bytes().unwrap().as_ptr(), pointer);
        assert_eq!(session.base_id(), Some(object_id(&expected)));
        assert!(session.base_ready());
        assert!(session.retained_charge() <= 512 << 20);
        assert!(session.retained_bytes() >= expected.len());
        previous = expected.clone();
        expected_objects.push(expected);
    }
    drop(endpoint);
    let mut reopened = store.open();
    for expected in expected_objects {
        assert_eq!(
            reopened
                .read_object(&context, object_id(&expected))
                .unwrap(),
            expected
        );
    }
}

#[test]
fn lost_ack_never_promotes_and_explicit_recovery_confirms_committed_target() {
    let store = Store::new();
    let mut endpoint = store.open();
    let context = wan::context([2; 32]);
    let mut session = WarmSession::new(&context, &artifact()).unwrap();
    let base = random(65537, 27);
    let target = edit(&base);
    let old_id = object_id(&base);
    session
        .seed_base(&context, base, [1; 32], |frames| {
            exchange(&mut endpoint, &context, frames, &mut 0)
        })
        .unwrap();
    let mut calls = 0;
    assert!(matches!(
        session.send(&context, target.clone(), [2; 32], false, |frames| {
            calls += 1;
            let replies = exchange(&mut endpoint, &context, frames, &mut 0)?;
            assert_eq!(replies.last().unwrap().kind, 103);
            Err(Error::Storage)
        }),
        Err(Error::Storage)
    ));
    assert_eq!(calls, 1);
    assert_eq!(session.base_id(), Some(old_id));
    assert!(!session.base_ready());
    assert!(!session.schema_ready());
    assert_eq!(
        endpoint.read_object(&context, object_id(&target)).unwrap(),
        target
    );
    drop(endpoint);
    let mut endpoint = store.open();
    let pointer = target.as_ptr();
    let id = object_id(&target);
    let mut requests = Vec::new();
    session
        .recover(&context, target, [3; 32], |frames| {
            requests.extend_from_slice(frames);
            exchange(&mut endpoint, &context, frames, &mut 0)
        })
        .unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].kind, 1);
    assert_eq!(session.base_id(), Some(id));
    assert_eq!(session.base_bytes().unwrap().as_ptr(), pointer);
    assert!(session.base_ready());
}

#[test]
fn missing_or_corrupt_base_does_not_retry_and_recovery_cold_sends_explicitly() {
    for corrupt in [false, true] {
        let store = Store::new();
        let mut endpoint = store.open();
        let context = wan::context([3; 32]);
        let mut session = WarmSession::new(&context, &artifact()).unwrap();
        let base = random(65537, 37);
        let target = edit(&base);
        let id = object_id(&base);
        session
            .seed_base(&context, base, [1; 32], |frames| {
                exchange(&mut endpoint, &context, frames, &mut 0)
            })
            .unwrap();
        let path = store.publication(&context, id);
        if corrupt {
            let mut bytes = std::fs::read(&path).unwrap();
            *bytes.last_mut().unwrap() ^= 1;
            std::fs::write(path, bytes).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
        let mut calls = 0;
        let result = session.send(&context, target.clone(), [2; 32], false, |frames| {
            calls += 1;
            exchange(&mut endpoint, &context, frames, &mut 0)
        });
        assert!(matches!(result, Err(Error::Missing | Error::Integrity)));
        assert_eq!(calls, 1);
        assert_eq!(session.base_id(), Some(id));
        assert!(!session.base_ready());
        assert!(endpoint.read_object(&context, object_id(&target)).is_err());
        let mut batches = 0;
        let stats = session
            .recover(&context, target.clone(), [3; 32], |frames| {
                batches += 1;
                exchange(&mut endpoint, &context, frames, &mut 0)
            })
            .unwrap();
        assert_eq!(batches, 2);
        assert_eq!(stats.mode, Mode::Raw);
        assert_eq!(session.base_id(), Some(object_id(&target)));
        assert!(session.base_ready());
    }
}

#[test]
fn context_changes_and_invalid_ack_invalidate_hints_without_promotion() {
    let context = wan::context([4; 32]);
    for field in 0..9 {
        let mut changed = context.clone();
        match field {
            0 => changed.tenant.push('x'),
            1 => changed.domain.push('x'),
            2 => changed.sender.push('x'),
            3 => changed.recipient.push('x'),
            4 => changed.mailbox = hex(&[5; 32]),
            5 => changed.capability.push('x'),
            6 => changed.authorization_scope.push('x'),
            7 => changed.policy_epoch += 1,
            _ => changed.execution_epoch += 1,
        }
        let mut session = WarmSession::new(&context, &artifact()).unwrap();
        let mut calls = 0;
        assert!(matches!(
            session.send(&changed, b"new".to_vec(), [1; 32], false, |_| {
                calls += 1;
                Err(Error::Storage)
            }),
            Err(Error::Unauthorized)
        ));
        assert_eq!(calls, 0);
        assert!(!session.base_ready());
    }
    for wrong_sequence in [false, true] {
        let store = Store::new();
        let mut endpoint = store.open();
        let mut session = WarmSession::new(&context, &artifact()).unwrap();
        let base = random(65537, 49);
        let target = edit(&base);
        let old = object_id(&base);
        session
            .seed_base(&context, base, [1; 32], |frames| {
                exchange(&mut endpoint, &context, frames, &mut 0)
            })
            .unwrap();
        assert!(matches!(
            session.send(&context, target, [2; 32], false, |frames| {
                let mut replies = exchange(&mut endpoint, &context, frames, &mut 0)?;
                let ack = replies.last_mut().unwrap();
                if wrong_sequence {
                    ack.seq += 1;
                } else {
                    ack.body[0] ^= 1;
                }
                Ok(replies)
            }),
            Err(Error::Integrity)
        ));
        assert_eq!(session.base_id(), Some(old));
        assert!(!session.base_ready());
    }
}

#[test]
fn input_capacity_limits_precede_network_and_do_not_destroy_known_base() {
    let store = Store::new();
    let mut endpoint = store.open();
    let context = wan::context([6; 32]);
    let mut session = WarmSession::new(&context, &artifact()).unwrap();
    let base = b"valid base".to_vec();
    let old = object_id(&base);
    session
        .seed_base(&context, base, [1; 32], |frames| {
            exchange(&mut endpoint, &context, frames, &mut 0)
        })
        .unwrap();
    let mut excess_capacity = Vec::with_capacity(client::MAX_BATCHED_OBJECT + 1);
    excess_capacity.push(1);
    for target in [excess_capacity, vec![0; client::MAX_BATCHED_OBJECT + 1]] {
        let mut calls = 0;
        assert!(matches!(
            session.send(&context, target, [2; 32], false, |_| {
                calls += 1;
                Err(Error::Storage)
            }),
            Err(Error::Limit)
        ));
        assert_eq!(calls, 0);
        assert_eq!(session.base_id(), Some(old));
        assert!(session.base_ready());
    }
    session.invalidate_receiver_state();
    assert_eq!(session.base_id(), Some(old));
    assert!(!session.base_ready());
    assert!(WarmSession::new(&context, b"invalid artifact").is_err());
}

#[test]
fn schema_seed_is_explicit_auxiliary_and_context_change_discards_scoped_state() {
    let keys: Vec<_> = (0..8)
        .map(|i| format!("stable_schema_measurement_identifier_{i}"))
        .collect();
    let record = |values: &[u64]| {
        format!(
            "{{{}}}\n",
            keys.iter()
                .zip(values)
                .map(|(key, value)| format!("\"{key}\":{value}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let training = record(&[0; 8]);
    let artifact = schema::fit(&[training.as_bytes()]).unwrap();
    let mut state = 0x8434_93ae_b2af_1d57u64;
    let mut target = Vec::new();
    for _ in 0..4096 {
        let mut values = [0; 8];
        for value in &mut values {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *value = state;
        }
        target.extend_from_slice(record(&values).as_bytes());
    }
    let context = wan::context([8; 32]);
    let store = Store::new();
    let mut endpoint = store.open();
    let mut session = WarmSession::new(&context, &artifact).unwrap();
    let mut calls = 0;
    assert!(matches!(
        session.send(&context, target.clone(), [1; 32], false, |_| {
            calls += 1;
            Err(Error::Storage)
        }),
        Err(Error::Missing)
    ));
    assert_eq!(calls, 0);
    session
        .seed_schema(&context, [2; 32], |frames| {
            exchange(&mut endpoint, &context, frames, &mut 0)
        })
        .unwrap();
    assert!(session.schema_ready());
    assert_eq!(session.base_id(), None);
    let expected = target.clone();
    let sent = session
        .send(&context, target, [3; 32], false, |frames| {
            exchange(&mut endpoint, &context, frames, &mut 0)
        })
        .unwrap();
    assert_eq!(sent.transfer.mode, Mode::SchemaZstdV1);
    assert_eq!(
        endpoint
            .read_object(&context, object_id(&expected))
            .unwrap(),
        expected
    );
    let mut changed = context.clone();
    changed.policy_epoch += 1;
    assert!(matches!(
        session.send(&changed, Vec::new(), [4; 32], false, |_| {
            panic!("changed context must fail before network")
        }),
        Err(Error::Unauthorized)
    ));
    assert_eq!(session.base_id(), None);
    assert!(!session.schema_ready());
    assert!(!session.base_ready());
}

#[test]
fn runtime_session_reuses_fixed_prediction_and_requires_checkpoint_ack() {
    let context = wan::context([7; 32]);
    let store = Store::new();
    let mut endpoint = store.open();
    let checkpoint = State::seeded(48).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 81,
        payload: 1.25,
        output: OutputKind::Trace,
    };
    let prediction = PreparedPrediction::new(&checkpoint, &recipe).unwrap();
    let target = prediction.prediction().to_vec();
    let mut session = RuntimeSession::new(&context, prediction).unwrap();
    let mut calls = 0;
    assert!(matches!(
        session.send(&context, target.clone(), [1; 32], false, |_| {
            calls += 1;
            Err(Error::Storage)
        }),
        Err(Error::Missing)
    ));
    assert_eq!(calls, 0);
    session
        .seed_checkpoint(&context, [2; 32], |frames| {
            exchange(&mut endpoint, &context, frames, &mut 0)
        })
        .unwrap();
    assert!(session.checkpoint_ready());
    assert_eq!(session.base_id(), None);
    for (index, bytes) in [target.clone(), edit(&target)].into_iter().enumerate() {
        let expected = bytes.clone();
        let pointer = bytes.as_ptr();
        let sent = session
            .send(&context, bytes, [3 + index as u8; 32], false, |frames| {
                exchange(&mut endpoint, &context, frames, &mut 0)
            })
            .unwrap();
        assert_eq!(sent.transfer.mode, Mode::RuntimeV1);
        assert_eq!(sent.transfer.object, object_id(&expected));
        assert_eq!(session.base_bytes().unwrap().as_ptr(), pointer);
        assert!(session.retained_charge() <= 512 << 20);
        assert_eq!(
            endpoint
                .read_object(&context, object_id(&expected))
                .unwrap(),
            expected
        );
    }
    let old = session.base_id();
    std::fs::remove_file(store.publication(&context, object_id(&checkpoint))).unwrap();
    let mut uncommitted = target.clone();
    uncommitted[11] ^= 0x5a;
    let mut calls = 0;
    assert!(matches!(
        session.send(&context, uncommitted.clone(), [6; 32], false, |frames| {
            calls += 1;
            exchange(&mut endpoint, &context, frames, &mut 0)
        }),
        Err(Error::Missing)
    ));
    assert_eq!(calls, 1);
    assert_eq!(session.base_id(), old);
    assert!(!session.checkpoint_ready());
    assert!(endpoint
        .read_object(&context, object_id(&uncommitted))
        .is_err());
    session
        .seed_checkpoint(&context, [7; 32], |frames| {
            exchange(&mut endpoint, &context, frames, &mut 0)
        })
        .unwrap();
    let sent = session
        .send(&context, uncommitted.clone(), [8; 32], false, |frames| {
            exchange(&mut endpoint, &context, frames, &mut 0)
        })
        .unwrap();
    assert_eq!(sent.transfer.mode, Mode::RuntimeV1);
    assert_eq!(
        endpoint
            .read_object(&context, object_id(&uncommitted))
            .unwrap(),
        uncommitted
    );
    let last = session.base_id();
    session.invalidate_receiver_state();
    assert!(!session.checkpoint_ready());
    assert_eq!(session.base_id(), last);
    session
        .recover(&context, target.clone(), [9; 32], |frames| {
            exchange(&mut endpoint, &context, frames, &mut 0)
        })
        .unwrap();
    assert!(!session.checkpoint_ready());
    assert_eq!(session.base_id(), Some(object_id(&target)));
}
