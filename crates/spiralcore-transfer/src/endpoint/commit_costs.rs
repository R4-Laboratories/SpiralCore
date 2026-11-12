//! Test-only attribution of a full ordered object turn after the 64-slot cache fills.
use super::*;
use crate::{
    resource_profile::ResourceProfile,
    v1::{
        Capabilities, ObjectEndpoint, ObjectMode, ObjectSender, OrderedBinding, OrderedLane,
        OrderedObjectEndpoint,
    },
};
use cpu_time::ThreadTime;
use std::{cell::RefCell, time::Instant};

#[derive(Clone, Copy, Debug)]
pub(super) enum Stage {
    Quota,
    Victim,
    FileSync,
    NamespacePublish,
    VerifyPublished,
    Readback,
    Block,
}
const STAGES: [Stage; 7] = [
    Stage::Quota,
    Stage::Victim,
    Stage::FileSync,
    Stage::NamespacePublish,
    Stage::VerifyPublished,
    Stage::Readback,
    Stage::Block,
];

#[derive(Clone, Copy, Default)]
struct Totals {
    calls: u64,
    wall_ns: u128,
    thread_cpu_ns: u128,
}
thread_local! {
    static COUNTERS: RefCell<Option<[Totals; STAGES.len()]>> = const { RefCell::new(None) };
}

pub(super) struct Span {
    stage: Stage,
    wall: Instant,
    cpu: ThreadTime,
}
pub(super) fn span(stage: Stage) -> Option<Span> {
    COUNTERS
        .with_borrow(|counts| counts.is_some())
        .then(|| Span {
            stage,
            wall: Instant::now(),
            cpu: ThreadTime::now(),
        })
}
impl Drop for Span {
    fn drop(&mut self) {
        let cpu_ns = self.cpu.elapsed().as_nanos();
        let wall_ns = self.wall.elapsed().as_nanos();
        COUNTERS.with_borrow_mut(|counts| {
            let stage = &mut counts.as_mut().unwrap()[self.stage as usize];
            stage.calls += 1;
            stage.wall_ns += wall_ns;
            stage.thread_cpu_ns += cpu_ns;
        });
    }
}

#[test]
#[ignore = "explicit native filesystem timing witness"]
fn cold_whole_ordered_commit_after_full_cache() {
    const WARMUP: u64 = 64;
    const BLOCKS: u64 = 128;
    const SIZE: usize = 16384;
    let ctx = crate::wan::context([7; 32]);
    let caps = Capabilities::small_v1();
    let profile = caps.negotiate(&caps).unwrap();
    let res = ResourceProfile::SmallV1;
    for repetition in 0..3 {
        let root = metadata_tests::Fixture::new();
        let inner = ObjectEndpoint::open_with_resource_profile(
            &root.0,
            res.receiver_limits(),
            crate::wan::BenchAuthority,
            profile,
            res,
        )
        .unwrap();
        let binding = OrderedBinding::new(
            &ctx,
            ctx.namespace().unwrap(),
            0,
            b"application\0",
            b"artifact\0",
        )
        .unwrap();
        let mut endpoint = OrderedObjectEndpoint::new(inner, binding.clone(), &ctx).unwrap();
        let mut sender =
            ObjectSender::new_with_resource_profile(ObjectMode::Whole, &ctx, None, profile, res)
                .unwrap();
        let lane = OrderedLane::Application;
        let mut request_bytes = 0;
        let mut reply_bytes = 0;
        for ordinal in 0..WARMUP + BLOCKS {
            if ordinal == WARMUP {
                COUNTERS
                    .with_borrow_mut(|counts| *counts = Some([Totals::default(); STAGES.len()]));
            }
            let mut bytes = vec![0x61; SIZE];
            bytes[..8].copy_from_slice(&ordinal.to_be_bytes());
            let _block = span(Stage::Block);
            let stats = sender
                .send(
                    &ctx,
                    bytes.clone(),
                    binding.transfer_id(lane, ordinal),
                    false,
                    |frames| {
                        frames
                            .iter()
                            .map(|frame| endpoint.handle(lane, ordinal, &ctx, frame, 1))
                            .collect()
                    },
                )
                .unwrap();
            if ordinal >= WARMUP {
                request_bytes += stats.request_bytes;
                reply_bytes += stats.response_bytes;
            }
            let _readback = span(Stage::Readback);
            assert_eq!(endpoint.read_object(&ctx, stats.object).unwrap(), bytes);
        }
        let result = COUNTERS.with_borrow_mut(Option::take).unwrap();
        let snapshot = endpoint.snapshot(&ctx).unwrap();
        assert_eq!(snapshot.publications, WARMUP as usize);
        assert_eq!(snapshot.evictions, BLOCKS);
        for stage in STAGES {
            let totals = result[stage as usize];
            let calls = if matches!(stage, Stage::Quota) {
                BLOCKS * 2
            } else {
                BLOCKS
            };
            assert_eq!(totals.calls, calls, "{stage:?}");
            eprintln!("ORDERED_COMMIT_COST trial={repetition} blocks={BLOCKS} stage={stage:?} calls={} wall_ns={} thread_cpu_ns={}", totals.calls, totals.wall_ns, totals.thread_cpu_ns);
        }
        eprintln!("ORDERED_COMMIT_EXACT trial={repetition} blocks={BLOCKS} plaintext={} object_request={request_bytes} object_reply={reply_bytes} evictions={}", BLOCKS * SIZE as u64, snapshot.evictions);
    }
}
