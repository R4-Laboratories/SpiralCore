use spiralcore_gateway_core::object::{ObjectMode, ObjectSelection, ObjectSender};
use spiralcore_transfer::{
    endpoint::{ColdGraphPolicy, Context, Endpoint},
    hex, object_id,
    protocol::{Frame, Mode, Offer},
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
        &State::seeded(4096).unwrap().encode().unwrap(),
        &Recipe {
            steps: 48,
            payload: 1.25,
            output: OutputKind::Trace,
        },
    )
    .unwrap()
}
fn opaque() -> Vec<u8> {
    let mut x = 719u64;
    (0..262_144)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

#[test]
fn one_combined_sender_uses_runtime_rolling_sparse_and_cheap_repeats_with_exact_reopen() {
    let context = wan::context([91; 32]);
    let schema = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let prediction = prediction();
    let checkpoint = prediction.checkpoint().to_vec();
    let generated = prediction.prediction().to_vec();
    let mut residual = generated.clone();
    residual[4096..4608].iter_mut().for_each(|b| *b ^= 0x5a);
    let opaque = opaque();
    let mut edited = opaque.clone();
    edited[8192..8704].iter_mut().for_each(|b| *b ^= 0xa5);
    let directory = tempfile::tempdir().unwrap();
    let cache = directory.path().join("cache");
    let mut endpoint = open(&cache);
    assert!(matches!(
        ObjectSender::new(ObjectMode::Combined, &context, Some(&schema)),
        Err(Error::Malformed)
    ));
    let mut sender = ObjectSender::with_combined(&context, prediction, &schema).unwrap();
    sender
        .seed_checkpoint(&context, [1; 32], |f| exchange(&mut endpoint, &context, f))
        .unwrap();
    sender
        .seed_schema(&context, [2; 32], |f| exchange(&mut endpoint, &context, f))
        .unwrap();
    let mut rolling = None;
    for (index, bytes) in [
        generated.clone(),
        residual.clone(),
        residual.clone(),
        opaque.clone(),
        edited.clone(),
        edited.clone(),
    ]
    .into_iter()
    .enumerate()
    {
        if index == 4 {
            // Auxiliary ACKs must leave the previous opaque target as the rolling base.
            sender
                .seed_checkpoint(&context, [20; 32], |f| exchange(&mut endpoint, &context, f))
                .unwrap();
            sender
                .seed_schema(&context, [21; 32], |f| exchange(&mut endpoint, &context, f))
                .unwrap();
        }
        let mut offers = Vec::new();
        let mut frames_seen = 0;
        let receipt = sender
            .send_with_receipt(
                &context,
                bytes.clone(),
                [index as u8 + 3; 32],
                false,
                |frames| {
                    frames_seen += frames.len();
                    for frame in frames.iter().filter(|frame| frame.kind == 1) {
                        offers.push(Offer::decode(&frame.body)?);
                    }
                    exchange(&mut endpoint, &context, frames)
                },
            )
            .unwrap();
        let ObjectSelection::Combined(selection) = receipt.selection else {
            panic!("combined receipt required");
        };
        assert_eq!(receipt.transfer.object, object_id(&bytes));
        assert_eq!(selection.selected_mode, receipt.transfer.mode);
        assert!(selection.checkpoint_ready && selection.schema_ready);
        assert_eq!(offers.len(), 1);
        match receipt.transfer.mode {
            Mode::RuntimeV1 => assert_eq!(offers[0].base, Some(object_id(&checkpoint))),
            Mode::SparseV1 => assert_eq!(offers[0].base, rolling),
            Mode::SchemaZstdV1 => assert_eq!(offers[0].base, Some(object_id(&schema))),
            Mode::Raw | Mode::ZstdWholeV1 => assert_eq!(offers[0].base, None),
            other => panic!("unexpected combined candidate {other:?}"),
        }
        if index == 0 {
            assert_eq!(receipt.transfer.mode, Mode::RuntimeV1);
        }
        if index == 4 {
            assert_eq!(receipt.transfer.mode, Mode::SparseV1);
        }
        if matches!(index, 2 | 5) {
            assert_eq!(receipt.transfer.mode, Mode::Raw);
            assert!(selection.exact_repeat_probe);
            assert_eq!(selection.candidates.len(), 1);
            assert!(selection.pruned.is_empty());
            assert_eq!(frames_seen, 1);
            assert_eq!(receipt.transfer.data_bytes, 0);
        } else {
            assert!(!selection.exact_repeat_probe);
        }
        rolling = Some(object_id(&bytes));
    }
    // Force replay from the fixed checkpoint after a different rolling target.
    std::fs::remove_file(
        cache
            .join(hex(&context.namespace().unwrap()))
            .join(format!("{}.object", hex(&object_id(&generated)))),
    )
    .unwrap();
    let receipt = sender
        .send_with_receipt(&context, generated.clone(), [30; 32], false, |frames| {
            let offer = Offer::decode(&frames[0].body)?;
            assert_eq!(offer.mode, Mode::RuntimeV1);
            assert_eq!(offer.base, Some(object_id(&checkpoint)));
            exchange(&mut endpoint, &context, frames)
        })
        .unwrap();
    assert_eq!(receipt.transfer.mode, Mode::RuntimeV1);
    drop(endpoint);
    let mut reopened = open(&cache);
    for bytes in [checkpoint, schema, generated, residual, opaque, edited] {
        assert_eq!(
            reopened.read_object(&context, object_id(&bytes)).unwrap(),
            bytes
        );
    }
}
