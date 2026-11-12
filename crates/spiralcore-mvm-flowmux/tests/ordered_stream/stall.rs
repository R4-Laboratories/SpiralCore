use cpu_time::ProcessTime;
use spiralcore_mvm_flowmux::{
    stall::{PendingSample, StallObserver, StallSample},
    tls::{TlsCarrier, TlsCounters},
};
use std::{
    collections::VecDeque,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Sample {
    at_ms: u128,
    process_cpu_ms: Option<u128>,
    progress: [u64; 4],
    quiet_ms: [u128; 4],
    peers: [StallSample; 2],
    tls: [TlsCounters; 2],
}

/// Read-only polling. Dropping the sole sender wakes the sampler on return or unwinding. Keep a
/// fixed ring near the original deadline, irrespective of workload duration.
pub(super) fn watch(
    stop: std::sync::mpsc::Receiver<()>,
    peers: [StallObserver; 2],
    tls: [&TlsCarrier; 2],
    progress: &[AtomicU64; 4],
    began: Instant,
    blocks: u64,
    fixture: &str,
) {
    let mut last = [0; 4];
    let mut changed = [Instant::now(); 4];
    let cpu = ProcessTime::try_now().ok();
    let mut ring = VecDeque::with_capacity(8);
    let mut observed = 0u64;
    loop {
        let now = Instant::now();
        let counts = progress.each_ref().map(|v| v.load(Ordering::Relaxed));
        for i in 0..4 {
            if last[i] != counts[i] {
                changed[i] = now;
            }
        }
        last = counts;
        let quiet = changed.map(|time| now.duration_since(time).as_millis());
        let snapshot = peers.each_ref().map(|peer| peer.sample(now));
        let near_deadline = snapshot.iter().any(|p| {
            matches!(
                p.pending,
                PendingSample::Frame {
                    remaining_ms: 0..=750,
                    ..
                }
            )
        });
        if near_deadline || (0..4).any(|i| counts[i] < blocks && quiet[i] >= 1250) {
            if ring.len() == 8 {
                ring.pop_front();
            }
            ring.push_back(Sample {
                at_ms: now.duration_since(began).as_millis(),
                process_cpu_ms: cpu
                    .and_then(|t| t.try_elapsed().ok())
                    .map(|t| t.as_millis()),
                progress: counts,
                quiet_ms: quiet,
                peers: snapshot,
                tls: tls.map(TlsCarrier::counters),
            });
            observed = observed.saturating_add(1);
        }
        match stop.recv_timeout(Duration::from_millis(100)) {
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            _ => break,
        }
    }
    let mut out = std::io::stderr().lock();
    let _ = writeln!(
        out,
        "ORDERED_STALL_WINDOW observed={observed} retained={} fixture={fixture} interval_ms=100 threshold_ms=1250 original_operation_ms=2000",
        ring.len()
    );
    for sample in ring {
        let _ = writeln!(
            out,
            "ORDERED_STALL fixture={fixture} at_ms={} process_cpu_ms={:?} progress={:?} quiet_ms={:?} peers={:?} tls={:?}",
            sample.at_ms,
            sample.process_cpu_ms,
            sample.progress,
            sample.quiet_ms,
            sample.peers,
            sample.tls
        );
    }
}
