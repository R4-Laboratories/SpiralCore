//! One-way baseline through the current canonical helper, without a custom port.
use crate::{Config, cleanup_after_launch, expected_body, receive_probe_offsets};
use anyhow::{Context, Result, ensure};
use cpu_time::ProcessTime;
use mvm_client::{LaunchRequest, LifecycleMode, LocalBackend, MvmClient, RootfsSource};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

pub const MVM_SOURCE: &str = "11c6f04bd1f6fa261b6543d678acf8226e699f33";
pub const PRODUCER_PATH: &str = "/usr/local/bin/spiralcore-mailbox-producer";
mod process {
    include!("process.rs");
}
use process::{HeldExecutable, ProcessObservation};
mod definition {
    include!("definition.rs");
}
pub use definition::DefinitionGuard;
mod installed;
use installed::InstalledWitness;
mod cast_echo;
pub mod cast_readback;
mod pair;
pub use pair::run_pair;
#[derive(Serialize)]
pub struct Report {
    pub source: &'static str,
    pub scope: &'static str,
    pub status: &'static str,
    pub error: Option<String>,
    pub plan_id: Option<String>,
    pub expected_bytes: usize,
    pub expected_sha256: String,
    pub received_bytes: Option<usize>,
    pub received_sha256: Option<String>,
    pub exact_readback: bool,
    pub setup_wall_ns: Option<u128>,
    pub receive_wall_ns: Option<u128>,
    pub receive_probes: usize,
    pub receive_wait_class: &'static str,
    pub lifecycle_wall_ns: u128,
    pub host_orchestrator_cpu_ns: u128,
    pub guest_cpu: &'static str,
    pub wire_bytes: &'static str,
    pub guest_mailbox_stream_scope: &'static str,
    pub guest_mailbox_stream_counts:
        Option<mvm_hostd::mailbox_service::guest_io::GuestStreamCounts>,
    pub verified_helper_receipt_wall_ns: Option<u128>,
    pub host_orchestrator_cpu_through_receipt_ns: Option<u128>,
    pub processes: Vec<ProcessObservation>,
    pub installed_helper: Option<String>,
    pub installed_daemon: Option<String>,
    pub stop: String,
    pub cleanup: String,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Build through the real facade; no VM is started by this function.
pub fn request(config: &Config) -> Result<LaunchRequest> {
    request_with_command(config, vec![PRODUCER_PATH.to_owned()])
}
fn request_with_command(config: &Config, command: Vec<String>) -> Result<LaunchRequest> {
    config.validate()?;
    mvm_client::validate_vm_name(&config.name)?;
    let grant = mvm_client::admission::mailbox_grant::resolve(Some(&config.mailbox_hex), None)?
        .context("requested mailbox grant unavailable")?;
    Ok(LaunchRequest::builder(
        LifecycleMode::Persistent,
        RootfsSource::LocalPath(config.rootfs.clone()),
    )
    .name(&config.name)
    .backend("firecracker")
    .cpus(1)
    .memory_mib(256)
    .ttl_seconds(180)
    .mailbox(grant)
    .command(command)
    .build()?)
}

pub fn validate_rootfs_input(path: &std::path::Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "rootfs input must not be a symlink"
    );
    if metadata.is_dir() {
        mvm_core::private_fs::require_current_owner(&metadata)?;
        ensure!(
            mvm_core::private_fs::mode_bits(path, &metadata)? == 0o700,
            "rootfs source directory must be privately owned 0700"
        );
    } else {
        ensure!(
            metadata.is_file(),
            "rootfs input must be a regular file or private directory"
        );
        use std::os::unix::fs::MetadataExt;
        ensure!(
            (metadata.uid() == 0 || metadata.uid() == rustix::process::geteuid().as_raw())
                && metadata.mode() & 0o022 == 0,
            "rootfs image must be root/current-user owned and protected from group/other writes"
        );
    }
    Ok(())
}

