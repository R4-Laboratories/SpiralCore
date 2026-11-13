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
    key_ids: Vec<[u8; 32]>,
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
            key_ids: vec![material.key_id],
            not_before: claims.not_before_unix_ms,
            expires: claims.expires_at_unix_ms,
        }
    }
    pub fn with_control(material: &Material, control: &Material) -> Result<Self> {
        if material.binding != control.binding || material.key_id == control.key_id {
            return Err("receipt route differs from admitted physical route".into());
        }
        let mut admission = Self::new(material);
        admission.key_ids.push(control.key_id);
        Ok(admission)
    }
    pub fn validate(&self, routing: &MailboxRoutingFrame, key_id: &[u8; 32]) -> Result<()> {
        let now = now_ms()?;
        if routing.transport_binding_id != self.binding_id
            || routing.authorization_domain_route_label != self.domain_label
            || routing.destination_route_token != self.destination
            || !self.key_ids.contains(key_id)
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
    host_budget::{
        HostTransportBudget, LinkReservation, TransportBudgetConfig, TransportLimits,
        TransportScope,
    },
    installed_supervisor::{InstalledSupervisor, WorkerBinding, WorkerGrant, WorkerRole},
    installed_worker::ParentControl,
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
        fd::{AsRawFd, OwnedFd},
        unix::{
            fs::{DirBuilderExt, OpenOptionsExt},
            net::{UnixListener, UnixStream},
            process::CommandExt,
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
enum LaunchContext<'a> {
    Legacy,
    Installed(&'a mut InstalledSupervisor),
}
impl LaunchContext<'_> {
    fn installed(&mut self) -> Result<&mut InstalledSupervisor> {
        match self {
            Self::Installed(controller) => Ok(controller),
            Self::Legacy => Err("installed worker cannot use a legacy launch context".into()),
        }
    }
}
struct InstalledWorker {
    grant: WorkerGrant,
    binding: WorkerBinding,
    _control: ParentControl,
}
struct Worker {
    child: Option<Child>,
    installed: Option<InstalledWorker>,
    output: PathBuf,
    #[cfg(target_os = "linux")]
    home: PathBuf,
    #[cfg(target_os = "linux")]
    pid: Option<u32>,
}
impl Worker {
    fn start(
        root: &Path,
        kernel: &Path,
        guest: &[u8],
        bootstrap: &Bootstrap,
        disk: Option<&Path>,
        socket: Option<&Path>,
        launch: &mut LaunchContext<'_>,
    ) -> Result<Self> {
        let name = match bootstrap.role {
            Role::Sender => "sender",
            Role::Receiver => "receiver",
            Role::Verify => "recovery",
        };
        let output = root.join(format!("{name}.json"));
        let stdout = private_file(&output)?;
        let stderr = private_file(&root.join(format!("{name}.stderr")))?;
        let mut private_bootstrap = serde_json::to_value(bootstrap)?;
        private_bootstrap["utc_seconds"] = json!(u64::try_from(now_ms()? / 1000)?);
        private_bootstrap["report_exit"] = json!(cfg!(target_os = "linux"));
        let initrd = host::initramfs(guest, &serde_json::to_vec(&private_bootstrap)?)?;
        let home = root.join(format!("mvm-{name}"));
        fs::DirBuilder::new().mode(0o700).create(&home)?;
        if let LaunchContext::Installed(controller) = launch {
            let (mut control, child_control) = ParentControl::pair()?;
            let (input_reader, mut input_writer) = UnixStream::pair()?;
            let role = match bootstrap.role {
                Role::Sender => WorkerRole::Sender,
                Role::Receiver => WorkerRole::Receiver,
                Role::Verify => WorkerRole::Verify,
            };
            let role_number = match role {
                WorkerRole::Sender => 1,
                WorkerRole::Receiver => 2,
                WorkerRole::Verify => 3,
            };
            let fd = child_control.as_raw_fd();
            let mut command = Command::new(std::env::current_exe()?);
            command
                .env("MVM_HOME", &home)
                .arg("worker-installed")
                .arg(kernel)
                .arg(disk.unwrap_or(Path::new("-")))
                .arg(socket.unwrap_or(Path::new("-")))
                .arg(bootstrap.plan.bandwidth_profile.as_str())
                .arg(role_number.to_string())
                .stdin(Stdio::from(OwnedFd::from(input_reader)))
                .stdout(stdout)
                .stderr(stderr);
            // SAFETY: the child callback uses only scalar dup2/fcntl and an
            // owned pair endpoint kept alive until spawn returns. No allocation,
            // locks, received callback or owner-EX descriptor is passed.
            unsafe {
                command.pre_exec(move || {
                    if fd != 198 && libc::dup2(fd, 198) < 0 {
                        return Err(io::Error::last_os_error());
                    }
                    let flags = libc::fcntl(198, libc::F_GETFD);
                    if flags < 0 || libc::fcntl(198, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let grant = controller.spawn_worker(role, &mut command)?;
            drop(child_control);
            control.receive_challenge()?;
            let binding = controller.worker_binding(&grant)?;
            control.send_binding(&binding)?;
            control.wait_ready()?;
            control.write_initrd(&mut input_writer, &initrd)?;
            control.receive_boot_challenge()?;
            let fresh = controller.worker_binding(&grant)?;
            control.send_go(&fresh)?;
            return Ok(Self {
                child: None,
                installed: Some(InstalledWorker {
                    grant,
                    binding,
                    _control: control,
                }),
                output,
                #[cfg(target_os = "linux")]
                home,
                #[cfg(target_os = "linux")]
                pid: None,
            });
        }
        let child = Command::new(std::env::current_exe()?)
            .env("MVM_HOME", &home)
            .arg("worker")
            .arg(kernel)
            .arg(disk.unwrap_or(Path::new("-")))
            .arg(socket.unwrap_or(Path::new("-")))
            .arg(bootstrap.plan.bandwidth_profile.as_str())
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()?;
        let mut worker = Self {
            #[cfg(target_os = "linux")]
            pid: Some(child.id()),
            #[cfg(target_os = "linux")]
            home,
            child: Some(child),
            installed: None,
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
    fn finish(mut self, launch: &mut LaunchContext<'_>) -> Result<Value> {
        let deadline = Instant::now() + Duration::from_secs(150);
        let status = loop {
            let status = match &self.installed {
                Some(worker) => launch.installed()?.try_reap_worker(&worker.grant)?,
                None => self.child.as_mut().ok_or("missing child")?.try_wait()?,
            };
            if let Some(status) = status {
                break status;
            }
            if Instant::now() >= deadline {
                return Err("worker did not terminate within deadline".into());
            }
            std::thread::park_timeout(Duration::from_millis(10));
        };
        self.child.take();
        #[cfg(target_os = "linux")]
        if self.installed.is_none() {
            self.cleanup_vm()?;
        }
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
        if let Some(worker) = &self.installed {
            if value["installed_host_lease"] != true
                || value["backend_exit_confirmed"] != true
                || value["installed_generation"] != worker.binding.generation
                || value["installed_role"] != worker.binding.role
            {
                return Err("installed worker backend-exit receipt missing".into());
            }
            launch.installed()?.confirm_worker_exit(&worker.grant)?;
            self.installed.take();
        }
        Ok(value)
    }

    #[cfg(target_os = "linux")]
    fn cleanup_vm(&self) -> Result<()> {
        let status = Command::new(std::env::current_exe()?)
            .arg("cleanup-worker")
            .arg(
                self.pid
                    .ok_or("installed workers prohibit PID-based cleanup")?
                    .to_string(),
            )
            .env("MVM_HOME", &self.home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()?;
        if !status.success() {
            return Err("owned Firecracker cleanup failed".into());
        }
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(worker) = &self.installed {
            worker.grant.close_admission();
            return;
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            #[cfg(target_os = "linux")]
            if self.cleanup_vm().is_err() {
                eprintln!("owned Firecracker cleanup failed; inspect the private worker directory");
            }
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
mod duplex;
use duplex::relay;

#[cfg(test)]
pub(crate) fn relay_for_test(
    a: UnixStream,
    b: UnixStream,
    forward: RouteAdmission,
    reverse: RouteAdmission,
    links: [&LinkReservation; 2],
) -> Result<Value> {
    relay(a, b, forward, reverse, links)
}

// Focused existing refusal tests use the same reserved forwarding path.
#[cfg(test)]
fn forward_frame(
    input: &mut UnixStream,
    output: &mut UnixStream,
    route: &RouteAdmission,
    link: &LinkReservation,
    cancel: &CancellationToken,
    counters: (&mut StreamCounters, &mut StreamCounters),
    allow_clean_eof: bool,
) -> Result<bool> {
    let capacity = link.reserve_frame(spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES as u64)?;
    duplex::forward_frame(
        input,
        output,
        route,
        &capacity,
        cancel,
        counters,
        allow_clean_eof,
    )
}

/// Execute one explicitly selected persistent session. The caller chooses a new
/// private run directory; failures preserve public receipts and owned disk data.
pub fn session(root: &Path, kernel: &Path, guest_path: &Path, plan: Plan) -> Result<Value> {
    if plan.stage == Stage::One {
        // The host is a plaintext endpoint here, not a keyless relay. Its
        // StreamTransport buffers are not covered by this relay-only profile.
        return session_owned(
            root,
            kernel,
            guest_path,
            plan,
            None,
            &mut LaunchContext::Legacy,
        );
    }
    let budget = transport_budget()?;
    let scope = budget.scope(0)?;
    session_with_budget(root, kernel, guest_path, plan, &budget, &scope)
}

/// Default dedicated-supervisor profile. This is not machine-wide exclusion.
pub fn transport_budget() -> Result<HostTransportBudget> {
    let limits = TransportLimits {
        links: 2,
        frame_slots: 2,
        serialized_buffer_bytes: 2 * spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES as u64,
    };
    Ok(HostTransportBudget::new(TransportBudgetConfig {
        host: limits,
        tenant: limits,
        max_tenants: 1,
        max_frame_bytes: spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES as u64,
    })?)
}

/// Reuse one owner across all sessions of a supervisor; scope comes from its
/// trusted local provisioning, never from a received ciphertext route label.
pub fn session_with_budget(
    root: &Path,
    kernel: &Path,
    guest_path: &Path,
    plan: Plan,
    budget: &HostTransportBudget,
    scope: &TransportScope,
) -> Result<Value> {
    if budget.is_installation_bound() {
        return Err("installed budget requires its installed launch context".into());
    }
    if plan.stage != Stage::Two {
        return Err("transport-budget v1 covers the two-guest keyless relay only".into());
    }
    session_owned(
        root,
        kernel,
        guest_path,
        plan,
        Some((budget, scope)),
        &mut LaunchContext::Legacy,
    )
}

/// One installed session, with no route into the legacy unleased launcher.
/// The caller explicitly retires the epoch after this entry drains its workers.
pub fn session_installed(
    controller: &mut InstalledSupervisor,
    root: &Path,
    kernel: &Path,
    guest_path: &Path,
    plan: Plan,
) -> Result<Value> {
    controller.claim_session()?;
    let budget = controller.budget().clone();
    let result = (|| {
        // Fail before run files or child activation on missing trusted assets.
        File::open(kernel)?;
        File::open(guest_path)?;
        let scope = budget.scope(0)?;
        let accounted = (plan.stage == Stage::Two).then_some((&budget, &scope));
        session_owned(
            root,
            kernel,
            guest_path,
            plan,
            accounted,
            &mut LaunchContext::Installed(controller),
        )
    })();
    // Also handles a child adopted during spawn whose startup never returned a
    // token to Worker. Cleanup uses exact owned handles, without PID attach.
    let drained = controller.close_and_drain_workers();
    match (result, drained) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
    }
}

fn session_owned(
    root: &Path,
    kernel: &Path,
    guest_path: &Path,
    plan: Plan,
    budget: Option<(&HostTransportBudget, &TransportScope)>,
    launch: &mut LaunchContext<'_>,
) -> Result<Value> {
    fs::DirBuilder::new().mode(0o700).create(root)?;
    let mut journal = private_file(&root.join("host-events.jsonl"))?;
    emit(
        &mut journal,
        &json!({"event":"session_begin","plan":plan,"utc_unix_ms":now_ms()?}),
    )?;
    let result = (|| {
        // Reserve all physical links before activating either child process.
        let links = match budget {
            Some((budget, scope)) => (0..2)
                .map(|_| budget.reserve_link(scope))
                .collect::<std::result::Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        session_inner(root, kernel, guest_path, plan, &mut journal, &links, launch)
    })();
    let snapshot = budget.map(|(budget, _)| budget.snapshot()).transpose()?;
    emit(
        &mut journal,
        &json!({"event":"host_transport_released","budget":snapshot}),
    )?;
    let result = result.and_then(|mut value| -> Result<Value> {
        value["host_transport_budget"] = serde_json::to_value(&snapshot)?;
        value["host_transport_budget_coverage"] = json!(if snapshot.is_some() {
            "two-guest keyless relay serialized buffers and links only"
        } else {
            "not accounted: one-guest host plaintext endpoint"
        });
        Ok(value)
    });
    match &result {
        Ok(value) => {
            emit(&mut journal, value)?;
            let mut file = private_file(&root.join("result.json"))?;
            serde_json::to_writer_pretty(&mut file, value)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
        }
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
    links: &[LinkReservation],
    launch: &mut LaunchContext<'_>,
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
            kernel,
            &guest,
            &pair.sender,
            None,
            Some(&a_path),
            launch,
        )?;
        let stream = accept(&a_listener)?;
        let receive_result = campaign::receiver(
            &pair.receiver,
            &root.join("cache"),
            stream,
            HandshakeRole::Acceptor,
            journal,
        );
        let worker_result = worker.finish(launch);
        receiver = receive_result?;
        worker_a = worker_result?;
        sender = event(&worker_a, "sender_done")?;
        relay_receipt = Value::Null;
    } else {
        let disk = root.join("receiver.ext4");
        host::create_disk(&disk)?;
        let b_path = sockets.join("b.sock");
        let b_listener = UnixListener::bind(&b_path)?;
        let forward_material = pair.sender.outgoing.as_ref().ok_or("missing call route")?;
        let reverse_material = pair
            .receiver
            .outgoing
            .as_ref()
            .ok_or("missing reply route")?;
        let forward = RouteAdmission::with_control(
            forward_material,
            &pair
                .sender
                .service
                .as_ref()
                .ok_or("missing sender service profile")?
                .outgoing_control,
        )?;
        let reverse = RouteAdmission::with_control(
            reverse_material,
            &pair
                .receiver
                .service
                .as_ref()
                .ok_or("missing receiver service profile")?
                .outgoing_control,
        )?;
        let b = Worker::start(
            root,
            kernel,
            &guest,
            &pair.receiver,
            Some(&disk),
            Some(&b_path),
            launch,
        )?;
        let a = Worker::start(
            root,
            kernel,
            &guest,
            &pair.sender,
            None,
            Some(&a_path),
            launch,
        )?;
        let a_socket = accept(&a_listener)?;
        let b_socket = accept(&b_listener)?;
        let relay_result = relay(a_socket, b_socket, forward, reverse, [&links[0], &links[1]]);
        let a_result = a.finish(launch);
        let b_result = b.finish(launch);
        relay_receipt = relay_result?;
        emit(journal, &relay_receipt)?;
        worker_a = a_result?;
        let completed_b = b_result?;
        sender = event(&worker_a, "sender_done")?;
        receiver = event(&completed_b, "receiver_done")?;
        let published: Vec<Published> = serde_json::from_value(receiver["published"].clone())?;
        let verify = Bootstrap {
            report_exit: false,
            role: Role::Verify,
            plan: plan.clone(),
            utc_seconds: u64::try_from(now_ms()? / 1000)?,
            incoming: None,
            outgoing: None,
            service: None,
            published,
        };
        let verifier = Worker::start(root, kernel, &guest, &verify, Some(&disk), None, launch)?;
        recovery = Some(verifier.finish(launch)?);
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
        "backend":if cfg!(target_os="linux") {"firecracker"} else {"hvf"},
        "mailbox_rate_enforced": !cfg!(target_os="linux"),
        "mailbox_rate_bytes_per_second":if cfg!(target_os="linux") {None} else {Some(plan.bandwidth_profile.bytes_per_second())},
        "mailbox_burst_bytes":if cfg!(target_os="linux") {None} else {Some(plan.bandwidth_profile.burst_bytes())},
        "logical_envelope_and_framing_bytes":logical,"physical_vsock_stream_bytes":physical,
        "relay":relay_receipt,"sender_worker":worker_a,"receiver_worker":worker_b,"recovery_worker":recovery,
        "pairwise_clock_offset_ms":a_offset-b_offset,
        "orchestrator_process_cpu_us":micros(cpu.try_elapsed()?)?,"lifecycle_wall_us":micros(wall.elapsed())?,
        "byte_scope":"both directions, encrypted MVM envelopes, 4-byte frame prefixes and readiness; excludes virtio control/credit headers",
        "cpu_scope":"endpoint process CPU is separate from worker/VMM process CPU; do not sum nested guest and worker CPU"});
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Arm, Pair, Plan, Stage};
    use mvm_contract::protocol::mailbox::*;

    #[test]
    fn shared_owner_refuses_before_spawn_and_partial_link_admission_rolls_back() {
        let root = tempfile::tempdir().unwrap();
        let owner = transport_budget().unwrap();
        let tenant = owner.scope(0).unwrap();
        let held = owner.reserve_link(&tenant).unwrap();
        let plan = Plan {
            stage: Stage::Two,
            arm: Arm::Raw,
            size: 128,
            trial: 0,
            bandwidth_profile: Default::default(),
        };
        let run = root.path().join("refused");
        let error = session_with_budget(
            &run,
            Path::new("missing-kernel"),
            Path::new("missing-guest"),
            plan.clone(),
            &owner,
            &tenant,
        )
        .unwrap_err();
        assert!(error.to_string().contains("Full"));
        let snapshot = owner.snapshot().unwrap();
        assert_eq!(snapshot.usage.links, 1);
        assert_eq!(snapshot.high_water.links, 2);
        assert_eq!(snapshot.usage.frame_slots, 0);
        assert!(!run.join("result.json").exists());
        let names: Vec<_> = fs::read_dir(&run)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["host-events.jsonl"]);
        let mut one = plan;
        one.stage = Stage::One;
        let run = root.path().join("unsupported");
        let error = session_with_budget(
            &run,
            Path::new("missing-kernel"),
            Path::new("missing-guest"),
            one,
            &owner,
            &tenant,
        )
        .unwrap_err();
        assert!(error.to_string().contains("keyless relay only"));
        assert!(!run.exists());
        drop(held);
        assert_eq!(owner.snapshot().unwrap().usage.links, 0);
    }

    #[test]
    fn partial_read_wrong_route_cancel_and_failed_write_release_buffer_credit() {
        let pair = Pair::provision(Plan {
            stage: Stage::Two,
            arm: Arm::Raw,
            size: 128,
            trial: 0,
            bandwidth_profile: Default::default(),
        })
        .unwrap();
        let material = pair.sender.outgoing.as_ref().unwrap();
        let route = RouteAdmission::new(material);
        let binding = material.service_binding(None).unwrap();
        let now = spiralcore_mvm_adapter::service::ServiceTime {
            unix_ms: now_ms().unwrap(),
            monotonic_ms: 0,
        };
        let mut options = spiralcore_mvm_adapter::service_sender::MessageOptions::fresh(
            ContentType::parse(spiralcore_mvm_adapter::CONTENT_TYPE, 1).unwrap(),
            now,
            now.unix_ms + 10000,
        )
        .unwrap();
        options.reply_to = material.reply_to;
        let prepared = spiralcore_mvm_adapter::service_sender::prepare(
            &binding,
            b"opaque bytes",
            &options,
            now,
        )
        .unwrap();
        for mode in ["truncated", "wrong-route", "cancelled", "failed-write"] {
            let owner = transport_budget().unwrap();
            let link = owner.reserve_link(&owner.scope(0).unwrap()).unwrap();
            let (mut peer, mut input) = UnixStream::pair().unwrap();
            let (mut output, sink) = UnixStream::pair().unwrap();
            let cancel = CancellationToken::default();
            let mut envelope = prepared.envelope().clone();
            if mode == "wrong-route" {
                envelope.routing.destination_route_token =
                    DestinationRouteToken::from_bytes([8; 32]);
            }
            let wire = mvm_core::mailbox::encode_envelope(&envelope).unwrap();
            peer.write_all(&(wire.len() as u32).to_be_bytes()).unwrap();
            if mode == "truncated" {
                peer.write_all(&wire[..3]).unwrap();
                peer.shutdown(Shutdown::Write).unwrap();
            } else {
                peer.write_all(&wire).unwrap();
            }
            if mode == "cancelled" {
                cancel.cancel();
            }
            if mode == "failed-write" {
                sink.shutdown(Shutdown::Both).unwrap();
                drop(sink);
            }
            let mut read = StreamCounters::default();
            let mut sent = StreamCounters::default();
            assert!(
                forward_frame(
                    &mut input,
                    &mut output,
                    &route,
                    &link,
                    &cancel,
                    (&mut read, &mut sent),
                    false
                )
                .is_err(),
                "{mode}"
            );
            let snapshot = owner.snapshot().unwrap();
            assert_eq!(snapshot.usage.frame_slots, 0, "{mode}");
            assert_eq!(snapshot.usage.serialized_buffer_bytes, 0, "{mode}");
            if mode != "failed-write" {
                assert_eq!(sent.envelope_bytes_sent, 0);
            }
            drop(link);
            assert_eq!(owner.snapshot().unwrap().usage.links, 0);
        }
    }
    #[test]
    fn relay_credit_precedes_payload_read_and_malformed_reads_release_it() {
        use crate::host_budget::*;
        let owner = HostTransportBudget::new(TransportBudgetConfig {
            host: TransportLimits {
                links: 1,
                frame_slots: 1,
                serialized_buffer_bytes: 262144,
            },
            tenant: TransportLimits {
                links: 1,
                frame_slots: 1,
                serialized_buffer_bytes: 262144,
            },
            max_tenants: 1,
            max_frame_bytes: 262144,
        })
        .unwrap();
        let link = owner.reserve_link(&owner.scope(0).unwrap()).unwrap();
        let held = link.reserve_frame(262144).unwrap();
        let pair = Pair::provision(Plan {
            stage: Stage::Two,
            arm: Arm::Raw,
            size: 128,
            trial: 0,
            bandwidth_profile: Default::default(),
        })
        .unwrap();
        let route = RouteAdmission::new(pair.sender.outgoing.as_ref().unwrap());
        let (mut peer, mut input) = UnixStream::pair().unwrap();
        let (mut output, _sink) = UnixStream::pair().unwrap();
        let cancel = CancellationToken::default();
        let mut read = StreamCounters::default();
        let mut written = StreamCounters::default();
        assert!(
            forward_frame(
                &mut input,
                &mut output,
                &route,
                &link,
                &cancel,
                (&mut read, &mut written),
                false
            )
            .is_err()
        );
        assert_eq!(read.framing_bytes_received, 0);
        assert_eq!(owner.snapshot().unwrap().usage.frame_slots, 1);
        drop(held);
        // A zero-length frame fails after its prefix, before a payload read.
        peer.write_all(&[0; 4]).unwrap();
        assert!(
            forward_frame(
                &mut input,
                &mut output,
                &route,
                &link,
                &cancel,
                (&mut read, &mut written),
                false
            )
            .is_err()
        );
        assert_eq!(read.framing_bytes_received, 4);
        assert_eq!(read.envelope_bytes_received, 0);
        assert_eq!(written.envelope_bytes_sent, 0);
        let snapshot = owner.snapshot().unwrap();
        assert_eq!(snapshot.usage.frame_slots, 0);
        assert_eq!(snapshot.usage.serialized_buffer_bytes, 0);
    }
    #[test]
    fn relay_admits_only_the_provisioned_unexpired_outer_route() {
        let pair = Pair::provision(Plan {
            stage: Stage::Two,
            arm: Arm::Warm,
            size: 4096,
            trial: 0,
            bandwidth_profile: Default::default(),
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
        let control = &pair.sender.service.as_ref().unwrap().outgoing_control;
        let admission = RouteAdmission::with_control(material, control).unwrap();
        admission.validate(&routing, &control.key_id).unwrap();
        assert!(
            admission
                .validate(
                    &routing,
                    &pair
                        .receiver
                        .service
                        .as_ref()
                        .unwrap()
                        .outgoing_control
                        .key_id
                )
                .is_err()
        );
    }

    #[test]
    fn canonical_relay_forwards_all_four_envelopes_and_matches_endpoint_counters() {
        let pair = Pair::provision(Plan {
            stage: Stage::Two,
            arm: crate::config::Arm::Raw,
            size: 128,
            trial: 1,
            bandwidth_profile: Default::default(),
        })
        .unwrap();
        let (a, relay_a) = UnixStream::pair().unwrap();
        let (b, relay_b) = UnixStream::pair().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let forward = RouteAdmission::with_control(
            pair.sender.outgoing.as_ref().unwrap(),
            &pair.sender.service.as_ref().unwrap().outgoing_control,
        )
        .unwrap();
        let reverse = RouteAdmission::with_control(
            pair.receiver.outgoing.as_ref().unwrap(),
            &pair.receiver.service.as_ref().unwrap().outgoing_control,
        )
        .unwrap();
        std::thread::scope(|scope| {
            let sender = scope.spawn(|| {
                campaign::sender(&pair.sender, a, HandshakeRole::Initiator, &mut Vec::new())
                    .unwrap()
            });
            let receiver = scope.spawn(|| {
                campaign::receiver(
                    &pair.receiver,
                    &temp.path().join("cache"),
                    b,
                    HandshakeRole::Initiator,
                    &mut Vec::new(),
                )
                .unwrap()
            });
            let budget = transport_budget().unwrap();
            let scope = budget.scope(0).unwrap();
            let links = [
                budget.reserve_link(&scope).unwrap(),
                budget.reserve_link(&scope).unwrap(),
            ];
            let receipt =
                relay(relay_a, relay_b, forward, reverse, [&links[0], &links[1]]).unwrap();
            let sender = sender.join().unwrap();
            let receiver = receiver.join().unwrap();
            counters_agree(&sender["counters"], &receipt["a"]).unwrap();
            counters_agree(&receiver["counters"], &receipt["b"]).unwrap();
            drop(links);
            let snapshot = budget.snapshot().unwrap();
            assert_eq!(snapshot.usage.links, 0);
            assert_eq!(snapshot.usage.frame_slots, 0);
            assert_eq!(snapshot.usage.serialized_buffer_bytes, 0);
            assert_eq!(snapshot.high_water.links, 2);
            assert_eq!(snapshot.high_water.frame_slots, 2);
            assert_eq!(snapshot.high_water.serialized_buffer_bytes, 524288);
        });
    }
}

#[cfg(test)]
mod duplex_tests;
