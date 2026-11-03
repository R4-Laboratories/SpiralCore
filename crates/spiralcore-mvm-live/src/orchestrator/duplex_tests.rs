use super::*;
use crate::config::Arm;
use mvm_contract::protocol::mailbox::ContentType;
use mvm_core::mailbox::encode_envelope;
use spiralcore_mvm_adapter::{service::ServiceTime, service_sender};

// A deadlock watchdog, not a latency/performance threshold. All failure paths
// shut down the endpoints and join the relay before asserting their result.
const WATCHDOG: Duration = Duration::from_secs(3);

fn sealed_frame(material: &Material, payload: &[u8]) -> Vec<u8> {
    let binding = material.service_binding(None).unwrap();
    let now = ServiceTime {
        unix_ms: now_ms().unwrap(),
        monotonic_ms: 0,
    };
    let mut options = service_sender::MessageOptions::fresh(
        ContentType::parse(spiralcore_mvm_adapter::CONTENT_TYPE, 1).unwrap(),
        now,
        now.unix_ms + 30_000,
    )
    .unwrap();
    options.reply_to = material.reply_to;
    let prepared = service_sender::prepare(&binding, payload, &options, now).unwrap();
    let wire = encode_envelope(prepared.envelope()).unwrap();
    // Qualify the fixture against the same outer admission before starting any
    // worker, so a wrong direction/key cannot masquerade as blocked progress.
    let envelope = decode_envelope(&wire).unwrap();
    RouteAdmission::new(material)
        .validate(&envelope.routing, &envelope.encryption.key_id)
        .unwrap();
    wire
}

fn independent_direction_progress(reverse_first: bool) {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let forward_material = pair.sender.outgoing.as_ref().unwrap();
    let reverse_material = pair.receiver.outgoing.as_ref().unwrap();
    let forward = RouteAdmission::new(forward_material);
    let reverse = RouteAdmission::new(reverse_material);
    let first = sealed_frame(forward_material, b"first tiny Call");
    let second = sealed_frame(forward_material, b"second tiny Call");
    let reverse_wire = sealed_frame(reverse_material, b"tiny reverse ciphertext");
    let (mut a, relay_a) = UnixStream::pair().unwrap();
    let (mut b, relay_b) = UnixStream::pair().unwrap();
    let owner = transport_budget().unwrap();
    let relay_owner = owner.clone();
    let worker = std::thread::spawn(move || -> Result<Value> {
        let scope = relay_owner.scope(0)?;
        let links = [
            relay_owner.reserve_link(&scope)?,
            relay_owner.reserve_link(&scope)?,
        ];
        relay(relay_a, relay_b, forward, reverse, [&links[0], &links[1]])
    });
    let cancel = CancellationToken::default();
    let mut ac = StreamCounters::default();
    let mut bc = StreamCounters::default();
    let progress = (|| -> Result<()> {
        // The relay accepts A, then B. Both readiness handshakes finish before
        // selecting which application direction is intentionally withheld.
        live::handshake_stream(
            &mut a,
            HandshakeRole::Initiator,
            &cancel,
            Instant::now() + WATCHDOG,
            &mut ac,
        )?;
        live::handshake_stream(
            &mut b,
            HandshakeRole::Initiator,
            &cancel,
            Instant::now() + WATCHDOG,
            &mut bc,
        )?;
        if reverse_first {
            // A has sent no forward application frame, including no Call.
            live::write_ciphertext_frame(
                &mut b,
                &reverse_wire,
                &cancel,
                Instant::now() + WATCHDOG,
                &mut bc,
            )?;
            let actual =
                live::read_ciphertext_frame(&mut a, &cancel, Instant::now() + WATCHDOG, &mut ac)?;
            if actual != reverse_wire {
                return Err("reverse frame changed in relay".into());
            }
            if ac.frames_sent != 0 || ac.frames_received != 1 {
                return Err("reverse progress required a forward application frame".into());
            }
        } else {
            // B sends no reverse application event. Both Calls must arrive
            // byte-for-byte before that withheld direction is released.
            for wire in [&first, &second] {
                live::write_ciphertext_frame(
                    &mut a,
                    wire,
                    &cancel,
                    Instant::now() + WATCHDOG,
                    &mut ac,
                )?;
            }
            for expected in [&first, &second] {
                let actual = live::read_ciphertext_frame(
                    &mut b,
                    &cancel,
                    Instant::now() + WATCHDOG,
                    &mut bc,
                )?;
                if actual != *expected {
                    return Err("forward frame changed in relay".into());
                }
            }
            if bc.frames_sent != 0 || bc.frames_received != 2 {
                return Err("forward progress required a reverse application event".into());
            }
        }
        Ok(())
    })();
    // Wake either old choreography or both future direction pumps, including
    // when handshake/read/write failed. A worker is never left detached.
    let _ = a.shutdown(Shutdown::Both);
    let _ = b.shutdown(Shutdown::Both);
    let relay_result = worker.join().expect("relay worker panicked");
    let snapshot = owner.snapshot().unwrap();
    assert_eq!(snapshot.usage.links, 0);
    assert_eq!(snapshot.usage.frame_slots, 0);
    assert_eq!(snapshot.usage.serialized_buffer_bytes, 0);
    assert!(
        progress.is_ok(),
        "independent direction did not progress: {progress:?}; relay: {relay_result:?}; A: {ac:?}; B: {bc:?}"
    );
}

