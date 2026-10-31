//! Small, explicit native guest supervisor for the development witness.
use crate::{
    Result,
    config::{Material, now_ms},
};
use mvm_contract::protocol::mailbox::{
    AuthorizationDomainRouteLabel, DestinationRouteToken, MailboxRoutingFrame, TransportBindingId,
};

pub struct RouteAdmission {
    binding_id: TransportBindingId,
    domain_label: AuthorizationDomainRouteLabel,
    destination: DestinationRouteToken,
    key_id: [u8; 32],
    not_before: i64,
    expires: i64,
}
impl RouteAdmission {
    pub fn new(material: &Material) -> Self {
        let claims = &material.binding.claims;
        Self {
            binding_id: claims.binding_id,
            domain_label: claims.authorization_domain_route_label,
            destination: claims.destination_route_token,
            key_id: material.key_id,
            not_before: claims.not_before_unix_ms,
            expires: claims.expires_at_unix_ms,
        }
    }
    pub fn validate(&self, routing: &MailboxRoutingFrame, key_id: &[u8; 32]) -> Result<()> {
        let now = now_ms()?;
        if routing.transport_binding_id != self.binding_id
            || routing.authorization_domain_route_label != self.domain_label
            || routing.destination_route_token != self.destination
            || key_id != &self.key_id
            || routing.hop_limit == 0
            || now < self.not_before
            || now >= self.expires
            || now >= routing.expires_at_unix_ms
            || routing.expires_at_unix_ms > self.expires
        {
            return Err("outer route is not admitted".into());
        }
        Ok(())
    }
}

use crate::{
    campaign,
    config::{Bootstrap, Pair, Plan, Published, Role, Stage},
    host,
};
use cpu_time::ProcessTime;
use mvm_core::mailbox::{decode_envelope, transport::CancellationToken};
use serde_json::{Value, json};
use spiralcore_mvm_adapter::live::{self, HandshakeRole, StreamCounters};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    net::Shutdown,
    os::{
        fd::AsRawFd,
        unix::{
            fs::{DirBuilderExt, OpenOptionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

fn private_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}
fn emit(out: &mut impl Write, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}
fn micros(duration: Duration) -> Result<u64> {
    Ok(u64::try_from(duration.as_micros())?)
}
fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err("file exceeds live witness bound".into());
    }
    Ok(bytes)
}

