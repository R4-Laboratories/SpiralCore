//! Bounded concurrent TCP-loopback mailbox qualification; not native MVM/WAN.
#[allow(dead_code)]
#[path = "service_capacity.rs"]
mod capacity;
use capacity::{Pair, Progress};
use cpu_time::{ProcessTime, ThreadTime};
use mvm_core::mailbox::encode_envelope;
use serde_json::{Value, json};
use spiralcore_mvm_adapter::{
    broker::BrokerContext,
    issuance::IssuancePolicy,
    service::{CallStatus, HostBudget},
    service_sender::{self, MessageOptions},
};
use std::{
    fs::OpenOptions,
    io::Write,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const AUTHORIZATION_LIFETIME_MS: i64 = 900_000;
const MAX_OPERATIONS: usize = 100_000;
const MAX_DURATION_MS: u64 = 360_000;
const PAYLOAD_BYTES: usize = 4096;
const OFFERED_TOTAL_PER_SECOND: u64 = 10;

fn payload(index: usize) -> Vec<u8> {
    let mut payload = (0..PAYLOAD_BYTES)
        .map(|i| (i as u8).wrapping_mul(73))
        .collect::<Vec<_>>();
    payload[..8].copy_from_slice(&(index as u64).to_le_bytes());
    payload[PAYLOAD_BYTES - 1] = 0;
    payload
}
fn emit(out: &mut impl Write, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, row)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}
fn budget() -> Result<HostBudget> {
    let (limits, control) = capacity::limits();
    Ok(HostBudget::with_control(limits, control)?)
}
fn cleanup(pair: &mut Pair, broker: &BrokerContext) -> Result<()> {
    broker.with_time(|_, now| -> Result<_> {
        pair.caller.service.expire(now)?;
        pair.receiver.service.expire(now)?;
        pair.caller.tracker.expire(now)?;
        pair.receiver.tracker.expire(now)?;
        Ok(())
    })
}
fn state(pair: &Pair) -> Value {
    let a = pair.caller.service.snapshot();
    let b = pair.receiver.service.snapshot();
    let ta = pair.caller.tracker.snapshot();
    let tb = pair.receiver.tracker.snapshot();
    json!({"fifo_scopes":a.fifo_scopes+b.fifo_scopes,"fifo_entries":a.fifo_entries+b.fifo_entries,
        "service_accepted":a.accepted+b.accepted,"service_calls":a.calls+b.calls,
        "tracker_entries":ta.retained_entries+tb.retained_entries,
        "tracker_attempts":ta.attempts+tb.attempts,
        "tracker_request_indices":ta.request_indices+tb.request_indices,
        "tracker_pending_receipts":ta.pending_receipts+tb.pending_receipts,
        "next_tracker_expiry_monotonic_ms":ta.next_expiry_monotonic_ms.into_iter().chain(tb.next_expiry_monotonic_ms).min()})
}
fn counters(pair: &Pair) -> Result<Value> {
    let a = pair.caller.stream.counters()?;
    let b = pair.receiver.stream.counters()?;
    Ok(
        json!({"caller_to_receiver_bytes":a.envelope_bytes_sent+a.framing_bytes_sent,
        "receiver_to_caller_bytes":b.envelope_bytes_sent+b.framing_bytes_sent,
        "frames_sent":a.frames_sent+b.frames_sent,
        "agreement":a.envelope_bytes_sent==b.envelope_bytes_received
            &&b.envelope_bytes_sent==a.envelope_bytes_received
            &&a.framing_bytes_sent==b.framing_bytes_received
            &&b.framing_bytes_sent==a.framing_bytes_received}),
    )
}
fn close(pair: &Pair, index: usize) -> Vec<Value> {
    let mut errors = Vec::new();
    capacity::record_close_result(&mut errors, index, "caller", pair.caller.stream.close());
    capacity::record_close_result(&mut errors, index, "receiver", pair.receiver.stream.close());
    errors
}

