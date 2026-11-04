//! Component measurement only: real core frames/private-file publication;
//! no TLS/IP/WAN/guest claim. The outer process records its own peak RSS.
use cpu_time::ThreadTime;
use spiralcore_transfer::{
    endpoint::Context,
    hex, object_id,
    protocol::Frame,
    schema,
    shared_runtime::{OutputKind, Recipe, State},
    v1::{ObjectEndpoint, ObjectMode, ObjectSender},
    wan, Result,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

pub type SenderFactory = fn(ObjectMode, &Context, &[u8], &[u8], &Recipe) -> Result<ObjectSender>;
pub type ReceiverFactory = fn(&Path) -> Result<ObjectEndpoint<wan::BenchAuthority>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Store(PathBuf);
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn micros(value: std::time::Duration) -> u64 {
    u64::try_from(value.as_micros()).unwrap()
}
fn emit(records: &mut Option<std::fs::File>, row: serde_json::Value) {
    use std::io::Write;
    if let Some(records) = records {
        serde_json::to_writer(&mut *records, &row).unwrap();
        writeln!(records).unwrap();
    }
}
fn dispatch(
    receiver: &mut ObjectEndpoint<wan::BenchAuthority>,
    context: &Context,
    frames: &[Frame],
    cpu: &mut u64,
    wall: &mut u64,
) -> Result<Vec<Frame>> {
    let started = ThreadTime::now();
    let at = Instant::now();
    let result = (|| {
        let mut replies = Vec::new();
        for frame in frames {
            let reply = receiver.handle(context, frame, 0)?;
            let done = reply.kind == 103;
            replies.push(reply);
            if done {
                break;
            }
        }
        Ok(replies)
    })();
    *cpu = cpu.checked_add(micros(started.elapsed())).unwrap();
    *wall = wall.checked_add(micros(at.elapsed())).unwrap();
    result
}
pub fn run(
    label: &str,
    make_sender: SenderFactory,
    make_receiver: ReceiverFactory,
    operation_charge: Option<u64>,
) {
    let mut records = std::env::var_os("SPIRALCORE_RESOURCE_MEASURE_DIR").map(|directory| {
        let directory = PathBuf::from(directory);
        let meta = std::fs::symlink_metadata(&directory).unwrap();
        assert!(meta.is_dir() && !meta.file_type().is_symlink());
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!("{label}.jsonl")))
            .unwrap()
    });
    let context = wan::context([181; 32]);
    let artifact = schema::fit(&[
        b"{\"value\":0,\"kind\":\"reading\"}\n",
        b"{\"value\":1,\"kind\":\"reading\"}\n",
    ])
    .unwrap();
    let checkpoint = State::seeded(64).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 8,
        payload: 1.25,
        output: OutputKind::Trace,
    };
    let generated =
        spiralcore_transfer::shared_runtime::replay(&checkpoint, &recipe, 65536).unwrap();
    assert!(generated.len() <= 16384);
    let mut edited = generated.clone();
    let middle = edited.len() / 2;
    for byte in &mut edited[middle..middle + 32] {
        *byte ^= 0xa5;
    }
    let mut random = 37u64;
    let unrelated: Vec<u8> = (0..16384)
        .map(|_| {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            random as u8
        })
        .collect();
    let mut fit = Vec::new();
    for n in 0..1024 {
        fit.extend_from_slice(format!("{{\"value\":{n},\"kind\":\"reading\"}}\n").as_bytes());
    }
    fit.truncate(16384);
    let mut drift = b"different\tfields\tall-literal\n".repeat(1024);
    drift.truncate(16384);
    let short = fit[..600].to_vec();
    let cases = [
        ("runtime-fit", generated),
        ("edited", edited),
        ("unrelated", unrelated),
        ("schema-fit", fit),
        ("schema-drift", drift),
        ("short600", short.clone()),
        ("repeat-short600", short),
    ];
    for trial in 0..3 {
        for mode in [
            ObjectMode::Raw,
            ObjectMode::Whole,
            ObjectMode::Warm,
            ObjectMode::Runtime,
            ObjectMode::Combined,
        ] {
            let store = Store(std::env::temp_dir().join(format!(
                "spiral-resource-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )));
            let cpu = ThreadTime::now();
            let wall = Instant::now();
            let mut receiver = make_receiver(&store.0).unwrap();
            let mut sender = make_sender(mode, &context, &artifact, &checkpoint, &recipe).unwrap();
            let setup_cpu = micros(cpu.elapsed());
            let setup_wall = micros(wall.elapsed());
            let mut receiver_cpu = 0;
            let mut receiver_wall = 0;
            let cpu = ThreadTime::now();
            let wall = Instant::now();
            let mut request_bytes = 0;
            let mut response_bytes = 0;
            let mut exchange = |frames: &[Frame]| {
                dispatch(
                    &mut receiver,
                    &context,
                    frames,
                    &mut receiver_cpu,
                    &mut receiver_wall,
                )
            };
            if matches!(mode, ObjectMode::Runtime | ObjectMode::Combined) {
                let stats = sender
                    .seed_checkpoint(&context, [1; 32], &mut exchange)
                    .unwrap();
                request_bytes += stats.request_bytes;
                response_bytes += stats.response_bytes;
            }
            if matches!(mode, ObjectMode::Warm | ObjectMode::Combined) {
                let stats = sender
                    .seed_schema(&context, [2; 32], &mut exchange)
                    .unwrap();
                request_bytes += stats.request_bytes;
                response_bytes += stats.response_bytes;
            }
            let seed_cpu = micros(cpu.elapsed());
            let seed_wall = micros(wall.elapsed());
            emit(
                &mut records,
                serde_json::json!({"profile":label,"trial":trial,"mode":format!("{mode:?}"),"phase":"setup-seed","setup_thread_cpu_us":setup_cpu,"setup_wall_us":setup_wall,"seed_thread_cpu_us":seed_cpu,"seed_wall_us":seed_wall,"receiver_dispatch_cpu_us":receiver_cpu,"receiver_dispatch_wall_us":receiver_wall,"request_frame_bytes":request_bytes,"response_frame_bytes":response_bytes,"retained_declared_bytes":sender.retained_charge(),"aggregate_declared_operation_bytes":operation_charge,"scope":"in-process core frames and private-file publication; no network"}),
            );
            for (position, (name, bytes)) in cases.iter().enumerate() {
                let mut id = [10 + position as u8; 32];
                id[31] = trial as u8;
                let mut receiver_cpu = 0;
                let mut receiver_wall = 0;
                let cpu = ThreadTime::now();
                let wall = Instant::now();
                let receipt = sender
                    .send_with_receipt(&context, bytes.clone(), id, true, |frames| {
                        dispatch(
                            &mut receiver,
                            &context,
                            frames,
                            &mut receiver_cpu,
                            &mut receiver_wall,
                        )
                    })
                    .unwrap();
                let total_cpu = micros(cpu.elapsed());
                let total_wall = micros(wall.elapsed());
                let verify_cpu = ThreadTime::now();
                let verify_wall = Instant::now();
                let readback = receiver
                    .read_object(&context, receipt.transfer.object)
                    .unwrap();
                assert_eq!(&readback, bytes);
                assert_eq!(receipt.transfer.object, object_id(bytes));
                emit(
                    &mut records,
                    serde_json::json!({"profile":label,"trial":trial,"mode":format!("{mode:?}"),"phase":"target","workload":name,"length":bytes.len(),"object":hex(&receipt.transfer.object),"selected":format!("{:?}",receipt.transfer.mode),"total_thread_cpu_us":total_cpu,"total_wall_us":total_wall,"receiver_dispatch_cpu_us":receiver_cpu,"receiver_dispatch_wall_us":receiver_wall,"sender_outside_dispatch_cpu_us":total_cpu.checked_sub(receiver_cpu).unwrap(),"sender_outside_dispatch_wall_us":total_wall.checked_sub(receiver_wall).unwrap(),"verification_cpu_us":micros(verify_cpu.elapsed()),"verification_wall_us":micros(verify_wall.elapsed()),"request_frame_bytes":receipt.transfer.request_bytes,"response_frame_bytes":receipt.transfer.response_bytes,"exact_readback":true,"retained_declared_bytes":sender.retained_charge()}),
                );
            }
        }
    }
}
