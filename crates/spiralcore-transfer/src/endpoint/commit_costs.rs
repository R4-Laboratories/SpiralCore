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
    held_read_comparison();
}

// Compare the original two checked opens with the held-file implementation on
// the same published bytes, using fresh authority and I/O for every operation.
fn held_read_comparison() {
    const READS: usize = 256;
    let context = crate::wan::context([7; 32]);
    let ns = context.namespace().unwrap();
    let resources = ResourceProfile::SmallV1;
    let root = metadata_tests::Fixture::new();
    let mut endpoint = Endpoint::open_with_resource_profile(
        &root.0,
        resources.receiver_limits(),
        crate::wan::BenchAuthority,
        resources,
    )
    .unwrap();
    let mut expected = vec![0x73; 16384];
    expected[16367..].fill(0);
    let offer = Offer::cold(&expected, Mode::Raw);
    endpoint.publish(&context, ns, &offer, &expected, None).unwrap();
    let mut run = |held: bool| {
        let cpu = ThreadTime::now();
        let wall = Instant::now();
        for _ in 0..READS {
            let (bytes, disk) = if held {
                endpoint.ordered_cache_read(&context, offer.object).unwrap()
            } else {
                // Original 4266155f ordered_cache_read on the same authority,
                // profile, persisted object and allocation reservation.
                endpoint.authority.authorize(&context).unwrap();
                endpoint.ordered_bounds().unwrap();
                let p = endpoint.publication(context.namespace().unwrap(), offer.object).unwrap();
                assert!(p.len <= resources.max_artifact_len() as u64);
                endpoint.reserve(p.len.checked_mul(2).unwrap()).unwrap();
                let bytes = read_publication(&p).unwrap();
                endpoint.authority.authorize(&context).unwrap();
                (bytes, p.len + if p.manifest.is_some() { 81 } else { 49 })
            };
            assert_eq!(bytes, expected);
            assert_eq!(disk, offer.len + 49);
        }
        (wall.elapsed().as_nanos(), cpu.elapsed().as_nanos())
    };
    for pair in 0..6 {
        let (prior, held) = if pair % 2 == 0 {
            (run(false), run(true))
        } else {
            let held = run(true);
            (run(false), held)
        };
        eprintln!("ORDERED_HELD_READ pair={pair} reads_each={READS} bytes_each={} prior_wall_ns={} prior_thread_cpu_ns={} held_wall_ns={} held_thread_cpu_ns={}",
            expected.len() * READS, prior.0, prior.1, held.0, held.1);
    }

}