struct Worker {
    index: usize,
    pairs: usize,
    operations: usize,
    duration_ms: u64,
    interference: bool,
    message_ttl_ms: i64,
    pair: Pair,
    broker: BrokerContext,
    start: Arc<Barrier>,
    abort: Arc<AtomicBool>,
    active: Arc<AtomicUsize>,
    peak_active: Arc<AtomicUsize>,
    checkpoints: SyncSender<Value>,
}
impl Worker {
    fn run(mut self) -> std::result::Result<Value, String> {
        let result = self.run_inner().map_err(|error| error.to_string());
        if result.is_err() {
            self.abort.store(true, Ordering::Relaxed);
        }
        result
    }
    fn run_inner(&mut self) -> Result<Value> {
        let mut completed = 0usize;
        let mut admitted = 0usize;
        let mut exact = 0usize;
        let mut fidelity_failures = 0usize;
        let mut latencies = Vec::with_capacity(self.operations.min(4096));
        let mut first_error = None;
        let mut malformed_refused = 0;
        let mut cancelled_refused = 0;
        let mut checkpoints_dropped = 0;
        let mut next_sample = 0;
        let mut tracker_peak = 0;
        let mut fifo_peak = 0;
        self.start.wait();
        let started = Instant::now();
        let thread_cpu = ThreadTime::now();
        for index in 0..self.operations {
            if self.abort.load(Ordering::Relaxed)
                || started.elapsed().as_millis() >= u128::from(MAX_DURATION_MS)
                || (self.duration_ms > 0
                    && started.elapsed().as_millis() >= u128::from(self.duration_ms))
            {
                break;
            }
            let current = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak_active.fetch_max(current, Ordering::SeqCst);
            if self.interference && self.index == 2 {
                self.broker.with_time(|_, now| -> Result<_> {
                    if self
                        .pair
                        .receiver
                        .service
                        .admit(&self.pair.receiver.incoming, b"malformed-envelope", now)
                        .is_ok()
                    {
                        return Err("malformed input admitted".into());
                    }
                    malformed_refused += 1;
                    Ok(())
                })?;
            }
            if self.interference && self.index == 0 && index == 0 {
                self.broker.with_time(|_, now| -> Result<_> {
                    // Cancel a separately identified, unsent Call. It does not
                    // consume this worker's persistent stream sequence.
                    let mut options = MessageOptions::fresh_for_binding(
                        &self.pair.caller.outgoing,
                        capacity::fixture::content_type(),
                        now,
                        now.unix_ms + self.message_ttl_ms,
                        IssuancePolicy::Exact,
                    )?;
                    options.reply_to = Some(self.pair.reply_to);
                    let prepared = service_sender::prepare(
                        &self.pair.caller.outgoing,
                        b"cancelled",
                        &options,
                        now,
                    )?;
                    let encoded = encode_envelope(prepared.envelope())?;
                    let handle = self.pair.caller.service.register_call(
                        &self.pair.caller.outgoing,
                        &encoded,
                        now,
                    )?;
                    if self
                        .pair
                        .caller
                        .service
                        .cancel(&self.pair.caller.outgoing, &handle, now)?
                        != CallStatus::Cancelled
                    {
                        return Err("Call cancellation failed".into());
                    }
                    cancelled_refused += 1;
                    Ok(())
                })?;
            }
            let began = Instant::now();
            let mut progress = Progress::default();
            let result = capacity::operation_with_behavior(
                &mut self.pair,
                &self.broker,
                true,
                index % 2 == 0,
                &payload(index),
                &mut progress,
                capacity::OperationBehavior {
                    message_ttl_ms: self.message_ttl_ms,
                    receiver_delay_ms: if self.interference && self.index == 1 {
                        50
                    } else {
                        0
                    },
                },
            );
            self.active.fetch_sub(1, Ordering::SeqCst);
            admitted += usize::from(progress.admitted);
            exact += progress.exact;
            if let Err(error) = result {
                let error = error.to_string();
                fidelity_failures += usize::from(error.contains("payload mismatch"));
                first_error = Some(
                    json!({"error":error,"stage":progress.stage,"operation":index,
                    "quota_refusal":progress.quota_refusal,
                    "diagnostic_scope":"refusal counter changed inside this operation's serialized local dispatch"}),
                );
                break;
            }
            completed += 1;
            latencies.push(began.elapsed().as_micros() as u64);
            let snapshot = state(&self.pair);
            tracker_peak = tracker_peak.max(snapshot["tracker_entries"].as_u64().unwrap_or(0));
            fifo_peak = fifo_peak.max(snapshot["fifo_scopes"].as_u64().unwrap_or(0));
            if started.elapsed().as_secs() >= next_sample {
                let checkpoint = json!({"event":"worker_checkpoint","worker":self.index,
                    "elapsed_ms":started.elapsed().as_millis() as u64,"completed_operations":completed,
                    "state":snapshot,"rss_bytes":capacity::rss_bytes(),"fd_count":capacity::fd_count()});
                if self.checkpoints.try_send(checkpoint).is_err() {
                    checkpoints_dropped += 1;
                }
                next_sample += 10;
            }
            if index % 20 == 0
                && (capacity::rss_bytes().is_some_and(|rss| rss > 512 * 1024 * 1024)
                    || capacity::fd_count().is_some_and(|fds| fds > 128))
            {
                self.abort.store(true, Ordering::Relaxed);
                first_error = Some(
                    json!({"error":"registered RSS or FD ceiling","stage":"resource_sampler"}),
                );
                break;
            }
            // The hot worker runs unpaced. Other pairs keep the registered
            // aggregate offered rate divided across the full pair population.
            if self.duration_ms > 0 && !(self.interference && self.index == 0) {
                let target_ms =
                    (index + 1) as u64 * self.pairs as u64 * 1000 / OFFERED_TOTAL_PER_SECOND;
                if let Some(wait) = Duration::from_millis(target_ms).checked_sub(started.elapsed())
                {
                    std::thread::sleep(wait);
                }
            }
        }
        let elapsed_us = started.elapsed().as_micros() as u64;
        let final_state = state(&self.pair);
        let mut transport_receive_cancelled = 0usize;
        let mut transport_receive_cancel_us = None;
        if self.interference && self.index + 1 == self.pairs {
            let token = mvm_core::mailbox::transport::CancellationToken::default();
            let cancel = token.clone();
            let cancel_thread = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                cancel.cancel();
            });
            let cancellation_started = Instant::now();
            let result = self
                .pair
                .caller
                .stream
                .receive(&token, Instant::now() + Duration::from_secs(2));
            cancel_thread
                .join()
                .map_err(|_| "transport cancellation worker panicked")?;
            transport_receive_cancel_us = Some(cancellation_started.elapsed().as_micros() as u64);
            if matches!(
                result,
                Err(spiralcore_mvm_adapter::live::LiveError::Cancelled)
            ) {
                transport_receive_cancelled = 1;
            } else {
                return Err("blocked transport receive did not observe cancellation".into());
            }
        }
        let counters = counters(&self.pair)?;
        let close_errors = close(&self.pair, self.index);
        Ok(
            json!({"worker":self.index,"completed_operations":completed,"admitted_requests":admitted,
            "exact_readbacks":exact,"fidelity_failures":fidelity_failures,"first_error":first_error,
            "incomplete_admitted_requests":admitted.saturating_sub(completed),"elapsed_us":elapsed_us,
            "worker_thread_cpu_us":thread_cpu.elapsed().as_micros() as u64,
            "thread_cpu_scope":"both endpoint roles and local broker actions on this pair worker; excludes other workers and controller",
            "latency":capacity::quantiles(&mut latencies),"counters":counters,"final_state":final_state,
            "post_operation_tracker_entry_peak":tracker_peak,"post_operation_fifo_scope_peak":fifo_peak,
            "malformed_refused":malformed_refused,"cancelled_refused":cancelled_refused,
            "transport_receive_cancelled":transport_receive_cancelled,"transport_receive_cancel_us":transport_receive_cancel_us,
            "cancellation_scope":"separate unsent Call plus terminal blocked receive; no claim of cancelling an admitted application operation remotely",
            "checkpoint_rows_dropped":checkpoints_dropped,"close_errors":close_errors}),
        )
    }
}