fn claim_fresh_home(config: &Config) -> Result<(PathBuf, HeldExecutable, PathBuf, fs::File)> {
    config.validate()?;
    ensure!(
        std::env::var_os("MVM_HOME").as_deref() == Some(config.home.as_os_str()),
        "MVM_HOME must equal the explicit isolated home"
    );
    let state = mvm_core::config::vm_state_dir_at(&config.home, &config.name);
    let home_meta = fs::symlink_metadata(&config.home)?;
    mvm_core::private_fs::require_current_owner(&home_meta)?;
    ensure!(
        home_meta.is_dir() && mvm_core::private_fs::mode_bits(&config.home, &home_meta)? == 0o700,
        "isolated home must be an owned non-symlink 0700 directory"
    );
    ensure!(
        fs::canonicalize(&config.home)? == config.home,
        "home must use its canonical path"
    );
    match fs::symlink_metadata(mvm_core::config::vms_dir_at(&config.home)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
        Ok(_) => anyhow::bail!("fresh isolated home must have no prior VM namespace"),
    }
    match fs::symlink_metadata(mvm_core::config::machine_state_root()) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
        Ok(_) => {
            anyhow::bail!("fresh isolated home must have no prior persistent machine namespace")
        }
    }
    ensure!(!state.exists(), "fresh name required");
    ensure!(!state.exists(), "refusing a name with existing VM state");
    validate_rootfs_input(&config.rootfs)?;
    let helper = PathBuf::from(
        std::env::var_os("MVM_MAILBOX_PATH")
            .context("explicit current MVM_MAILBOX_PATH required")?,
    );
    ensure!(
        helper.is_absolute() && fs::metadata(&helper)?.is_file(),
        "current mailbox helper missing"
    );
    let expected_helper = HeldExecutable::open(&helper)?;
    expected_helper.require_protected()?;
    let claim_path = config.home.join(".spiralcore-canonical-witness-owner");
    use std::os::unix::fs::OpenOptionsExt;
    let _claim = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&claim_path)?;
    Ok((helper, expected_helper, claim_path, _claim))
}

fn new_report(body: &[u8]) -> Report {
    Report {
        source: MVM_SOURCE,
        scope: "one canonical guest-to-host mailbox; no object-v1/WAN qualification",
        status: "failed",
        error: None,
        plan_id: None,
        expected_bytes: body.len(),
        expected_sha256: digest(&body),
        received_bytes: None,
        received_sha256: None,
        exact_readback: false,
        setup_wall_ns: None,
        receive_wall_ns: None,
        receive_probes: 0,
        receive_wait_class: "at most five bounded external probes; functional wait, not performance-qualified",
        lifecycle_wall_ns: 0,
        host_orchestrator_cpu_ns: 0,
        guest_cpu: "unavailable",
        wire_bytes: "unavailable; no virtio, NIC or WAN byte counters",
        guest_mailbox_stream_scope: mvm_hostd::mailbox_service::guest_io::SCOPE,
        guest_mailbox_stream_counts: None,
        verified_helper_receipt_wall_ns: None,
        host_orchestrator_cpu_through_receipt_ns: None,
        processes: Vec::new(),
        installed_helper: None,
        installed_daemon: None,
        stop: "not_attempted".into(),
        cleanup: "not_attempted".into(),
    }
}

