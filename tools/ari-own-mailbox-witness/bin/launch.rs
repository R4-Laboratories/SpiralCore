use anyhow::{Context, Result, ensure};
use mvm_client::entrypoint::dispatch::{self, CapturedOutput, DispatchStdin, EntrypointDispatch};
use mvm_client::{LaunchRequest, LifecycleMode, LocalBackend, MvmClient, RootfsSource};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const NAME: &str = "sc-ari-own";
fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).map(PathBuf::from).collect();
    if let [cmd, frame] = args.as_slice()
        && cmd == Path::new("call")
    {
        let input = fs::read(frame)?;
        ensure!(input.len() <= 16 * 1024, "input exceeds test limit");
        let mut observation = CapturedOutput::default();
        let completed = dispatch::dispatch(
            EntrypointDispatch {
                vm_name: NAME,
                stdin: DispatchStdin::OneShot(input),
                timeout_secs: 45,
                session_id: None,
            },
            &mut observation,
        )?;
        ensure!(
            completed.exit_code() == 0 && !observation.truncated,
            "workload call ended {:?}; bounded output stderr: {}",
            completed.terminal,
            String::from_utf8_lossy(&observation.stderr[..observation.stderr.len().min(400)])
        );
        ensure!(
            observation.stdout.len() <= 8192,
            "workload response too large"
        );
        std::io::stdout().write_all(&observation.stdout)?;
        return Ok(());
    }
    let [root, frame, report, digest] = args.as_slice() else {
        anyhow::bail!(
            "expected verified image, encoded frame, private report directory and preverified root digest"
        );
    };
    let home = PathBuf::from(env::var_os("MVM_HOME").context("MVM_HOME")?);
    ensure!(
        home.is_absolute()
            && home.canonicalize()? == home
            && !home.join("vms").exists()
            && !mvm_core::config::machine_spec_path(NAME).exists(),
        "fresh VM namespace required"
    );
    ensure!(
        mvm_core::user_config::load_strict(None)?.max_mailbox_retention_ms == Some(30_000),
        "host ceiling differs"
    );
    let grant =
        mvm_client::admission::mailbox_grant::resolve(Some(&"43".repeat(32)), Some(10_000), None)?
            .context("scope absent")?;
    let selected = mvm_core::crypto::image_verify::sha256_file(frame)?;
    ensure!(
        selected == "ff26a66df5b1d9de994e60824757686d2af8eb4433b91e74a2f24b279f600044"
            && fs::metadata(frame)?.len() == 313,
        "synthetic pinned frame differs"
    );
    ensure!(
        mvm_core::crypto::image_verify::sha256_file(root)?
            == digest.to_str().context("invalid operator root digest")?,
        "sealed operator root differs"
    );
    let request = LaunchRequest::builder(
        LifecycleMode::Persistent,
        RootfsSource::LocalPath(root.canonicalize()?),
    )
    .profile("standard")
    .name(NAME)
    .backend("firecracker")
    .cpus(1)
    .memory_mib(256)
    .ttl_seconds(150)
    .mailbox(grant)
    .build()?;
    let backend = LocalBackend::with_hypervisor("firecracker");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let started = Instant::now();
    let outcome = rt.block_on(backend.launch(request))?;
    let setup = started.elapsed().as_nanos();
    let entered = (|| -> Result<_> {
        let posture = outcome
            .admitted
            .mailbox_posture()
            .context("missing boot authority")?;
        ensure!(
            format!("{:?}", posture.variant()) == "Prod"
                && outcome.admitted.plan().mailbox == Some(grant),
            "production posture or original retention differs"
        );
        let signed_boot = mvm_core::config::vm_state_dir(NAME).join("mailbox-reader-boot.json");
        ensure!(signed_boot.exists(), "original boot reader record absent");
        posture.verify(outcome.admitted.plan(), NAME)?;
        let mut child = Command::new(env::current_exe()?)
            .args(["call"])
            .arg(frame)
            .stdout(Stdio::from(fs::File::create_new(
                report.join("workload-output.json"),
            )?))
            .stderr(Stdio::from(fs::File::create_new(
                report.join("workload-error.log"),
            )?))
            .spawn()?;
        let call = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if call.elapsed() >= Duration::from_secs(55) {
                child.kill()?;
                child.wait()?;
                anyhow::bail!("canonical workload call exceeded host observation deadline");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        ensure!(status.success(), "canonical workload call failed: {status}");
        let payload: serde_json::Value =
            serde_json::from_slice(&fs::read(report.join("workload-output.json"))?)?;
        ensure!(
            payload["profile"] == "mvm_own_mailbox_guest_v1"
                && payload["body_bytes"] == 257
                && payload["frame_bytes"] == 313
                && payload["body_sha256"]
                    == "54acfbfedc4d8da40f76f275e1a98f10af8ef1fb9fb39e5a67a00aabcbe6597c"
                && payload["message_id"]
                    .as_str()
                    .is_some_and(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()))
                && [
                    "receive_request_bytes",
                    "receive_response_bytes",
                    "ack_request_bytes",
                    "ack_response_bytes"
                ]
                .iter()
                .all(|key| payload[*key].as_u64().is_some_and(|n| n > 0)),
            "original readback, request or ACK measurement differs"
        );
        Ok(
            serde_json::json!({"signed_plan_id":outcome.plan_id,"setup_ns":setup,"call_wall_ns":call.elapsed().as_nanos(),"boot_binding_present":signed_boot.exists(),"guest":payload}),
        )
    })();
    let stop = rt.block_on(backend.stop_machine(&outcome.machine.id));
    if let Err(error) = stop {
        anyhow::bail!("original stop failed: {error}; call={entered:?}");
    }
    rt.block_on(backend.remove_machine(&outcome.machine.id))?;
    let measured = entered?;
    println!(
        "{}",
        serde_json::json!({"result":measured,"including_stop_ns":started.elapsed().as_nanos()})
    );
    Ok(())
}