#[test]
fn reverse_ciphertext_progresses_before_any_forward_call() {
    independent_direction_progress(true);
}

#[test]
fn two_forward_ciphertexts_progress_while_reverse_is_withheld() {
    independent_direction_progress(false);
}

struct RunningRelay {
    a: UnixStream,
    b: UnixStream,
    ac: StreamCounters,
    bc: StreamCounters,
    forward_wire: Vec<u8>,
    reverse_wire: Vec<u8>,
    done: std::sync::mpsc::Receiver<()>,
    worker: Option<std::thread::JoinHandle<Result<Value>>>,
}

impl RunningRelay {
    fn start(owner: &HostTransportBudget) -> Self {
        Self::start_with_output_buffer(owner, None)
    }

    fn start_with_output_buffer(owner: &HostTransportBudget, send_buffer: Option<i32>) -> Self {
        let pair = Pair::provision(Plan {
            stage: Stage::Two,
            arm: Arm::Raw,
            size: 128,
            trial: 0,
            bandwidth_profile: Default::default(),
        })
        .unwrap();
        let forward = RouteAdmission::new(pair.sender.outgoing.as_ref().unwrap());
        let reverse = RouteAdmission::new(pair.receiver.outgoing.as_ref().unwrap());
        let forward_wire = if send_buffer.is_some() {
            sealed_frame(pair.sender.outgoing.as_ref().unwrap(), &[7; 32 << 10])
        } else {
            Vec::new()
        };
        let reverse_wire = sealed_frame(
            pair.receiver.outgoing.as_ref().unwrap(),
            b"buffered reverse ciphertext",
        );
        let (a, relay_a) = UnixStream::pair().unwrap();
        let (b, relay_b) = UnixStream::pair().unwrap();
        if let Some(send_buffer) = send_buffer {
            // SAFETY: this is our live relay-B socket and a correctly sized
            // integer socket option. No unrelated descriptor is modified.
            let status = unsafe {
                libc::setsockopt(
                    relay_b.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_SNDBUF,
                    (&send_buffer as *const i32).cast(),
                    std::mem::size_of_val(&send_buffer) as libc::socklen_t,
                )
            };
            assert_eq!(status, 0, "{}", io::Error::last_os_error());
        }
        let owner = owner.clone();
        let (done_tx, done) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || -> Result<Value> {
            let result = (|| {
                let scope = owner.scope(0)?;
                let links = [owner.reserve_link(&scope)?, owner.reserve_link(&scope)?];
                relay(relay_a, relay_b, forward, reverse, [&links[0], &links[1]])
            })();
            let _ = done_tx.send(());
            result
        });
        Self {
            a,
            b,
            ac: StreamCounters::default(),
            bc: StreamCounters::default(),
            forward_wire,
            reverse_wire,
            done,
            worker: Some(worker),
        }
    }

    fn ready(&mut self) -> Result<()> {
        for (socket, counters) in [(&mut self.a, &mut self.ac), (&mut self.b, &mut self.bc)] {
            live::handshake_stream(
                socket,
                HandshakeRole::Initiator,
                &CancellationToken::default(),
                Instant::now() + WATCHDOG,
                counters,
            )?;
        }
        Ok(())
    }

    fn stop(&self) {
        let _ = self.a.shutdown(Shutdown::Both);
        let _ = self.b.shutdown(Shutdown::Both);
    }

    fn finish(mut self) -> (Result<Value>, bool) {
        // Observe autonomous completion before test-side shutdown. On failure,
        // still wake/join the worker before reporting a watchdog failure.
        let completed = self.done.recv_timeout(WATCHDOG).is_ok();
        self.stop();
        let result = self.worker.take().unwrap().join().expect("relay panicked");
        (result, completed)
    }
}

