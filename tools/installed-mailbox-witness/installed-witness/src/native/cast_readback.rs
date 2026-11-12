//! Read the original destination workload through its ordinary MVM call.
//! The body expected from A is never part of B's call input.
use anyhow::{Context, Result, ensure};
use cpu_time::ProcessTime;
use mvm_client::{
    entrypoint::dispatch::{self, CallObserver, CapturedOutput, DispatchStdin, EntrypointDispatch},
    launch::LaunchOutcome,
};
use mvm_core::protocol::mailbox_worker_control::{MailboxWorkerState, context_commitment};
use mvm_hostd::{mailbox_process::owned_client::InstalledMailboxClient, stream::ShownChunk};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub const DESTINATION_WORKLOAD: &str = "/usr/local/bin/spiralcore-cast-recipient";
const READBACK_WINDOW: Duration = Duration::from_secs(15);
const CHALLENGE_BYTES: usize = 32;
const MAX_FIXTURE: usize = 1024;

#[derive(Clone, Copy)]
pub struct OriginalParticipant<'a> {
    pub launch: &'a LaunchOutcome,
    pub control: &'a InstalledMailboxClient,
}

impl OriginalParticipant<'_> {
    fn verify(&self) -> Result<()> {
        let r = self.control.acquire_request();
        let owner = self
            .control
            .owner()
            .context("no retained original worker")?;
        let plan = self.launch.admitted.plan();
        let digest: String = r
            .plan_commitment
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        ensure!(
            plan.mailbox == Some(r.mailbox_grant)
                && r.mailbox_grant.local_service.is_some()
                && self.launch.machine.name == r.vm_id
                && r.vm_id == plan.workload.0
                && r.workload_id == plan.workload.0
                && r.tenant == plan.tenant.0
                && self.launch.plan_id == format!("sha256:{digest}")
                && self.launch.admitted.plan_id().0 == self.launch.plan_id
                && context_commitment(r)? == owner.context_commitment,
            "original plan, local-service grant or worker binding changed"
        );
        ensure!(
            self.control
                .query(Instant::now() + Duration::from_secs(2))?
                == MailboxWorkerState::Serving,
            "original worker no longer Serving"
        );
        Ok(())
    }
}

struct ReadbackObserver {
    captured: CapturedOutput,
    controls: bool,
    captured_bytes: usize,
}