/// Owned child: early return terminates and reaps only this worker.
struct Worker {
    child: Option<Child>,
    output: PathBuf,
}
impl Worker {
    fn start(
        root: &Path,
        name: &str,
        kernel: &Path,
        guest: &[u8],
        bootstrap: &Bootstrap,
        disk: Option<&Path>,
        socket: Option<&Path>,
    ) -> Result<Self> {
        let output = root.join(format!("{name}.json"));
        let stdout = private_file(&output)?;
        let stderr = private_file(&root.join(format!("{name}.stderr")))?;
        let mut private_bootstrap = serde_json::to_value(bootstrap)?;
        private_bootstrap["utc_seconds"] = json!(u64::try_from(now_ms()? / 1000)?);
        let initrd = host::initramfs(guest, &serde_json::to_vec(&private_bootstrap)?)?;
        let child = Command::new(std::env::current_exe()?)
            .arg("worker")
            .arg(kernel)
            .arg(disk.unwrap_or(Path::new("-")))
            .arg(socket.unwrap_or(Path::new("-")))
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()?;
        let mut worker = Self {
            child: Some(child),
            output,
        };
        let mut input = worker
            .child
            .as_mut()
            .ok_or("missing child")?
            .stdin
            .take()
            .ok_or("missing child stdin")?;
        input.write_all(&initrd)?;
        drop(input); // Credential-bearing initramfs stays in the pipe/memory.
        Ok(worker)
    }
    fn finish(mut self) -> Result<Value> {
        let deadline = Instant::now() + Duration::from_secs(150);
        let status = loop {
            if let Some(status) = self.child.as_mut().ok_or("missing child")?.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                return Err("worker did not terminate within deadline".into());
            }
            std::thread::park_timeout(Duration::from_millis(10));
        };
        self.child.take();
        if !status.success() {
            return Err(format!(
                "worker failed: {status}; public receipt {}",
                self.output.display()
            )
            .into());
        }
        let value: Value = serde_json::from_slice(&read_bounded(&self.output, 4 << 20)?)?;
        if value["ok"] != true || value["event"] != "worker_done" {
            return Err("worker success receipt missing".into());
        }
        Ok(value)
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn accept(listener: &UnixListener) -> Result<UnixStream> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + Duration::from_secs(40);
    loop {
        match listener.accept() {
            Ok((socket, _)) => {
                socket.set_nonblocking(false)?;
                return Ok(socket);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("guest connect deadline")?;
        let mut fd = libc::pollfd {
            fd: listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one live pollfd, no aliasing, bounded integral timeout.
        let result = unsafe { libc::poll(&mut fd, 1, remaining.as_millis().min(1000) as i32) };
        if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
            return Err(io::Error::last_os_error().into());
        }
    }
}
fn event(worker: &Value, name: &str) -> Result<Value> {
    let events = worker["guest_events"]
        .as_array()
        .ok_or("missing guest events")?;
    let mut found = events.iter().filter(|value| value["event"] == name);
    let first = found
        .next()
        .ok_or_else(|| format!("missing {name} guest event"))?
        .clone();
    if found.next().is_some() {
        return Err(format!("duplicate {name} guest event").into());
    }
    Ok(first)
}
fn counters_agree(sender: &Value, receiver: &Value) -> Result<()> {
    for (sent, received) in [
        ("envelope_bytes_sent", "envelope_bytes_received"),
        ("framing_bytes_sent", "framing_bytes_received"),
        ("frames_sent", "frames_received"),
    ] {
        if sender[sent].as_u64().is_none()
            || sender[sent] != receiver[received]
            || sender[received] != receiver[sent]
        {
            return Err(format!("directional byte accounting mismatch: {sent}").into());
        }
    }
    Ok(())
}
fn byte_total(counters: &Value) -> Result<u64> {
    [
        "envelope_bytes_sent",
        "envelope_bytes_received",
        "framing_bytes_sent",
        "framing_bytes_received",
    ]
    .into_iter()
    .try_fold(0u64, |sum, key| {
        sum.checked_add(counters[key].as_u64().ok_or("missing byte counter")?)
            .ok_or("byte counter overflow")
    })
    .map_err(Into::into)
}
fn relay(
    mut a: UnixStream,
    mut b: UnixStream,
    forward: RouteAdmission,
    reverse: RouteAdmission,
) -> Result<Value> {
    let wall = Instant::now();
    let cpu = ProcessTime::try_now()?;
    let cancel = CancellationToken::default();
    let mut ac = StreamCounters::default();
    let mut bc = StreamCounters::default();
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
    loop {
        let before = ac.envelope_bytes_received + ac.framing_bytes_received;
        let wire = match live::read_ciphertext_frame(
            &mut a,
            &cancel,
            Instant::now() + Duration::from_secs(30),
            &mut ac,
        ) {
            Ok(wire) => wire,
            Err(live::LiveError::Io(error))
                if error.kind() == io::ErrorKind::UnexpectedEof
                    && before == ac.envelope_bytes_received + ac.framing_bytes_received =>
            {
                break;
            }
            Err(error) => return Err(error.into()),
        };
        let envelope = decode_envelope(&wire)?;
        forward.validate(&envelope.routing, &envelope.encryption.key_id)?;
        live::write_ciphertext_frame(
            &mut b,
            &wire,
            &cancel,
            Instant::now() + Duration::from_secs(30),
            &mut bc,
        )?;
        let reply = live::read_ciphertext_frame(
            &mut b,
            &cancel,
            Instant::now() + Duration::from_secs(30),
            &mut bc,
        )?;
        let envelope = decode_envelope(&reply)?;
        reverse.validate(&envelope.routing, &envelope.encryption.key_id)?;
        live::write_ciphertext_frame(
            &mut a,
            &reply,
            &cancel,
            Instant::now() + Duration::from_secs(30),
            &mut ac,
        )?;
    }
    let _ = a.shutdown(Shutdown::Both);
    let _ = b.shutdown(Shutdown::Both);
    Ok(
        json!({"event":"relay_done","a":ac,"b":bc,"cpu_us":micros(cpu.try_elapsed()?)?,"wall_us":micros(wall.elapsed())?}),
    )
}

/// Execute one explicitly selected persistent session. The caller chooses a new
/// private run directory; failures preserve public receipts and owned disk data.
pub fn session(root: &Path, kernel: &Path, guest_path: &Path, plan: Plan) -> Result<Value> {
    fs::DirBuilder::new().mode(0o700).create(root)?;
    let mut journal = private_file(&root.join("host-events.jsonl"))?;
    emit(
        &mut journal,
        &json!({"event":"session_begin","plan":plan,"utc_unix_ms":now_ms()?}),
    )?;
    let result = session_inner(root, kernel, guest_path, plan, &mut journal);
    match &result {
        Ok(value) => emit(&mut journal, value)?,
        Err(error) => emit(
            &mut journal,
            &json!({"event":"session_failed","error":error.to_string()}),
        )?,
    }
    journal.sync_all()?;
    File::open(root)?.sync_all()?;
    result
}
fn session_inner(
    root: &Path,
    kernel: &Path,
    guest_path: &Path,
    plan: Plan,
    journal: &mut File,
) -> Result<Value> {
    let wall = Instant::now();
    let cpu = ProcessTime::try_now()?;
    let guest = read_bounded(guest_path, 32 << 20)?;
    // Short private socket paths avoid the Unix socket pathname limit.
    let sockets = std::env::temp_dir().join(format!(
        "scmvm-{}-{:016x}",
        std::process::id(),
        rand::random::<u64>()
    ));
    fs::DirBuilder::new().mode(0o700).create(&sockets)?;
    struct SocketDir(PathBuf);
    impl Drop for SocketDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _socket_dir = SocketDir(sockets.clone());
    let a_path = sockets.join("a.sock");
    let a_listener = UnixListener::bind(&a_path)?;
    let pair = Pair::provision(plan.clone())?;
    let sender;
    let receiver;
    let worker_a;
    let mut worker_b = None;
    let mut recovery = None;
    let relay_receipt;
    if plan.stage == Stage::One {
        let worker = Worker::start(
            root,
            "sender",
            kernel,
            &guest,
            &pair.sender,
            None,
            Some(&a_path),
        )?;
        let stream = accept(&a_listener)?;
        let receive_result = campaign::receiver(
            &pair.receiver,
            &root.join("cache"),
            stream,
            HandshakeRole::Acceptor,
            journal,
        );
        let worker_result = worker.finish();
        receiver = receive_result?;
        worker_a = worker_result?;
        sender = event(&worker_a, "sender_done")?;
        relay_receipt = Value::Null;
    } else {
        let disk = root.join("receiver.ext4");
        host::create_disk(&disk)?;
        let b_path = sockets.join("b.sock");
        let b_listener = UnixListener::bind(&b_path)?;
        let forward =
            RouteAdmission::new(pair.sender.outgoing.as_ref().ok_or("missing call route")?);
        let reverse = RouteAdmission::new(
            pair.receiver
                .outgoing
                .as_ref()
                .ok_or("missing reply route")?,
        );
        let b = Worker::start(
            root,
            "receiver",
            kernel,
            &guest,
            &pair.receiver,
            Some(&disk),
            Some(&b_path),
        )?;
        let a = Worker::start(
            root,
            "sender",
            kernel,
            &guest,
            &pair.sender,
            None,
            Some(&a_path),
        )?;
        let a_socket = accept(&a_listener)?;
        let b_socket = accept(&b_listener)?;
        let relay_result = relay(a_socket, b_socket, forward, reverse);
        let a_result = a.finish();
        let b_result = b.finish();
        relay_receipt = relay_result?;
        emit(journal, &relay_receipt)?;
        worker_a = a_result?;
        let completed_b = b_result?;
        sender = event(&worker_a, "sender_done")?;
        receiver = event(&completed_b, "receiver_done")?;
        let published: Vec<Published> = serde_json::from_value(receiver["published"].clone())?;
        let verify = Bootstrap {
            role: Role::Verify,
            plan: plan.clone(),
            utc_seconds: u64::try_from(now_ms()? / 1000)?,
            incoming: None,
            outgoing: None,
            published,
        };
        let verifier = Worker::start(root, "recovery", kernel, &guest, &verify, Some(&disk), None)?;
        recovery = Some(verifier.finish()?);
        worker_b = Some(completed_b);
    }
    counters_agree(&sender["counters"], &receiver["counters"])?;
    let a_offset = worker_a["guest_clock_offset_ms"]
        .as_i64()
        .ok_or("missing sender clock observation")?;
    let b_offset = worker_b
        .as_ref()
        .map(|worker| {
            worker["guest_clock_offset_ms"]
                .as_i64()
                .ok_or("missing receiver clock observation")
        })
        .transpose()?
        .unwrap_or(0);
    if a_offset
        .checked_sub(b_offset)
        .ok_or("clock offset overflow")?
        .unsigned_abs()
        > 5000
    {
        return Err("pairwise guest clock bound exceeded".into());
    }
    if worker_b.is_some() {
        counters_agree(&sender["counters"], &relay_receipt["a"])?;
        counters_agree(&receiver["counters"], &relay_receipt["b"])?;
    }
    let logical = byte_total(&sender["counters"])?;
    let physical = logical
        .checked_mul(if plan.stage == Stage::Two { 2 } else { 1 })
        .ok_or("physical byte overflow")?;
    let value = json!({"event":"session_done","ok":true,"plan":plan,"sender":sender,"receiver":receiver,
        "logical_envelope_and_framing_bytes":logical,"physical_vsock_stream_bytes":physical,
        "relay":relay_receipt,"sender_worker":worker_a,"receiver_worker":worker_b,"recovery_worker":recovery,
        "pairwise_clock_offset_ms":a_offset-b_offset,
        "orchestrator_process_cpu_us":micros(cpu.try_elapsed()?)?,"lifecycle_wall_us":micros(wall.elapsed())?,
        "byte_scope":"both directions, encrypted MVM envelopes, 4-byte frame prefixes and readiness; excludes virtio control/credit headers",
        "cpu_scope":"endpoint process CPU is separate from worker/VMM process CPU; do not sum nested guest and worker CPU"});
    let mut file = private_file(&root.join("result.json"))?;
    serde_json::to_writer_pretty(&mut file, &value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Arm, Pair, Plan, Stage};
    use mvm_contract::protocol::mailbox::*;
    #[test]
    fn relay_admits_only_the_provisioned_unexpired_outer_route() {
        let pair = Pair::provision(Plan {
            stage: Stage::Two,
            arm: Arm::Warm,
            size: 4096,
            trial: 0,
        })
        .unwrap();
        let material = pair.sender.outgoing.as_ref().unwrap();
        let claims = &material.binding.claims;
        let routing = MailboxRoutingFrame {
            protocol_version: ProtocolVersion::V1,
            authorization_domain_route_label: claims.authorization_domain_route_label,
            destination_route_token: claims.destination_route_token,
            delivery_id: DeliveryId::from_bytes([3; 32]),
            ciphertext_len: 99,
            padding: CiphertextPadding::Exact,
            expires_at_unix_ms: now_ms().unwrap() + 10000,
            hop_limit: 2,
            transport_binding_id: claims.binding_id,
        };
        let admission = RouteAdmission::new(material);
        admission.validate(&routing, &material.key_id).unwrap();
        let mut wrong = routing.clone();
        wrong.destination_route_token = DestinationRouteToken::from_bytes([4; 32]);
        assert!(admission.validate(&wrong, &material.key_id).is_err());
        let mut expired = routing.clone();
        expired.expires_at_unix_ms = now_ms().unwrap();
        assert!(admission.validate(&expired, &material.key_id).is_err());
        assert!(admission.validate(&routing, &[0; 32]).is_err());
    }
}
