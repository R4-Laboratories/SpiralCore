//! Bounded independent pumps for the existing opaque canonical transport.
use super::*;
use crate::host_budget::FrameReservation;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    thread,
};

pub(super) fn forward_frame(
    input: &mut UnixStream,
    output: &mut UnixStream,
    route: &RouteAdmission,
    _capacity: &FrameReservation,
    cancel: &CancellationToken,
    counters: (&mut StreamCounters, &mut StreamCounters),
    allow_clean_eof: bool,
) -> Result<bool> {
    let (received, sent) = counters;
    let wire = if allow_clean_eof {
        let Some(wire) = live::read_ciphertext_frame_or_eof(
            input,
            cancel,
            Instant::now() + Duration::from_secs(30),
            received,
        )?
        else {
            return Ok(false);
        };
        wire
    } else {
        live::read_ciphertext_frame(
            input,
            cancel,
            Instant::now() + Duration::from_secs(30),
            received,
        )?
    };
    let envelope = decode_envelope(&wire)?;
    route.validate(&envelope.routing, &envelope.encryption.key_id)?;
    live::write_ciphertext_frame(
        output,
        &wire,
        cancel,
        Instant::now() + Duration::from_secs(30),
        sent,
    )?;
    Ok(true)
}

fn stop(a: &UnixStream, b: &UnixStream, cancel: &CancellationToken) {
    cancel.cancel();
    let _ = a.shutdown(Shutdown::Both);
    let _ = b.shutdown(Shutdown::Both);
}

// A failed startup or unwind must also close owned streams. Started workers
// handle their own failure before their sibling's join can block.
struct Close<'a>(&'a UnixStream, &'a UnixStream, &'a CancellationToken);
impl Drop for Close<'_> {
    fn drop(&mut self) {
        stop(self.0, self.1, self.2);
    }
}

#[derive(Debug, Default)]
struct PumpReport {
    received: StreamCounters,
    sent: StreamCounters,
    failure: Option<String>,
}

fn pump(
    mut input: UnixStream,
    mut output: UnixStream,
    route: &RouteAdmission,
    capacity: FrameReservation,
    cancel: &CancellationToken,
) -> PumpReport {
    let mut report = PumpReport::default();
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
        while forward_frame(
            &mut input,
            &mut output,
            route,
            &capacity,
            cancel,
            (&mut report.received, &mut report.sent),
            true,
        )? {}
        output.shutdown(Shutdown::Write)?;
        Ok(())
    }));
    report.failure = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("direction worker panicked".into()),
    };
    if report.failure.is_some() {
        stop(&input, &output, cancel);
    }
    // No frame representation outlives forward_frame; credit ends after I/O.
    drop(capacity);
    report
}

fn add_counters(target: &mut StreamCounters, source: StreamCounters) -> Result<()> {
    for (target, value) in [
        (&mut target.envelope_bytes_sent, source.envelope_bytes_sent),
        (
            &mut target.envelope_bytes_received,
            source.envelope_bytes_received,
        ),
        (&mut target.framing_bytes_sent, source.framing_bytes_sent),
        (
            &mut target.framing_bytes_received,
            source.framing_bytes_received,
        ),
        (&mut target.frames_sent, source.frames_sent),
        (&mut target.frames_received, source.frames_received),
    ] {
        *target = target.checked_add(value).ok_or("relay counter overflow")?;
    }
    Ok(())
}

pub(super) fn relay(
    mut a: UnixStream,
    mut b: UnixStream,
    forward: RouteAdmission,
    reverse: RouteAdmission,
    links: [&LinkReservation; 2],
) -> Result<Value> {
    let wall = Instant::now();
    let cpu = ProcessTime::try_now()?;
    let cancel = CancellationToken::default();
    let mut ac = StreamCounters::default();
    let mut bc = StreamCounters::default();
    // Preadmit both directions: a busy forward pump cannot consume reverse
    // capacity. These fixed reservations cover idle reads as well as writes.
    let forward_capacity =
        links[0].reserve_frame(spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES as u64)?;
    let reverse_capacity =
        links[1].reserve_frame(spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES as u64)?;
    live::handshake_stream(
        &mut a,
        HandshakeRole::Acceptor,
        &cancel,
        Instant::now() + Duration::from_secs(30),
        &mut ac,
    )?;
    live::handshake_stream(
        &mut b,
        HandshakeRole::Acceptor,
        &cancel,
        Instant::now() + Duration::from_secs(30),
        &mut bc,
    )?;
    let a_write = a.try_clone()?;
    let b_write = b.try_clone()?;
    let a_read = a.try_clone()?;
    let b_read = b.try_clone()?;
    let _close = Close(&a, &b, &cancel);
    let (forward_report, reverse_report) = thread::scope(|scope| -> Result<_> {
        let token = &cancel;
        let forward_worker = thread::Builder::new()
            .name("mailbox-forward".into())
            .spawn_scoped(scope, move || {
                pump(a_read, b_write, &forward, forward_capacity, token)
            })?;
        let reverse_worker = match thread::Builder::new()
            .name("mailbox-reverse".into())
            .spawn_scoped(scope, move || {
                pump(b_read, a_write, &reverse, reverse_capacity, token)
            }) {
            Ok(worker) => worker,
            Err(error) => {
                stop(&a, &b, &cancel);
                let forward_report = forward_worker.join().unwrap_or_else(|_| PumpReport {
                    failure: Some("forward join panicked".into()),
                    ..Default::default()
                });
                return Ok((
                    forward_report,
                    PumpReport {
                        failure: Some(format!("reverse startup failed: {error}")),
                        ..Default::default()
                    },
                ));
            }
        };
        let forward_report = forward_worker.join();
        if forward_report.is_err() {
            stop(&a, &b, &cancel);
        }
        let reverse_report = reverse_worker.join();
        Ok((
            forward_report.map_err(|_| "forward join panicked")?,
            reverse_report.map_err(|_| "reverse join panicked")?,
        ))
    })?;
    add_counters(&mut ac, forward_report.received)?;
    add_counters(&mut ac, reverse_report.sent)?;
    add_counters(&mut bc, reverse_report.received)?;
    add_counters(&mut bc, forward_report.sent)?;
    if forward_report.failure.is_some() || reverse_report.failure.is_some() {
        return Err(format!(
            "relay failed: {}",
            json!({"forward":forward_report.failure,"reverse":reverse_report.failure,"a":ac,"b":bc})
        )
        .into());
    }
    Ok(
        json!({"event":"relay_done","a":ac,"b":bc,"cpu_us":micros(cpu.try_elapsed()?)?,"wall_us":micros(wall.elapsed())?}),
    )
}
