//! Bounded, paced native-service endpoint load. This is TCP loopback, not WAN/MVM capacity.
#[path = "service_capacity/fixtures.rs"]
mod fixture;
use cpu_time::ProcessTime;
use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{
    encode_envelope,
    transport::{CancellationToken, RouteId},
};
use serde_json::{Value, json};
use spiralcore_mvm_adapter::{
    broker::BrokerContext,
    issuance::IssuancePolicy,
    live::{HandshakeRole, StreamTransport},
    receipt::{PreparedReceipt, ReceiptObservation, ReceiptReturnConfig, ReceiptTracker},
    service::{
        CallStatus, ControlReserve, HostBudget, LocalMailboxService, MailboxBinding, ServiceLimits,
    },
    service_sender::{self, MessageOptions, OrderingStream},
};
use std::{
    fs::OpenOptions,
    io::Write,
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const POLICY: IssuancePolicy = IssuancePolicy::Exact;
const MAX_OPERATIONS: usize = 100_000;

struct Endpoint {
    service: LocalMailboxService,
    tracker: ReceiptTracker,
    incoming: MailboxBinding,
    outgoing: MailboxBinding,
    cast_incoming: MailboxBinding,
    cast_outgoing: MailboxBinding,
    control: MailboxBinding,
    returns: ReceiptReturnConfig,
    ordering: OrderingStream,
    cast_ordering: OrderingStream,
    stream: StreamTransport<TcpStream>,
}
struct Pair {
    caller: Endpoint,
    receiver: Endpoint,
    reply_to: ReplyRoute,
}
#[derive(Default)]
struct Progress {
    admitted: bool,
    exact: usize,
    receiver_exact: bool,
    admission_us: Option<u64>,
    reply_us: Option<u64>,
    stage: &'static str,
}

fn limits() -> (ServiceLimits, ControlReserve) {
    let mut l = ServiceLimits {
        retention_ms: 1000,
        ..ServiceLimits::default()
    };
    l.host.identities = 65536;
    l.tenant.identities = 32768;
    l.mailbox.identities = 32768;
    l.sender.identities = 16384;
    let mut r = ControlReserve::default();
    r.host.identities = 16384;
    r.tenant.identities = 8192;
    r.mailbox.identities = 8192;
    r.sender.identities = 4096;
    (l, r)
}
fn endpoint(
    peer: u8,
    caller: bool,
    budget: &HostBudget,
    stream: StreamTransport<TcpStream>,
) -> Result<Endpoint> {
    let incoming_kind = if caller {
        MailboxMessageKind::Reply
    } else {
        MailboxMessageKind::Call
    };
    let outgoing_kind = if caller {
        MailboxMessageKind::Call
    } else {
        MailboxMessageKind::Reply
    };
    Ok(Endpoint {
        service: LocalMailboxService::new(budget.clone())?,
        tracker: ReceiptTracker::new(budget.clone(), 4096)?,
        incoming: fixture::binding(peer, incoming_kind, !caller, false, false),
        outgoing: fixture::binding(peer, outgoing_kind, caller, false, true),
        cast_incoming: fixture::binding(peer, MailboxMessageKind::Cast, !caller, false, false),
        cast_outgoing: fixture::binding(peer, MailboxMessageKind::Cast, caller, false, true),
        control: fixture::binding(peer, MailboxMessageKind::Cast, !caller, true, false),
        returns: ReceiptReturnConfig::new(
            fixture::binding(peer, MailboxMessageKind::Cast, caller, true, true),
            budget.clone(),
            fixture::receipt_key(!caller).verifying_key(),
        )?,
        ordering: OrderingStream::new(),
        cast_ordering: OrderingStream::new(),
        stream,
    })
}
fn pair(peer: u8, budget: &HostBudget) -> Result<Pair> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let socket = TcpStream::connect(listener.local_addr()?)?;
    socket.set_nodelay(true)?;
    let (accepted, _) = listener.accept()?;
    accepted.set_nodelay(true)?;
    let sender = StreamTransport::new(
        socket,
        RouteId::from_bytes([20; 32]),
        Duration::from_secs(5),
    )?;
    let receiver = StreamTransport::new(
        accepted,
        RouteId::from_bytes([30; 32]),
        Duration::from_secs(5),
    )?;
    let join = std::thread::spawn(move || {
        let result = receiver.handshake(HandshakeRole::Acceptor, &CancellationToken::default());
        (receiver, result)
    });
    sender.handshake(HandshakeRole::Initiator, &CancellationToken::default())?;
    let (receiver, accepted) = join.join().map_err(|_| "handshake thread panicked")?;
    accepted?;
    Ok(Pair {
        caller: endpoint(peer, true, budget, sender)?,
        receiver: endpoint(peer, false, budget, receiver)?,
        reply_to: fixture::reply_to(peer),
    })
}
fn receive(endpoint: &Endpoint) -> Result<Vec<u8>> {
    Ok(endpoint.stream.receive(
        &CancellationToken::default(),
        Instant::now() + Duration::from_secs(5),
    )?)
}
fn send_ack(
    receipt: &PreparedReceipt<'_>,
    endpoint: &Endpoint,
    broker: &BrokerContext,
) -> Result<()> {
    let cancel = CancellationToken::default();
    let (permit, now) = broker
        .with_time(|_, now| -> Result<_> { Ok((receipt.reserve_send(&cancel, now)?, now)) })?;
    permit.send(&endpoint.stream, &cancel, now)?;
    Ok(())
}
fn operation(
    pair: &mut Pair,
    broker: &BrokerContext,
    stable: bool,
    call: bool,
    payload: &[u8],
    progress: &mut Progress,
) -> Result<()> {
    let started = Instant::now();
    let caller = &mut pair.caller;
    let receiver = &mut pair.receiver;
    let outgoing = if call {
        &caller.outgoing
    } else {
        &caller.cast_outgoing
    };
    let incoming = if call {
        &receiver.incoming
    } else {
        &receiver.cast_incoming
    };
    progress.stage = "prepare_and_register";
    let (prepared, send, call_handle, now) = broker.with_time(|_, now| -> Result<_> {
        caller.service.expire(now)?;
        receiver.service.expire(now)?;
        caller.tracker.expire(now)?;
        receiver.tracker.expire(now)?;
        let options = if stable {
            let ordering = if call {
                &mut caller.ordering
            } else {
                &mut caller.cast_ordering
            };
            ordering.next_for_binding(
                outgoing,
                fixture::content_type(),
                now,
                now.unix_ms + 1000,
                POLICY,
            )?
        } else {
            MessageOptions::fresh_for_binding(
                outgoing,
                fixture::content_type(),
                now,
                now.unix_ms + 1000,
                POLICY,
            )?
        };
        let mut options = options;
        if call {
            options.reply_to = Some(pair.reply_to);
        }
        let prepared = service_sender::prepare(outgoing, payload, &options, now)?;
        let encoded = encode_envelope(prepared.envelope())?;
        let handle = if call {
            Some(caller.service.register_call(outgoing, &encoded, now)?)
        } else {
            None
        };
        let send = caller.tracker.register_send(
            outgoing,
            &caller.control,
            &fixture::receipt_key(true).verifying_key(),
            &encoded,
            now,
        )?;
        Ok((prepared, send, handle, now))
    })?;
    progress.stage = "send_request";
    outgoing.send_prepared(
        &prepared,
        &caller.stream,
        &CancellationToken::default(),
        now,
    )?;
    let original = receive(receiver)?;
    progress.stage = "receiver_admission";
    let (admitted, ack, _now) = broker.with_time(|_, now| -> Result<_> {
        let (admitted, ack) =
            receiver
                .returns
                .admit(&mut receiver.service, incoming, &original, now)?;
        Ok((admitted, ack, now))
    })?;
    if admitted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
        return Err("unexpected admission outcome".into());
    }
    progress.admitted = true;
    send_ack(&ack, receiver, broker)?;
    progress.stage = "caller_observe_admission";
    let receipt = receive(caller)?;
    broker.with_time(|_, now| -> Result<_> {
        caller
            .tracker
            .observe_receipt(&caller.control, &receipt, now)?;
        let ReceiptObservation::Verified(receipt) =
            caller.tracker.take_receipt(outgoing, &send, now)?
        else {
            return Err("missing admission receipt".into());
        };
        if receipt.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
            return Err("negative admission receipt".into());
        }
        Ok(())
    })?;
    progress.admission_us = Some(started.elapsed().as_micros() as u64);
    progress.stage = "receiver_payload";
    let proof = broker.with_time(|_, now| -> Result<_> {
        Ok(receiver
            .service
            .receive(incoming, now)?
            .ok_or("missing admitted payload")?)
    })?;
    if proof.payload() != payload {
        return Err("receiver exact payload mismatch".into());
    }
    progress.exact += 1;
    progress.receiver_exact = true;
    if !call {
        return Ok(());
    }
    progress.stage = "prepare_reply";
    let (reply, reply_send, now) = broker.with_time(|_, now| -> Result<_> {
        let options = if stable {
            receiver
                .ordering
                .next_reply(&receiver.outgoing, &proof, now, POLICY)?
        } else {
            MessageOptions::for_reply_with_policy(&receiver.outgoing, &proof, now, POLICY)?
        };
        let reply = receiver.service.reply_prepared(
            &receiver.outgoing,
            proof.handle(),
            service_sender::request(&receiver.outgoing, payload, &options, now)?,
            now,
        )?;
        let encoded = encode_envelope(reply.envelope())?;
        let send = receiver.tracker.register_send(
            &receiver.outgoing,
            &receiver.control,
            &fixture::receipt_key(false).verifying_key(),
            &encoded,
            now,
        )?;
        Ok((reply, send, now))
    })?;
    receiver.outgoing.send_prepared(
        &reply,
        &receiver.stream,
        &CancellationToken::default(),
        now,
    )?;
    progress.stage = "caller_reply_admission";
    let reply = receive(caller)?;
    let (ack, _now) = broker.with_time(|_, now| -> Result<_> {
        let (admitted, ack) =
            caller
                .returns
                .admit(&mut caller.service, &caller.incoming, &reply, now)?;
        if admitted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
            return Err("Reply admission refused".into());
        }
        caller
            .tracker
            .observe_application_reply(&caller.incoming, &reply, now)?;
        let CallStatus::Replied(Some(echo)) =
            caller
                .service
                .poll_call(outgoing, &call_handle.ok_or("missing call handle")?, now)?
        else {
            return Err("missing application Reply".into());
        };
        if echo != payload {
            return Err("caller exact payload mismatch".into());
        }
        progress.exact += 1;
        progress.reply_us = Some(started.elapsed().as_micros() as u64);
        Ok((ack, now))
    })?;
    send_ack(&ack, caller, broker)?;
    progress.stage = "receiver_observe_reply_receipt";
    let receipt = receive(receiver)?;
    broker.with_time(|_, now| -> Result<_> {
        receiver
            .tracker
            .observe_receipt(&receiver.control, &receipt, now)?;
        let ReceiptObservation::Verified(ack) =
            receiver
                .tracker
                .take_receipt(&receiver.outgoing, &reply_send, now)?
        else {
            return Err("missing Reply receipt".into());
        };
        if !receiver.service.acknowledge_reply(
            &receiver.outgoing,
            &ack,
            &fixture::receipt_key(false).verifying_key(),
            now,
        )? {
            return Err("Reply receipt did not release reply".into());
        }
        Ok(())
    })?;
    Ok(())
}
fn quantiles(values: &mut [u64]) -> Value {
    values.sort_unstable();
    let get = |percent: usize| {
        values
            .get((values.len() * percent / 100).min(values.len().saturating_sub(1)))
            .copied()
    };
    json!({"p50_us":get(50),"p95_us":get(95),"p99_us":get(99),"samples":values.len()})
}
fn rss_bytes() -> Option<u64> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
        // SAFETY: getrusage writes the valid allocated rusage structure; only
        // read initialized data after a successful call. RUSAGE_SELF is local.
        let usage = unsafe {
            if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) != 0 {
                return None;
            }
            usage.assume_init()
        };
        let value = u64::try_from(usage.ru_maxrss).ok()?;
        if cfg!(target_os = "macos") {
            Some(value)
        } else {
            value.checked_mul(1024)
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}
fn fd_count() -> Option<usize> {
    #[cfg(target_os = "linux")]
    let path = "/proc/self/fd";
    #[cfg(target_os = "macos")]
    let path = "/dev/fd";
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        Some(std::fs::read_dir(path).ok()?.count())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}
/// Keep teardown diagnostics separate from already completed payload checks.
/// A peer may already have shut down its half by the time local shutdown runs.
/// Unexpected errors are retained too; neither class erases a measurement row.
pub fn record_close_result(
    errors: &mut Vec<Value>,
    pair: usize,
    endpoint: &str,
    result: spiralcore_mvm_adapter::live::LiveResult<()>,
) {
    if let Err(error) = result {
        let (classification, os_error) = match &error {
            spiralcore_mvm_adapter::live::LiveError::Io(io)
                if io.kind() == std::io::ErrorKind::NotConnected =>
            {
                ("peer_already_disconnected", io.raw_os_error())
            }
            spiralcore_mvm_adapter::live::LiveError::Io(io) => {
                ("unexpected_teardown_error", io.raw_os_error())
            }
            _ => ("unexpected_teardown_error", None),
        };
        errors.push(json!({"pair":pair,"endpoint":endpoint,"classification":classification,"os_error":os_error,"error":format!("{error:?}")}));
    }
}
/// Run one bounded cell. `duration_ms==0` disables pacing for unit/saturation runs.
pub fn run_cell(
    endpoints: usize,
    stable: bool,
    messages: usize,
    duration_ms: u64,
) -> Result<Value> {
    if ![2, 8, 32].contains(&endpoints)
        || messages == 0
        || messages > MAX_OPERATIONS
        || duration_ms > 120_000
    {
        return Err("outside registered capacity probe limits".into());
    }
    let setup = Instant::now();
    let setup_cpu = ProcessTime::now();
    let (l, r) = limits();
    let budget = HostBudget::with_control(l, r)?;
    let broker = BrokerContext::new(budget.clone());
    let mut pairs = (0..endpoints / 2)
        .map(|p| pair(p as u8, &budget))
        .collect::<Result<Vec<_>>>()?;
    let setup_wall_us = setup.elapsed().as_micros() as u64;
    let setup_cpu_us = setup_cpu.elapsed().as_micros() as u64;
    let mut payload = vec![0; 4096];
    for (i, b) in payload.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(73);
    }
    payload[4095] = 0;
    let rss_start = rss_bytes();
    let mut sampled_peak = rss_start;
    let fd_start = fd_count();
    let mut fd_sampled_peak = fd_start;
    let mut latencies = Vec::with_capacity(messages);
    let mut admission_latencies = Vec::with_capacity(messages);
    let mut reply_latencies = Vec::with_capacity(messages);
    let mut receiver_exact = 0;
    let mut timeline = Vec::with_capacity(128);
    let mut next_sample = 0;
    let mut calls = 0usize;
    let mut casts = 0usize;
    let mut completed_calls = 0;
    let mut completed_casts = 0;
    let mut exact = 0usize;
    let mut refusal = None;
    let mut fidelity_failures = 0;
    let mut attempted = 0;
    let mut error_stage = None;
    let mut stop_reason = "operation_limit";
    let mut budget_high = 0;
    let mut scopes_high = 0;
    let mut entries_high = 0;
    let start = Instant::now();
    let cpu = ProcessTime::now();
    for index in 0..messages {
        if start.elapsed() >= Duration::from_secs(120) {
            stop_reason = "hard_120_second_limit";
            break;
        }
        if duration_ms > 0 && start.elapsed() >= Duration::from_millis(duration_ms) {
            stop_reason = "requested_duration";
            break;
        }
        let operation_start = Instant::now();
        let pair_count = pairs.len();
        let is_call = (index / pair_count) % 2 == 0;
        payload[..8].copy_from_slice(&(index as u64).to_le_bytes());
        let mut progress = Progress::default();
        attempted += 1;
        let outcome = operation(
            &mut pairs[index % pair_count],
            &broker,
            stable,
            is_call,
            &payload,
            &mut progress,
        );
        exact += progress.exact;
        receiver_exact += usize::from(progress.receiver_exact);
        if let Some(value) = progress.admission_us {
            admission_latencies.push(value);
        }
        if let Some(value) = progress.reply_us {
            reply_latencies.push(value);
        }
        if progress.admitted {
            if is_call { calls += 1 } else { casts += 1 }
        }
        match outcome {
            Ok(()) => {
                if is_call {
                    completed_calls += 1
                } else {
                    completed_casts += 1
                }
            }
            Err(error) => {
                let error = error.to_string();
                if error.contains("payload mismatch") {
                    fidelity_failures += 1;
                }
                refusal = Some(error);
                error_stage = Some(progress.stage);
                stop_reason = "first_error";
                break;
            }
        }
        latencies.push(operation_start.elapsed().as_micros() as u64);
        let snapshot = budget.snapshot()?;
        budget_high = budget_high.max(snapshot.identities);
        let mut scopes = 0;
        let mut entries = 0;
        for pair in &pairs {
            for endpoint in [&pair.caller, &pair.receiver] {
                let snapshot = endpoint.service.snapshot();
                scopes += snapshot.fifo_scopes;
                entries += snapshot.fifo_entries;
            }
        }
        scopes_high = scopes_high.max(scopes);
        entries_high = entries_high.max(entries);
        if start.elapsed().as_secs() >= next_sample && timeline.len() < 128 {
            timeline.push(json!({"elapsed_ms":start.elapsed().as_millis() as u64,"identities":snapshot.identities,"queued_bytes":snapshot.queued_bytes,"queued_messages":snapshot.queued_messages,"active_calls":snapshot.active_calls,"fifo_scopes":scopes,"fifo_entries":entries,"completed_operations":completed_calls+completed_casts,"process_cpu_us":cpu.elapsed().as_micros() as u64}));
            next_sample = start.elapsed().as_secs() + 1;
        }
        if index % 20 == 0 {
            sampled_peak = sampled_peak.into_iter().chain(rss_bytes()).max();
            fd_sampled_peak = fd_sampled_peak.into_iter().chain(fd_count()).max();
            if sampled_peak.is_some_and(|peak| peak > 512 * 1024 * 1024)
                || fd_sampled_peak.is_some_and(|fds| fds > 128)
            {
                refusal = Some("registered RSS or FD ceiling reached".into());
                error_stage = Some("resource_sampler");
                stop_reason = "resource_limit";
                break;
            }
        }
        if duration_ms > 0 {
            let target = Duration::from_millis(((index + 1) as u64) * 50);
            if let Some(wait) = target.checked_sub(start.elapsed()) {
                std::thread::sleep(wait);
            }
        }
    }
    let elapsed_us = start.elapsed().as_micros() as u64;
    let cpu_us = cpu.elapsed().as_micros() as u64;
    let mut forward = 0;
    let mut reverse = 0;
    let mut frames = 0;
    let mut agreement = true;
    let mut close_errors = Vec::new();
    for (index, pair) in pairs.iter().enumerate() {
        let a = pair.caller.stream.counters()?;
        let b = pair.receiver.stream.counters()?;
        forward += a.envelope_bytes_sent + a.framing_bytes_sent;
        reverse += b.envelope_bytes_sent + b.framing_bytes_sent;
        frames += a.frames_sent + b.frames_sent;
        agreement &= a.envelope_bytes_sent == b.envelope_bytes_received
            && b.envelope_bytes_sent == a.envelope_bytes_received
            && a.framing_bytes_sent == b.framing_bytes_received
            && b.framing_bytes_sent == a.framing_bytes_received;
        record_close_result(
            &mut close_errors,
            index,
            "caller",
            pair.caller.stream.close(),
        );
        record_close_result(
            &mut close_errors,
            index,
            "receiver",
            pair.receiver.stream.close(),
        );
    }
    let before_cleanup = budget.snapshot()?;
    let cleanup_start = Instant::now();
    let cleanup_wait_ms = if refusal.is_some() { 2100 } else { 0 };
    if cleanup_wait_ms > 0 {
        std::thread::sleep(Duration::from_millis(cleanup_wait_ms));
    }
    broker.with_time(|_, now| -> Result<_> {
        for pair in &mut pairs {
            for endpoint in [&mut pair.caller, &mut pair.receiver] {
                endpoint.service.expire(now)?;
                endpoint.tracker.expire(now)?;
            }
        }
        Ok(())
    })?;
    let cleanup_wall_us = cleanup_start.elapsed().as_micros() as u64;
    let snapshot = budget.snapshot()?;
    let mut row = json!({
        "scope":"authenticated TCP loopback logical endpoints; not WAN/native MVM/fleet capacity",
        "configured_endpoints":endpoints,"tcp_pairs":endpoints/2,
        "ordering":if stable{"stable"}else{"fresh"},"pipeline_per_pair":1,
        "global_dispatch":"serial round robin","requested_operations":messages,
        "requested_duration_ms":duration_ms,
        "pacing_total_operations_per_second":if duration_ms>0{Some(20)}else{None},
        "setup_wall_us":setup_wall_us,"setup_process_cpu_us":setup_cpu_us,
        "elapsed_us":elapsed_us,"process_cpu_us":cpu_us,
        "accepted_calls":calls,"accepted_casts":casts,"exact_readbacks":exact,
        "fidelity_failures":fidelity_failures,"first_error":refusal,
        "first_error_stage":error_stage,"stop_reason":stop_reason
    });
    let counters = json!({
        "frames_sent":frames,"actual_caller_to_receiver_bytes":forward,
        "actual_receiver_to_caller_bytes":reverse,"actual_both_direction_bytes":forward+reverse,
        "byte_counter_scope":"canonical encrypted envelopes plus framing; excludes TCP/IP headers and kernel retransmission",
        "sender_receiver_counters_match":agreement,"latency":quantiles(&mut latencies),
        "admission_latency":quantiles(&mut admission_latencies),
        "application_reply_latency":quantiles(&mut reply_latencies),
        "process_peak_rss_at_start_bytes":rss_start,"sampled_process_peak_rss_bytes":sampled_peak,
        "rss_scope":"getrusage process lifetime high-water; CLI runs each cell in a fresh process",
        "measurement_process_id":std::process::id(),
        "rss_sample_interval_operations":20,"post_operation_sampled_identity_max":budget_high,
        "post_operation_sampled_fifo_scope_max":scopes_high,"post_operation_sampled_fifo_entry_max":entries_high,
        "final_identities":snapshot.identities,"attempted_operations":attempted,
        "completed_calls":completed_calls,"completed_casts":completed_casts
    });
    row.as_object_mut()
        .unwrap()
        .extend(counters.as_object().unwrap().clone());
    row["unexpected_teardown_errors"] = json!(
        close_errors
            .iter()
            .filter(|e| e["classification"] == "unexpected_teardown_error")
            .count()
    );
    row["close_errors"] = json!(close_errors);
    row["teardown_scope"] = json!(
        "socket shutdown after transfer accounting; close diagnostics do not establish payload loss or erase unexpected cleanup failures"
    );
    row["kernel_reported_peak_process_rss_bytes"] = json!(rss_bytes());
    row["fd_sampled_at_start"] = json!(fd_start);
    row["fd_sampled_max"] = json!(fd_sampled_peak);
    row["fd_count_scope"] = json!(
        "local process descriptors; enumeration may include its own temporary directory descriptor"
    );
    row["receiver_verified_requests"] = json!(receiver_exact);
    row["unverified_admitted_requests"] = json!(calls + casts - receiver_exact);
    row["incomplete_admitted_requests"] = json!(calls + casts - completed_calls - completed_casts);
    row["fidelity_scope"] =
        json!("compared readbacks only; unverified admitted requests are not successful delivery");
    row["post_operation_samples"] = json!(timeline);
    row["post_ttl_cleanup"] = json!({"wait_only_after_refusal_ms":cleanup_wait_ms,"wall_us":cleanup_wall_us,"before_active_calls":before_cleanup.active_calls,"after_active_calls":snapshot.active_calls,"before_queued_messages":before_cleanup.queued_messages,"after_queued_messages":snapshot.queued_messages,"service_reset":false});
    row["profile"] = json!({"name":"capacity-native-identities-retention1s-v1",
        "retention_ms":1000,"host_identities":65536,"tenant_identities":32768,
        "mailbox_identities":32768,"sender_identities":16384,"payload_bytes":4096,
        "message_ttl_ms":1000,"max_operations":MAX_OPERATIONS,"max_duration_ms":120000,"rss_ceiling_bytes":536870912,"fd_ceiling":128});
    for (name, cap) in [
        ("host", snapshot.limits.host),
        ("tenant", snapshot.limits.tenant),
        ("mailbox", snapshot.limits.mailbox),
        ("sender", snapshot.limits.sender),
    ] {
        row["profile"]["actual_limits"][name] = json!({"queued_bytes":cap.queued_bytes,"queued_messages":cap.queued_messages,"active_calls":cap.active_calls,"identities":cap.identities});
    }
    if let Some(reserve) = snapshot.control {
        for (name, cap) in [
            ("host", reserve.host),
            ("tenant", reserve.tenant),
            ("mailbox", reserve.mailbox),
            ("sender", reserve.sender),
        ] {
            row["profile"]["actual_control_reserve"][name] = json!({"queued_bytes":cap.queued_bytes,"queued_messages":cap.queued_messages,"identities":cap.identities});
        }
    }
    row["durability"] =
        json!("AcceptedEphemeral exact application echo; no durable object publication claim");
    Ok(row)
}
#[allow(dead_code)] // The integration test includes this example as a module.
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|a| a == "--cell") {
        if args.len() != 4 {
            return Err("usage: service_capacity --cell ENDPOINTS fresh|stable DURATION_MS".into());
        }
        let stable = match args[2].as_str() {
            "fresh" => false,
            "stable" => true,
            _ => return Err("unknown ordering mode".into()),
        };
        let row = run_cell(args[1].parse()?, stable, MAX_OPERATIONS, args[3].parse()?)?;
        serde_json::to_writer(std::io::stdout().lock(), &row)?;
        return Ok(());
    }
    if args.len() != 2 {
        return Err(
            "usage: service_capacity NEW_OUTPUT_JSONL DURATION_MS (0..120000; full run65000)"
                .into(),
        );
    }
    let duration = args[1].parse::<u64>()?;
    if duration > 120_000 {
        return Err("duration exceeds registration".into());
    }
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[0])?;
    for endpoints in [2, 8, 32] {
        for mode in ["fresh", "stable"] {
            let child = std::process::Command::new(std::env::current_exe()?)
                .args([
                    "--cell",
                    &endpoints.to_string(),
                    mode,
                    &duration.to_string(),
                ])
                .output()?;
            if !child.status.success() {
                return Err(format!(
                    "capacity child failed: {}",
                    String::from_utf8_lossy(&child.stderr)
                )
                .into());
            }
            if child.stdout.len() > 1_048_576 {
                return Err("capacity record exceeds bound".into());
            }
            let row: Value = serde_json::from_slice(&child.stdout)?;
            serde_json::to_writer(&mut out, &row)?;
            writeln!(out)?;
            out.sync_all()?;
        }
    }
    Ok(())
}