/// Short functional profile uses 30-second deadlines; paced measurements use
/// the explicit one-second load profile. Neither changes production defaults.
pub fn run_concurrent(
    endpoints: usize,
    operations: usize,
    duration_ms: u64,
    interference: bool,
) -> Result<Value> {
    run_concurrent_recorded(
        endpoints,
        operations,
        duration_ms,
        interference,
        &mut std::io::sink(),
    )
}
pub(crate) fn join_all<T>(
    workers: Vec<std::thread::JoinHandle<std::result::Result<T, String>>>,
) -> Result<Vec<T>> {
    // Joining is unconditional. Collecting Result directly would stop at the
    // first failed worker and detach all later handles.
    let joined = workers
        .into_iter()
        .map(|worker| match worker.join() {
            Ok(result) => result.map_err(|error| -> Box<dyn std::error::Error> { error.into() }),
            Err(_) => Err("sustained worker panicked".into()),
        })
        .collect::<Vec<_>>();
    joined.into_iter().collect()
}
pub(crate) fn run_concurrent_recorded(
    endpoints: usize,
    operations: usize,
    duration_ms: u64,
    interference: bool,
    out: &mut impl Write,
) -> Result<Value> {
    if ![2, 8, 32].contains(&endpoints)
        || operations == 0
        || operations > MAX_OPERATIONS
        || duration_ms > MAX_DURATION_MS
    {
        return Err("outside sustained probe registration".into());
    }
    let setup_wall = Instant::now();
    let setup_cpu = ProcessTime::now();
    let budget = budget()?;
    let broker = BrokerContext::new(budget.clone());
    let pairs = (0..endpoints / 2)
        .map(|index| capacity::pair_with_lifetime(index as u8, &budget, AUTHORIZATION_LIFETIME_MS))
        .collect::<Result<Vec<_>>>()?;
    let setup_wall_us = setup_wall.elapsed().as_micros() as u64;
    let setup_cpu_us = setup_cpu.elapsed().as_micros() as u64;
    let start = Arc::new(Barrier::new(pairs.len() + 1));
    let abort = Arc::new(AtomicBool::new(false));
    let active = Arc::new(AtomicUsize::new(0));
    let peak_active = Arc::new(AtomicUsize::new(0));
    let (send, receive) = sync_channel(32);
    let pair_count = pairs.len();
    let workers = pairs
        .into_iter()
        .enumerate()
        .map(|(index, pair)| Worker {
            index,
            pairs: pair_count,
            operations,
            duration_ms,
            interference,
            message_ttl_ms: if duration_ms == 0 { 30_000 } else { 1000 },
            pair,
            broker: broker.clone(),
            start: start.clone(),
            abort: abort.clone(),
            active: active.clone(),
            peak_active: peak_active.clone(),
            checkpoints: send.clone(),
        })
        .map(|worker| std::thread::spawn(move || worker.run()))
        .collect::<Vec<_>>();
    drop(send);
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    start.wait();
    let mut output_error = None;
    for checkpoint in receive {
        if let Err(error) = emit(out, &checkpoint) {
            abort.store(true, Ordering::Relaxed);
            output_error = Some(error);
            break;
        }
    }
    let rows = join_all(workers)?;
    if let Some(error) = output_error {
        return Err(error);
    }
    let sum = |key: &str| rows.iter().filter_map(|row| row[key].as_u64()).sum::<u64>();
    let forward = rows
        .iter()
        .filter_map(|row| row["counters"]["caller_to_receiver_bytes"].as_u64())
        .sum::<u64>();
    let reverse = rows
        .iter()
        .filter_map(|row| row["counters"]["receiver_to_caller_bytes"].as_u64())
        .sum::<u64>();
    let snapshot = budget.snapshot()?;
    let mut row = json!({"event":"concurrent_done","scope":"TCP loopback logical endpoints; no native MVM/WAN claim",
        "configured_endpoints":endpoints,"workers":pair_count,"pipeline_per_pair":1,
        "max_workers_with_operation_in_flight":peak_active.load(Ordering::SeqCst),
        "completed_operations":sum("completed_operations"),"exact_readbacks":sum("exact_readbacks"),
        "fidelity_failures":sum("fidelity_failures"),"incomplete_admitted_requests":sum("incomplete_admitted_requests"),
        "actual_caller_to_receiver_bytes":forward,"actual_receiver_to_caller_bytes":reverse,"actual_both_direction_bytes":forward+reverse,
        "sender_receiver_counters_match":rows.iter().all(|row|row["counters"]["agreement"]==true),
        "byte_scope":"actual canonical envelopes plus stream framing; excludes TCP/IP/retransmission",
        "setup_cpu_us":setup_cpu_us,"setup_wall_us":setup_wall_us,
        "process_cpu_us":cpu.elapsed().as_micros() as u64,"elapsed_us":wall.elapsed().as_micros() as u64,
        "peak_process_rss_bytes":capacity::rss_bytes(),"fd_count":capacity::fd_count(),
        "faults":{"malformed_refused":sum("malformed_refused"),"cancelled_refused":sum("cancelled_refused"),"transport_receive_cancelled":sum("transport_receive_cancelled")},
        "budget_refusals":snapshot.refusals,"final_budget_identities":snapshot.identities,
        "profiles":{"authorization_lifetime_ms":AUTHORIZATION_LIFETIME_MS,"message_ttl_ms":if duration_ms==0 {30_000}else{1000},
            "service_retention_ms":1000,"tracker_retention_extension_ms":300_000,"offered_total_operations_per_second":if duration_ms>0 {Some(OFFERED_TOTAL_PER_SECOND)}else{None},
            "interference":interference,"requested_duration_ms":duration_ms,"max_duration_ms":MAX_DURATION_MS,"max_operations_per_pair":operations,
            "rss_ceiling_bytes":512*1024*1024,"fd_ceiling":128},"per_worker":rows});
    let period_ms = pair_count as u64 * 1000 / OFFERED_TOTAL_PER_SECOND;
    let scheduled_per_worker = if duration_ms == 0 {
        operations as u64
    } else {
        (duration_ms.div_ceil(period_ms)).min(operations as u64)
    };
    row["scheduled_operations_per_paced_worker"] = json!(scheduled_per_worker);
    let workers_pass = rows.iter().all(|worker| {
        let error = &worker["first_error"];
        let hot = interference && duration_ms > 0 && worker["worker"] == 0;
        (hot && (error.is_null() || !error["quota_refusal"].is_null()))
            || (error.is_null()
                && worker["completed_operations"].as_u64() == Some(scheduled_per_worker)
                && worker["incomplete_admitted_requests"] == 0)
    });
    let pass = workers_pass
        && row["fidelity_failures"] == 0
        && row["sender_receiver_counters_match"] == true
        && row["completed_operations"]
            .as_u64()
            .is_some_and(|count| count > 0)
        && !(capacity::rss_bytes().is_some_and(|rss| rss > 512 * 1024 * 1024)
            || capacity::fd_count().is_some_and(|fds| fds > 128));
    row["qualification"] = json!(if pass { "PASS" } else { "FAIL" });
    emit(out, &row)?;
    if !pass {
        return Err("concurrent qualification gate failed; evidence preserved".into());
    }
    Ok(row)
}

