use spiralcore_gateway_core::object::{ObjectMode, ObjectSelection, ObjectSender};
use spiralcore_transfer::{
    endpoint::{ColdGraphPolicy, Context, Endpoint},
    hex, object_id,
    protocol::{Frame, Mode},
    schema,
    shared_runtime::{OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
    wan::{self, BenchAuthority},
    Error, Result,
};
use std::path::Path;

fn open(path: &Path) -> Endpoint<BenchAuthority> {
    Endpoint::open_with_policy(
        path,
        wan::limits(),
        BenchAuthority,
        ColdGraphPolicy::Disabled,
    )
    .unwrap()
}
fn exchange(
    endpoint: &mut Endpoint<BenchAuthority>,
    context: &Context,
    frames: &[Frame],
) -> Result<Vec<Frame>> {
    let mut replies = Vec::new();
    for frame in frames {
        let reply = endpoint
            .handle(context, frame, 0)
            .unwrap_or_else(|error| frame.reply(127, (error as u16).to_le_bytes().to_vec()));
        let terminal = matches!(reply.kind, 103 | 127);
        replies.push(reply);
        if terminal {
            break;
        }
    }
    Ok(replies)
}
fn prediction() -> PreparedPrediction {
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
fn runtime_profile_requires_checkpoint_and_returns_real_selection_with_exact_reopen() {
    let context = wan::context([71; 32]);
    let directory = tempfile::tempdir().unwrap();
    let cache = directory.path().join("cache");
    let mut endpoint = open(&cache);
    let prediction = prediction();
    let checkpoint = prediction.checkpoint().to_vec();
    let generated = prediction.prediction().to_vec();
    assert!(matches!(
        ObjectSender::new(ObjectMode::Runtime, &context, None),
        Err(Error::Malformed)
    ));
    let mut sender = ObjectSender::with_runtime(&context, prediction).unwrap();
    assert!(sender.retained_charge() >= (checkpoint.len() + generated.len()) as u64);
    assert!(matches!(
        sender.send_with_receipt(&context, generated.clone(), [1; 32], false, |_| panic!(
            "unseeded exchange"
        )),
        Err(Error::Missing)
    ));
    assert!(matches!(
        sender.seed(&context, generated.clone(), [1; 32], |_| panic!(
            "generic runtime seed"
        )),
        Err(Error::Unsupported)
    ));
    assert!(matches!(
        sender.seed_schema(&context, [1; 32], |_| panic!("runtime schema seed")),
        Err(Error::Unsupported)
    ));
    let seeded = sender
        .seed_checkpoint(&context, [2; 32], |frames| {
            exchange(&mut endpoint, &context, frames)
        })
        .unwrap();
    assert_eq!(seeded.object, object_id(&checkpoint));
    let mut residual = generated.clone();
    residual[11] ^= 0x5a;
    for (index, bytes) in [generated.clone(), residual.clone()]
        .into_iter()
        .enumerate()
    {
        let receipt = sender
            .send_with_receipt(
                &context,
                bytes.clone(),
                [3 + index as u8; 32],
                false,
                |frames| exchange(&mut endpoint, &context, frames),
            )
            .unwrap();
        assert_eq!(receipt.transfer.mode, Mode::RuntimeV1);
        assert_eq!(receipt.transfer.object, object_id(&bytes));
        let ObjectSelection::Runtime(selection) = receipt.selection else {
            panic!("missing runtime receipt")
        };
        assert_eq!(selection.selected, receipt.transfer.mode);
        assert!(selection
            .candidates
            .iter()
            .any(|candidate| candidate.mode == Mode::RuntimeV1));
        assert!(selection.expected_http_body_bytes > 0);
    }
    drop(endpoint);
    let mut endpoint = open(&cache);
    for bytes in [checkpoint, generated, residual] {
        assert_eq!(
            endpoint.read_object(&context, object_id(&bytes)).unwrap(),
            bytes
        );
    }
}

#[test]
fn runtime_missing_receiver_state_has_no_hidden_reseed_and_scope_change_is_permanent() {
    let context = wan::context([72; 32]);
    let directory = tempfile::tempdir().unwrap();
    let cache = directory.path().join("cache");
    let mut endpoint = open(&cache);
    let prediction = prediction();
    let checkpoint = object_id(prediction.checkpoint());
    let target = prediction.prediction().to_vec();
    let mut sender = ObjectSender::with_runtime(&context, prediction).unwrap();
    sender
        .seed_checkpoint(&context, [1; 32], |frames| {
            exchange(&mut endpoint, &context, frames)
        })
        .unwrap();
    std::fs::remove_file(
        cache
            .join(hex(&context.namespace().unwrap()))
            .join(format!("{}.object", hex(&checkpoint))),
    )
    .unwrap();
    let mut calls = 0;
    assert!(matches!(
        sender.send_with_receipt(&context, target.clone(), [2; 32], false, |frames| {
            calls += 1;
            exchange(&mut endpoint, &context, frames)
        }),
        Err(Error::Missing)
    ));
    assert_eq!(calls, 1);
    assert!(endpoint.read_object(&context, object_id(&target)).is_err());
    assert!(matches!(
        sender.send(&context, target.clone(), [3; 32], false, |_| panic!(
            "implicit retry"
        )),
        Err(Error::Missing)
    ));
    sender
        .seed_checkpoint(&context, [4; 32], |frames| {
            exchange(&mut endpoint, &context, frames)
        })
        .unwrap();
    let sent = sender
        .send(&context, target.clone(), [5; 32], false, |frames| {
            exchange(&mut endpoint, &context, frames)
        })
        .unwrap();
    assert_eq!(sent.mode, Mode::RuntimeV1);
    sender.invalidate_receiver_state();
    assert!(matches!(
        sender.send(&context, target.clone(), [6; 32], false, |_| panic!(
            "invalid readiness"
        )),
        Err(Error::Missing)
    ));
    let mut changed = context.clone();
    changed.policy_epoch += 1;
    assert!(matches!(
        sender.seed_checkpoint(&changed, [7; 32], |_| panic!("changed scope")),
        Err(Error::Unauthorized)
    ));
    assert_eq!(sender.retained_charge(), 0);
    assert!(matches!(
        sender.seed_checkpoint(&context, [8; 32], |_| panic!("revived old scope")),
        Err(Error::Unauthorized)
    ));
}

#[test]
fn warm_and_fixed_profiles_report_only_their_existing_policy() {
    let context = wan::context([73; 32]);
    let schema = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut x = 117u64;
    let bytes: Vec<u8> = (0..65537)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect();
    for mode in [ObjectMode::Raw, ObjectMode::Whole, ObjectMode::Warm] {
        let directory = tempfile::tempdir().unwrap();
        let mut endpoint = open(&directory.path().join("cache"));
        let mut sender = ObjectSender::new(
            mode,
            &context,
            (mode == ObjectMode::Warm).then_some(schema.as_slice()),
        )
        .unwrap();
        assert!(matches!(
            sender.seed_checkpoint(&context, [1; 32], |_| panic!("non-runtime checkpoint")),
            Err(Error::Unsupported)
        ));
        sender
            .seed(&context, bytes.clone(), [2; 32], |frames| {
                exchange(&mut endpoint, &context, frames)
            })
            .unwrap();
        let mut edited = bytes.clone();
        edited[100] ^= 1;
        let receipt = sender
            .send_with_receipt(&context, edited.clone(), [3; 32], false, |frames| {
                exchange(&mut endpoint, &context, frames)
            })
            .unwrap();
        assert_eq!(receipt.transfer.object, object_id(&edited));
        match (mode, receipt.selection) {
            (ObjectMode::Warm, ObjectSelection::Warm(selection)) => {
                assert_eq!(selection.selected_mode, Mode::SparseV1);
                assert_eq!(selection.selected_mode, receipt.transfer.mode);
                let repeated = sender
                    .send_with_receipt(&context, edited, [4; 32], false, |frames| {
                        exchange(&mut endpoint, &context, frames)
                    })
                    .unwrap();
                assert_eq!(repeated.transfer.data_bytes, 0);
                let ObjectSelection::Warm(selection) = repeated.selection else {
                    panic!("missing warm receipt")
                };
                assert_eq!(selection.candidates.len(), 1);
                assert_eq!(selection.selected_mode, Mode::Raw);
            }
            (ObjectMode::Raw | ObjectMode::Whole, ObjectSelection::Fixed) => {}
            _ => panic!("misrepresented candidate policy"),
        }
    }
}