impl Drop for RunningRelay {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn assert_released(owner: &HostTransportBudget) {
    let snapshot = owner.snapshot().unwrap();
    assert_eq!(snapshot.usage.links, 0);
    assert_eq!(snapshot.usage.frame_slots, 0);
    assert_eq!(snapshot.usage.serialized_buffer_bytes, 0);
}

#[test]
fn clean_half_close_drains_buffered_reverse_frame_and_matches_exact_counters() {
    let owner = transport_budget().unwrap();
    let mut running = RunningRelay::start(&owner);
    running.ready().unwrap();
    let cancel = CancellationToken::default();
    live::write_ciphertext_frame(
        &mut running.b,
        &running.reverse_wire,
        &cancel,
        Instant::now() + WATCHDOG,
        &mut running.bc,
    )
    .unwrap();
    running.a.shutdown(Shutdown::Write).unwrap();
    // First observe propagation of A's clean input EOF at B. Reverse bytes
    // already queued by B must survive that half-close until A drains them.
    assert!(
        live::read_ciphertext_frame_or_eof(
            &mut running.b,
            &cancel,
            Instant::now() + WATCHDOG,
            &mut running.bc,
        )
        .unwrap()
        .is_none()
    );
    let reverse = live::read_ciphertext_frame(
        &mut running.a,
        &cancel,
        Instant::now() + WATCHDOG,
        &mut running.ac,
    )
    .unwrap();
    assert_eq!(reverse, running.reverse_wire);
    running.b.shutdown(Shutdown::Write).unwrap();
    assert!(
        live::read_ciphertext_frame_or_eof(
            &mut running.a,
            &cancel,
            Instant::now() + WATCHDOG,
            &mut running.ac,
        )
        .unwrap()
        .is_none()
    );
    let ac = serde_json::to_value(running.ac).unwrap();
    let bc = serde_json::to_value(running.bc).unwrap();
    let (result, completed) = running.finish();
    assert!(
        completed,
        "half-close did not autonomously finish both pumps"
    );
    let result = result.unwrap();
    counters_agree(&ac, &result["a"]).unwrap();
    counters_agree(&bc, &result["b"]).unwrap();
    assert_eq!(ac["frames_sent"], 0);
    assert_eq!(ac["frames_received"], 1);
    assert_eq!(bc["frames_sent"], 1);
    assert_eq!(bc["frames_received"], 0);
    assert_released(&owner);
    let snapshot = owner.snapshot().unwrap();
    assert_eq!(snapshot.high_water.frame_slots, 2);
    assert_eq!(snapshot.high_water.serialized_buffer_bytes, 524288);
}

#[test]
fn partial_prefix_and_payload_eof_fail_and_wake_both_pumps_with_partial_counters() {
    for (prefix, payload) in [(1, 0), (2, 0), (3, 0), (4, 3)] {
        let owner = transport_budget().unwrap();
        let mut running = RunningRelay::start(&owner);
        running.ready().unwrap();
        running.a.write_all(&16u32.to_be_bytes()[..prefix]).unwrap();
        running.a.write_all(&[7; 3][..payload]).unwrap();
        running.a.shutdown(Shutdown::Write).unwrap();
        // B remains open and idle. Only fatal framing cancellation can wake its
        // pump before the test watchdog; no test-side close precedes this wait.
        let (result, completed) = running.finish();
        assert!(
            completed,
            "fatal framing did not wake sibling: {prefix}/{payload}"
        );
        let error = result.unwrap_err().to_string();
        let diagnostic: Value = serde_json::from_str(
            error
                .strip_prefix("relay failed: ")
                .expect("missing pump diagnostic"),
        )
        .unwrap();
        assert!(!diagnostic["forward"].is_null(), "{error}");
        assert_eq!(diagnostic["a"]["framing_bytes_received"], 8 + prefix);
        assert_eq!(diagnostic["a"]["envelope_bytes_received"], payload);
        assert_eq!(diagnostic["a"]["frames_received"], 0);
        assert_eq!(diagnostic["b"]["framing_bytes_received"], 8);
        assert_eq!(diagnostic["b"]["envelope_bytes_received"], 0);
        assert_eq!(diagnostic["b"]["frames_received"], 0);
        assert_eq!(diagnostic["b"]["frames_sent"], 0);
        assert_released(&owner);
    }
}

#[test]
fn second_direction_credit_refusal_rolls_back_before_handshake_or_read() {
    let limits = TransportLimits {
        links: 2,
        frame_slots: 1,
        serialized_buffer_bytes: 262144,
    };
    let owner = HostTransportBudget::new(TransportBudgetConfig {
        host: limits,
        tenant: limits,
        max_tenants: 1,
        max_frame_bytes: 262144,
    })
    .unwrap();
    let running = RunningRelay::start(&owner);
    // Neither peer sends even a readiness byte. Reserving the reverse frame
    // must refuse before the relay starts either handshake or direction pump.
    let (result, completed) = running.finish();
    assert!(completed, "credit refusal blocked on handshake/read");
    assert!(result.unwrap_err().to_string().contains("Full"));
    assert_released(&owner);
    let snapshot = owner.snapshot().unwrap();
    assert_eq!(snapshot.high_water.links, 2);
    assert_eq!(snapshot.high_water.frame_slots, 1);
    assert_eq!(snapshot.high_water.serialized_buffer_bytes, 262144);
    assert_eq!(snapshot.refusals, 1);
}

#[test]
fn blocked_forward_write_failure_wakes_idle_sibling_and_retains_partial_sent_bytes() {
    let owner = transport_budget().unwrap();
    let mut running = RunningRelay::start_with_output_buffer(&owner, Some(4096));
    running.ready().unwrap();
    let wire_len = running.forward_wire.len();
    assert!(wire_len > 8192);
    live::write_ciphertext_frame(
        &mut running.a,
        &running.forward_wire,
        &CancellationToken::default(),
        Instant::now() + WATCHDOG,
        &mut running.ac,
    )
    .unwrap();
    // Consume only the prefix. Its arrival proves the relay began this write;
    // the small owned send buffer cannot hold the complete encrypted payload.
    running.b.set_read_timeout(Some(WATCHDOG)).unwrap();
    let mut prefix = [0; 4];
    running.b.read_exact(&mut prefix).unwrap();
    assert_eq!(u32::from_be_bytes(prefix) as usize, wire_len);
    let mut readable = libc::pollfd {
        fd: running.b.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // Wait for queued payload without consuming it. This removes the race in
    // which closing immediately after the prefix precedes the first body write.
    // SAFETY: one valid, uniquely borrowed pollfd; the socket remains owned.
    let ready = unsafe { libc::poll(&mut readable, 1, WATCHDOG.as_millis() as i32) };
    assert_eq!(ready, 1, "payload never reached the blocked output");
    assert_ne!(readable.revents & libc::POLLIN, 0);
    running.b.shutdown(Shutdown::Both).unwrap();
    // A stays open. The fatal forward write must wake its idle sibling before
    // the harness supplies any cleanup shutdown. This is no 30s timeout claim.
    let (result, completed) = running.finish();
    assert!(completed, "blocked write failure did not wake idle sibling");
    let error = result.unwrap_err().to_string();
    let diagnostic: Value = serde_json::from_str(
        error
            .strip_prefix("relay failed: ")
            .expect("missing pump diagnostic"),
    )
    .unwrap();
    assert!(!diagnostic["forward"].is_null(), "{error}");
    let sent = diagnostic["b"]["envelope_bytes_sent"].as_u64().unwrap();
    assert!(sent > 0 && sent < wire_len as u64, "{error}");
    assert_eq!(diagnostic["b"]["framing_bytes_sent"], 12);
    assert_eq!(diagnostic["b"]["frames_sent"], 0);
    assert_eq!(diagnostic["a"]["frames_received"], 1);
    assert_released(&owner);
}