pub async fn run(config: &Config) -> Result<Report> {
    let (helper, expected_helper, claim_path, _claim) = claim_fresh_home(config)?;
    let state = mvm_core::config::vm_state_dir_at(&config.home, &config.name);
    let body = expected_body();
    let started = Instant::now();
    let cpu = ProcessTime::now();
    let mut report = new_report(&body);
    let mut installed = InstalledWitness::start(config, &helper)?;
    let backend = LocalBackend::with_hypervisor("firecracker")
        .with_installed_mailbox(installed.provider.clone());
    let mut retained_client = None;
    let mut launched = None;
    let mut definition_guard = None;
    let operation = async {
        launched = Some(backend.launch(request(config)?).await?);
        let outcome = launched
            .as_ref()
            .expect("successful original launch retained");
        report.plan_id = Some(outcome.plan_id.clone());
        definition_guard = Some(DefinitionGuard::capture(
            &mvm_core::config::machine_spec_path(&outcome.machine.name),
        )?);
        report.setup_wall_ns = Some(started.elapsed().as_nanos());
        let receive_started = Instant::now();
        // These are the canonical per-VM process records, not lookup authority.
        let vmm_pid = mvm_backends::fc::fc_pid_path(&config.name)
            .context("canonical Firecracker pid path missing")?;
        report
            .processes
            .push(ProcessObservation::capture(&vmm_pid, None)?);
        let client = installed.original_owner(config)?;
        ensure!(
            matches!(
                client.query(Instant::now() + Duration::from_secs(2))?,
                mvm_core::protocol::mailbox_worker_control::MailboxWorkerState::Serving
            ),
            "original installed worker did not report Serving"
        );
        retained_client = Some(client);
        report
            .processes
            .push(installed.observe_helper(&expected_helper)?);
        receive_payload(config, &body, receive_started, &mut report)?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    let mut errors = Vec::new();
    if let Err(error) = operation {
        errors.push(format!("{error:#}"));
    }
    for process in &mut report.processes {
        if let Err(error) = process.sample_before_stop() {
            errors.push(format!("process sample: {error:#}"));
        }
    }
    // An unsuccessful launch provides no original ownership for name-based stop.
    let cleaned = clean_original(&backend, &launched, &definition_guard).await;
    report.stop = cleaned.stop.into();
    report.cleanup = cleaned.cleanup.into();
    if let Some(error) = cleaned.error {
        errors.push(error);
    }
    if launched.is_none() {
        errors.push("unowned partial-launch recovery unresolved; original state retained".into());
    }
    let exit_until = Instant::now() + Duration::from_secs(5);
    for process in &mut report.processes {
        if let Err(error) = process.observe_exit(exit_until) {
            errors.push(format!("{error:#}"));
        }
    }
    match fs::symlink_metadata(&state) {
        Ok(_) => errors.push("canonical runtime state remains after cleanup".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => errors.push(format!("cannot establish runtime-state removal: {error}")),
    }
    match fs::symlink_metadata(mvm_core::config::machine_state_dir(&config.name)) {
        Ok(_) => errors.push("canonical persistent definition remains after cleanup".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => errors.push(format!(
            "cannot establish persistent-definition removal: {error}"
        )),
    }
    if report.processes.len() != 2 {
        errors.push("both original process identities were not captured".into());
    }
    if let Some(original) = retained_client.as_ref() {
        match original.query(Instant::now() + Duration::from_secs(2)) {
            Ok(mvm_core::protocol::mailbox_worker_control::MailboxWorkerState::Collected {
                exit:
                    mvm_core::protocol::mailbox_worker_control::MailboxWorkerExit::Code { code: 0 },
            }) => {
                report.installed_helper =
                    Some("original signed owner collected helper exit 0".into())
            }
            other => errors.push(format!(
                "original installed helper exit unresolved: {other:?}"
            )),
        }
    }
    // Only the original helper received the writer for this channel. Its signed
    // exit settles the measurement even when the tenant still admits work.
    if report.installed_helper.is_some() {
        if let Some(owner) = retained_client.as_ref().and_then(|client| client.owner()) {
            let counts = mvm_hostd::mailbox_service::guest_io::private_channel::take_reader(
                &installed.report_socket,
                owner,
            )
            .and_then(|reader| {
                mvm_hostd::mailbox_service::guest_io::receive_original(&reader, owner)
            });
            match counts {
                Ok(counts)
                    if report.exact_readback
                        && counts.guest_to_host_read_bytes < body.len() as u64 =>
                {
                    errors.push("guest stream receipt below verified application length".into());
                }
                Ok(counts) => {
                    report.guest_mailbox_stream_counts = Some(counts);
                    report.verified_helper_receipt_wall_ns = Some(started.elapsed().as_nanos());
                    report.host_orchestrator_cpu_through_receipt_ns =
                        Some(cpu.elapsed().as_nanos());
                }
                Err(error) => errors.push(format!("guest stream receipt unavailable: {error}")),
            }
        } else {
            errors.push("guest stream receipt lacks original owner".into());
        }
    }
    match installed.finish() {
        Ok(status) => {
            report.installed_daemon = Some(format!("original Child::wait returned {status}"))
        }
        Err(error) => errors.push(format!(
            "installed coordinator shutdown unresolved: {error:#}"
        )),
    }
    report.lifecycle_wall_ns = started.elapsed().as_nanos();
    report.host_orchestrator_cpu_ns = cpu.elapsed().as_nanos();
    if errors.is_empty() {
        match fs::remove_file(&claim_path) {
            Ok(()) => report.status = "passed",
            Err(error) => report.error = Some(format!("witness claim removal failed: {error}")),
        }
    } else {
        report.error = Some(errors.join("; "));
    }
    Ok(report)
}

fn receive_payload(
    config: &Config,
    body: &[u8],
    receive_started: Instant,
    report: &mut Report,
) -> Result<()> {
    let until = receive_started + Duration::from_secs(config.deadline_seconds);
    for offset in receive_probe_offsets(Duration::from_secs(config.deadline_seconds)) {
        let target = receive_started + offset;
        if let Some(wait) = target.checked_duration_since(Instant::now()) {
            thread::sleep(wait);
        }
        ensure!(Instant::now() < until, "canonical mailbox receive deadline");
        report.receive_probes += 1;
        let received = mvm_client::mailbox::recv(&config.name);
        ensure!(
            Instant::now() < until,
            "canonical receive completed after original deadline"
        );
        match received {
            Ok(Some(actual)) => {
                report.received_bytes = Some(actual.len());
                report.received_sha256 = Some(digest(&actual));
                report.exact_readback = actual.as_slice() == body;
                ensure!(report.exact_readback, "canonical mailbox bytes differ");
                break;
            }
            Ok(None) => {}
            Err(error) => {
                let unavailable = error.to_string()
                    == format!("{} has no reachable mailbox", config.name)
                    && error
                        .chain()
                        .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
                        .any(|io| {
                            matches!(
                                io.kind(),
                                std::io::ErrorKind::NotFound
                                    | std::io::ErrorKind::ConnectionRefused
                            )
                        });
                if !unavailable {
                    return Err(error.context("canonical audited receive"));
                }
            }
        }
    }
    ensure!(
        report.exact_readback,
        "bounded canonical receive probes exhausted without verified delivery"
    );
    report.receive_wall_ns = Some(receive_started.elapsed().as_nanos());
    Ok(())
}

async fn clean_original(
    backend: &LocalBackend,
    launched: &Option<mvm_client::launch::LaunchOutcome>,
    definition_guard: &Option<DefinitionGuard>,
) -> crate::CleanupResult {
    cleanup_after_launch(
        launched.as_ref(),
        async {
            let original = launched.as_ref().context("original launch absent")?;
            let plan = mvm_client::audit::chain::plan_persist::read_plan(&original.machine.name)?;
            ensure!(
                plan.plan_id.0 == original.plan_id && plan.workload.0 == original.machine.name,
                "persisted admission differs from original launch; stop not authorized"
            );
            definition_guard
                .as_ref()
                .context("original definition fence unavailable; retain state")?
                .verify()?;
            backend
                .stop_machine(&original.machine.id)
                .await
                .map_err(anyhow::Error::from)
        },
        async {
            let original = launched.as_ref().context("original launch absent")?;
            definition_guard
                .as_ref()
                .context("original definition fence unavailable; retain state")?
                .verify()?;
            backend
                .remove_machine_with(
                    &original.machine.id,
                    mvm_client::RemoveOptions { force: false },
                )
                .await
                .map_err(anyhow::Error::from)
        },
    )
    .await
}