impl CallObserver for ReadbackObserver {
    fn output(&mut self, chunk: &ShownChunk) {
        // The bounded fixture and the guest's JSON timing record fit within 2 KiB.
        let remaining = (MAX_FIXTURE + CHALLENGE_BYTES + 512) - self.captured_bytes;
        if self.captured.truncated || chunk.body.len() > remaining {
            self.captured.truncated = true;
            return;
        }
        self.captured_bytes += chunk.body.len();
        self.captured.output(chunk);
    }
    fn control(&mut self, _: &str, _: usize) {
        self.controls = true;
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadTime {
    pub workload_cpu_ns: u64,
    pub workload_wall_ns: u64,
}

#[derive(Serialize)]
pub struct CastReadback {
    pub source_plan: String,
    pub destination_plan: String,
    pub received_bytes: usize,
    pub received_sha256: String,
    pub challenge_bytes: usize,
    pub verified_response_bytes: usize,
    pub workload: WorkloadTime,
    pub driver_process_cpu_ns: u128,
    pub dispatch_through_verified_readback_ns: u128,
}

/// Called in the original pair driver, after B's warm OCI worker was launched.
/// `deliver` performs A's original guest Submit and its admitted route to B.
/// Its error propagates even if B wrote a response. Both signed workers must
/// remain Serving across the observed call. Readback is an application witness,
/// independent of the mailbox journal/receipt lifecycle.
pub fn verify_during_delivery(
    a: OriginalParticipant<'_>,
    b: OriginalParticipant<'_>,
    expected: &[u8],
    deliver: impl FnOnce() -> Result<()>,
) -> Result<CastReadback> {
    ensure!(
        !expected.is_empty() && expected.len() <= MAX_FIXTURE,
        "unsupported synthetic Cast size"
    );
    a.verify()?;
    b.verify()?;
    let (ra, rb) = (a.control.acquire_request(), b.control.acquire_request());
    let (oa, ob) = (a.control.owner().unwrap(), b.control.owner().unwrap());
    ensure!(
        ra.vm_id != rb.vm_id
            && ra.mailbox_grant.mailbox != rb.mailbox_grant.mailbox
            && ra.tenant == rb.tenant
            && ra.backend == rb.backend
            && oa.generation != ob.generation
            && oa.lease != ob.lease
            && oa.context_commitment != ob.context_commitment
            && oa.daemon_boot_nonce == ob.daemon_boot_nonce,
        "expected two original guest roles on one installed coordinator"
    );
    let mut challenge = [0u8; CHALLENGE_BYTES];
    File::open("/dev/urandom")?.read_exact(&mut challenge)?;
    let (begun, notice) = mpsc::sync_channel(1);
    let destination = b.launch.machine.name.clone();
    let begin = Instant::now();
    let cpu = ProcessTime::now();
    let captured = thread::scope(|s| -> Result<CapturedOutput> {
        let call = s.spawn(|| -> Result<CapturedOutput> {
            // The thread is scheduled. The signal is not guest-readiness evidence.
            begun.send(()).context("source delivery turn closed")?;
            call_original(&destination, challenge.to_vec())
        });
        let direction = notice
            .recv_timeout(READBACK_WINDOW)
            .context("destination dispatch did not start")
            .and_then(|()| deliver());
        let outcome = call
            .join()
            .map_err(|_| anyhow::anyhow!("destination call thread ended unexpectedly"))?;
        direction?;
        outcome
    })?;
    let payload = super::cast_echo::verify(expected, &challenge, &captured.stdout)
        .context("destination workload did not return the exact Cast and call challenge")?;
    let workload =
        serde_json::from_slice(&captured.stderr).context("invalid original workload timing")?;
    let wall_ns = begin.elapsed().as_nanos();
    let cpu_ns = cpu.elapsed().as_nanos();
    a.verify()?;
    b.verify()?;
    Ok(CastReadback {
        source_plan: a.launch.plan_id.clone(),
        destination_plan: b.launch.plan_id.clone(),
        received_bytes: payload.len(),
        received_sha256: super::digest(payload),
        challenge_bytes: challenge.len(),
        verified_response_bytes: captured.stdout.len(),
        workload,
        driver_process_cpu_ns: cpu_ns,
        dispatch_through_verified_readback_ns: wall_ns,
    })
}

fn call_original(name: &str, input: Vec<u8>) -> Result<CapturedOutput> {
    let mut observer = ReadbackObserver {
        captured: CapturedOutput::default(),
        controls: false,
        captured_bytes: 0,
    };
    let outcome = dispatch::dispatch(
        EntrypointDispatch {
            vm_name: name,
            stdin: DispatchStdin::OneShot(input),
            timeout_secs: READBACK_WINDOW.as_secs(),
            session_id: None,
        },
        &mut observer,
    )?;
    ensure!(
        outcome.exit_code() == 0 && !observer.controls && !observer.captured.truncated,
        "original call did not finish with complete workload output"
    );
    Ok(observer.captured)
}

#[derive(Serialize)]
pub struct AddressedReadback {
    pub destination: CastReadback,
    pub source_message_id: [u8; 32],
    pub source_workload: WorkloadTime,
    pub source_call_through_ephemeral_acceptance_ns: u128,
}

/// Execute the normal guest A workload call and B's independent workload call
/// while the installed carrier delivers the canonical Send. The carrier's
/// verdict only confirms transient ingress. The coordinator cannot see the
/// encrypted message identifier. Only A reports it; B independently returns
/// the original bytes and the private call challenge.
pub fn verify_original_pair(
    a: OriginalParticipant<'_>,
    b: OriginalParticipant<'_>,
    expected: &[u8],
    route: impl FnOnce() -> Result<()>,
) -> Result<AddressedReadback> {
    let name = a.launch.machine.name.clone();
    let mut source = None;
    let destination = verify_during_delivery(a, b, expected, || {
        thread::scope(|scope| -> Result<()> {
            let send = scope.spawn(|| {
                let start = Instant::now();
                let out = call_original(&name, expected.to_vec())?;
                let workload: WorkloadTime = serde_json::from_slice(&out.stderr)
                    .context("invalid original source workload timings")?;
                let id: [u8; 32] = out.stdout.try_into().map_err(|_| {
                    anyhow::anyhow!("source did not return a complete message identifier")
                })?;
                ensure!(id != [0; 32], "missing source message identifier");
                Ok::<_, anyhow::Error>((id, start.elapsed().as_nanos(), workload))
            });
            let routed = route();
            let (id, elapsed, workload) = send
                .join()
                .map_err(|_| anyhow::anyhow!("source workload call ended unexpectedly"))??;
            routed?;
            source = Some((id, elapsed, workload));
            Ok(())
        })
    })?;
    let (id, elapsed, workload) = source.context("missing original source observation")?;
    Ok(AddressedReadback {
        destination,
        source_message_id: id,
        source_workload: workload,
        source_call_through_ephemeral_acceptance_ns: elapsed,
    })
}