/// Preserve one pair, all service/tracker/scope owners and its TCP stream while
/// crossing the complete terminal-tracker retention period after saturation.
fn run_recovery(out: &mut impl Write) -> Result<Value> {
    let budget = budget()?;
    let broker = BrokerContext::new(budget.clone());
    let mut pair = capacity::pair_with_lifetime(0, &budget, AUTHORIZATION_LIFETIME_MS)?;
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let mut completed = 0usize;
    let mut exact = 0usize;
    let mut failure = None;
    for index in 0..5000 {
        if wall.elapsed() >= Duration::from_secs(60)
            || capacity::rss_bytes().is_some_and(|rss| rss > 512 * 1024 * 1024)
            || capacity::fd_count().is_some_and(|fds| fds > 128)
        {
            emit(
                out,
                &json!({"event":"recovery_failed","stage":"pre_saturation_resource_bound","elapsed_us":wall.elapsed().as_micros() as u64}),
            )?;
            return Err("recovery saturation exceeded registered time or resource bound".into());
        }
        let mut progress = Progress::default();
        let result = capacity::operation(
            &mut pair,
            &broker,
            true,
            index % 2 == 0,
            &payload(index),
            &mut progress,
            1000,
        );
        exact += progress.exact;
        if let Err(error) = result {
            failure = Some(
                json!({"error":error.to_string(),"stage":progress.stage,"admitted":progress.admitted,
                "exact_readbacks":progress.exact,"operation":index,"quota_refusal":progress.quota_refusal}),
            );
            break;
        }
        completed += 1;
    }
    let at_saturation = state(&pair);
    let diagnostic = budget.snapshot()?.refusals;
    emit(
        out,
        &json!({"event":"saturation","completed_operations":completed,"exact_readbacks":exact,
        "failure":failure,"state":at_saturation,"budget_refusals":diagnostic,"elapsed_us":wall.elapsed().as_micros() as u64}),
    )?;
    // This known phase has received its receipt and consumed the destination
    // sequence, but no Reply sequence was allocated. Other phases cannot safely
    // resume by allocating a new sequence over a potentially missing FIFO item.
    if failure.as_ref().is_none_or(|f| {
        f["stage"] != "caller_observe_admission"
            || f["admitted"] != true
            || f["error"] != "Full"
            || f["quota_refusal"].is_null()
    }) {
        return Err(
            "saturation stopped outside registered recoverable phase; evidence preserved".into(),
        );
    }
    let wait = Instant::now();
    while wait.elapsed() < Duration::from_millis(301_100) {
        std::thread::sleep(
            Duration::from_secs(10)
                .min(Duration::from_millis(301_100).saturating_sub(wait.elapsed())),
        );
        cleanup(&mut pair, &broker)?;
        emit(
            out,
            &json!({"event":"recovery_wait","wait_ms":wait.elapsed().as_millis() as u64,"state":state(&pair),
            "budget_identities":budget.snapshot()?.identities,"rss_bytes":capacity::rss_bytes()}),
        )?;
    }
    cleanup(&mut pair, &broker)?;
    let before_resume = state(&pair);
    let budget_before_resume = budget.snapshot()?;
    let mut resumed_exact = 0;
    let mut resume = Vec::new();
    for (index, call) in [true, false].into_iter().enumerate() {
        let mut progress = Progress::default();
        let result = capacity::operation(
            &mut pair,
            &broker,
            true,
            call,
            &payload(5001 + index),
            &mut progress,
            1000,
        );
        resumed_exact += progress.exact;
        resume.push(json!({"call":call,"success":result.is_ok(),"error":result.err().map(|e|e.to_string()),"stage":progress.stage,"exact_readbacks":progress.exact}));
    }
    let row = json!({"event":"recovery_done","same_service_tracker_scope_transport_owners":true,"service_reset":false,
        "completed_before_saturation":completed,"exact_readbacks_before_saturation":exact,"saturation_failure":failure,
        "state_at_saturation":at_saturation,"state_after_full_expiry":before_resume,"state_after_resume":state(&pair),
        "budget_after_full_expiry":{"queued_messages":budget_before_resume.queued_messages,"active_calls":budget_before_resume.active_calls,"owner_count":budget_before_resume.owner_count},
        "wait_ms":wait.elapsed().as_millis() as u64,"resumed_exact_readbacks":resumed_exact,"resume":resume,
        "resumed_operations":resume.iter().filter(|r|r["success"]==true).count(),"counters":counters(&pair)?,
        "elapsed_us":wall.elapsed().as_micros() as u64,"process_cpu_us":cpu.elapsed().as_micros() as u64,
        "peak_process_rss_bytes":capacity::rss_bytes(),"authorization_lifetime_ms":AUTHORIZATION_LIFETIME_MS,
        "message_ttl_ms":1000,"tracker_retention_extension_ms":300_000,"budget_refusals":diagnostic,
        "scope":"wall-clock TCP-loopback retention recovery, no native MVM/WAN claim","close_errors":close(&pair,0)});
    emit(out, &row)?;
    if row["resumed_operations"] != 2
        || row["resumed_exact_readbacks"] != 3
        || row["state_after_full_expiry"]["tracker_entries"] != 0
        || row["state_after_full_expiry"]["tracker_request_indices"] != 0
        || row["state_after_full_expiry"]["fifo_scopes"]
            != row["state_at_saturation"]["fifo_scopes"]
        || budget_before_resume.queued_messages != 0
        || budget_before_resume.active_calls != 0
        || row["counters"]["agreement"] != true
        || capacity::rss_bytes().is_some_and(|rss| rss > 512 * 1024 * 1024)
        || capacity::fd_count().is_some_and(|fds| fds > 128)
    {
        return Err("full retention recovery gate failed; evidence preserved".into());
    }
    Ok(row)
}

#[allow(dead_code)]
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|s| s == "--recovery") && args.len() == 2 {
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[1])?;
        run_recovery(&mut out)?;
        return Ok(());
    }
    if args.len() != 6 || args[0] != "--concurrent" {
        return Err("usage: service_sustained --concurrent ENDPOINTS OPS_PER_PAIR DURATION_MS steady|interference NEW_OUTPUT; or --recovery NEW_OUTPUT".into());
    }
    let interference = match args[4].as_str() {
        "steady" => false,
        "interference" => true,
        _ => return Err("unknown scenario".into()),
    };
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[5])?;
    run_concurrent_recorded(
        args[1].parse()?,
        args[2].parse()?,
        args[3].parse()?,
        interference,
        &mut out,
    )?;
    Ok(())
}
