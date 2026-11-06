use anyhow::{Context, Result, ensure};
use ed25519_dalek::SigningKey;
use mvm_client::guest::{ProcStart, ProcWaitEvent, start_process, wait_process};
use mvm_core::{
    crypto::image_verify::sha256_file,
    plan::{
        NetworkMode, PlanSeccompTier, SecretReleasePolicy, SynthesisInput, sign_plan,
        synthesize_plan,
    },
    user_config::{MvmConfig, TrustedPlanSigner},
};
use mvm_hostd::{
    plan_admission::{InMemoryNonceLedger, SystemClock},
    run::{LocalRunContext, LocalRunRequest, admit_and_boot_local},
};
use mvm_runtime::backend::AnyBackend;
use std::{path::PathBuf, time::Instant};

const NAME: &str = "sc-enforce4";
const PROBE: &str = "/bin/spiralcore-seccomp-probe";

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let rootfs = PathBuf::from(args.next().context("missing workload rootfs")?).canonicalize()?;
    let kernel =
        PathBuf::from(args.next().context("missing Firecracker kernel")?).canonicalize()?;
    ensure!(args.next().is_none());
    let home = mvm_core::config::mvm_home_strict()?;
    ensure!(home == PathBuf::from("/var/tmp/spiralcore-guest-enforcement-20261105/mvm"));
    ensure!(!mvm_core::config::vm_state_dir(NAME).exists());
    let host_signer = mvm_hostd::audit::host_keypair::load_or_init()?;
    let anchor = home.join("keys/host-signer.pub");
    ensure!(host_signer.public_path == anchor);
    ensure!(std::fs::read(&anchor)? == host_signer.verifying.as_bytes());
    drop(host_signer);
    let roothash = mvm_build::rootfs::emit_verity_sidecars(&rootfs)?;
    let verity = rootfs
        .parent()
        .context("workload parent missing")?
        .join("rootfs.verity");
    let rootfs_sha = sha256_file(&rootfs)?;
    let kernel_sha = sha256_file(&kernel)?;
    let signer = SigningKey::from_bytes(&rand::random());
    let signer_id = "spiralcore-private-agent-witness";
    let public_key: String = signer
        .verifying_key()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    mvm_core::user_config::save(
        &MvmConfig {
            trusted_plan_signers: vec![TrustedPlanSigner {
                signer_id: signer_id.into(),
                ed25519_pubkey_hex: public_key,
            }],
            ..MvmConfig::default()
        },
        None,
    )?;
    let input = SynthesisInput::builder()
        .vm_name(NAME)
        .tenant("local")
        .backend_name("firecracker")
        .image_name(NAME)
        .image_sha256(&rootfs_sha)
        .kernel_sha256(kernel_sha.as_str())
        .seccomp_tier(PlanSeccompTier::Vsock)
        .secret_release(SecretReleasePolicy::None)
        .network_mode(NetworkMode::HostVsockProxy)
        .cpus(1)
        .mem_mib(1024)
        .disk_mib(0)
        .boot_timeout_secs(60)
        .destroy_on_exit(true)
        .secrets(Vec::new())
        .shares(Vec::new())
        .redaction(Default::default())
        .reversible_replacement(Default::default())
        .audit_labels(Default::default())
        .services(Vec::new())
        .stream_edges(Vec::new())
        .stream_retention(Default::default())
        .build()?;
    let plan = synthesize_plan(&input)?;
    let signed = sign_plan(&plan, &signer, signer_id);
    let request = LocalRunRequest {
        name: NAME.into(),
        rootfs_path: rootfs,
        kernel_path: Some(kernel),
        verity_path: Some(verity),
        roothash: Some(roothash),
        cpus: 1,
        mem_mib: 1024,
        backend_name: "firecracker".into(),
        volumes: Vec::new(),
        destroy_on_exit: true,
        grants: None,
        secrets: Vec::new(),
        secret_release: SecretReleasePolicy::None,
        signed_plan: Some(signed),
        registry_pack_image: None,
    };
    let backend = AnyBackend::from_hypervisor("firecracker");
    let ledger = InMemoryNonceLedger::new();
    let start = Instant::now();
    let started = admit_and_boot_local(
        &backend,
        &request,
        LocalRunContext {
            clock: &SystemClock,
            ledger: &ledger,
            host_signer_keys_dir: None,
            emitter: None,
            assurance: None,
        },
    )?;
    let boot = start.elapsed();
    let exchange = (|| -> Result<serde_json::Value> {
        ensure!(started.admitted.plan_id() == &plan.plan_id);
        let token = start_process(
            NAME,
            ProcStart {
                argv: vec![PROBE.into()],
                env: Default::default(),
                cwd: None,
                allow_env: mvm_core::env_hygiene::EnvReadmit::none(),
            },
        )?;
        let mut stdout = Vec::new();
        let mut refused = false;
        let event = wait_process(NAME, &token, Some(20), |event| match event {
            ProcWaitEvent::Stdout { chunk }
                if chunk.len() <= 1024usize.saturating_sub(stdout.len()) =>
            {
                stdout.extend_from_slice(chunk)
            }
            ProcWaitEvent::Stdout { .. }
            | ProcWaitEvent::Stderr { .. }
            | ProcWaitEvent::Backpressure { .. } => refused = true,
            _ => {}
        })?;
        ensure!(
            !refused && matches!(event, ProcWaitEvent::Exit { code: 0 }),
            "direct probe terminal {event:?}"
        );
        let report: serde_json::Value = serde_json::from_slice(&stdout)?;
        ensure!(report == serde_json::json!({"seccomp":2,"af_inet":"EPERM","af_vsock":"created"}));
        Ok(report)
    })();
    backend
        .stop(&started.vm_id)
        .context("stop original Firecracker launch")?;
    let report = exchange?;
    println!(
        "{}",
        serde_json::json!({
            "report": report, "plan_id": plan.plan_id.0, "signed_seccomp_tier": "vsock",
            "rootfs_sha256": rootfs_sha, "kernel_sha256": kernel_sha,
            "boot_elapsed_us": boot.as_micros(), "through_stop_elapsed_us": start.elapsed().as_micros(),
        })
    );
    Ok(())
}
