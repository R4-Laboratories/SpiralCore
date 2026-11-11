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

pub const MVM_SOURCE: &str = "18b58e1ad2262a1c0bfa23d51f8e5679d3376f60";
pub const PRODUCER_PATH: &str = "/usr/local/bin/spiralcore-mailbox-producer";
mod process {
    include!("process.rs");
}
use process::{HeldExecutable, ProcessObservation};
mod definition {
    include!("definition.rs");
}
pub use definition::DefinitionGuard;
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
    pub processes: Vec<ProcessObservation>,
    pub stop: String,
    pub cleanup: String,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Build through the real facade; no VM is started by this function.
pub fn request(config: &Config) -> Result<LaunchRequest> {
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
    .command([PRODUCER_PATH.to_owned()])
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

pub async fn run(config: &Config) -> Result<Report> {
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
    let body = expected_body();
    let started = Instant::now();
    let cpu = ProcessTime::now();
    let mut report = Report {
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
        wire_bytes: "unavailable; not inferred from payload",
        processes: Vec::new(),
        stop: "not_attempted".into(),
        cleanup: "not_attempted".into(),
    };
    let backend = LocalBackend::with_hypervisor("firecracker");
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
        let until = receive_started + Duration::from_secs(config.deadline_seconds);
        // These are the canonical per-VM process records, not lookup authority.
        let vmm_pid = mvm_backends::fc::fc_pid_path(&config.name)
            .context("canonical Firecracker pid path missing")?;
        report
            .processes
            .push(ProcessObservation::capture(&vmm_pid, None)?);
        let mailbox_pid = state.join(mvm_vmm::host::vm_helpers::MAILBOX_PID_FILE);
        report.processes.push(ProcessObservation::capture(
            &mailbox_pid,
            Some(&expected_helper),
        )?);
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
                    report.exact_readback = actual == body;
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
    let cleaned = cleanup_after_launch(
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
    .await;
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
