//! One installed native guest witness. The caller owns the measured startup
//! budget, original admission and the verified host-agent enrollment.
use super::*;
use anyhow::{Context, bail, ensure};
use mvm_client::guest::{ProcWaitEvent, wait_process};
use mvm_core::plan::ExecutionPlan;
use std::{
    net::Ipv4Addr,
    os::unix::fs::MetadataExt as _,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

mod native_peer;
mod native_plan;

fn launched_process<'a>(
    service: &'a InstalledResident,
    id: ResidentId,
    vm_name: &str,
) -> Result<(&'a str, &'a str)> {
    let live = service
        .live
        .as_ref()
        .context("original resident unavailable")?;
    let State::Running(outcome) = &live.slot(id)?.state else {
        bail!("original native guest did not reach running");
    };
    ensure!(outcome.machine.name == vm_name);
    Ok((
        &outcome.machine.name,
        outcome
            .process
            .as_deref()
            .context("guest readback process missing")?,
    ))
}

fn read_guest_marker(name: &str, process: &str) -> Result<()> {
    let expected = b"spiralcore-guest-readback-exact\n";
    let mut marker = Vec::with_capacity(expected.len());
    let mut extra = false;
    let terminal = wait_process(name, process, Some(15), |event| match event {
        ProcWaitEvent::Stdout { chunk }
            if chunk.len() <= expected.len().saturating_sub(marker.len()) =>
        {
            marker.extend_from_slice(chunk);
        }
        ProcWaitEvent::Stdout { .. } | ProcWaitEvent::Stderr { .. } => extra = true,
        _ => {}
    })?;
    ensure!(
        matches!(terminal, ProcWaitEvent::Exit { code: 0 }),
        "guest readback process failed: {terminal:?}"
    );
    ensure!(
        !extra && marker == expected,
        "guest did not report exact readback"
    );
    Ok(())
}

struct NativeCredit(Option<ManagedReservation>);

impl NativeCredit {
    fn release(&mut self) {
        drop(self.0.take());
    }
}

impl Drop for NativeCredit {
    fn drop(&mut self) {
        if let Some(credit) = self.0.take() {
            // An unfinished native phase keeps its charge until process exit.
            std::mem::forget(credit);
        }
    }
}

fn run_roundtrip(
    service: &mut InstalledResident,
    runtime: &tokio::runtime::Runtime,
    id: ResidentId,
    plan: &ExecutionPlan,
    peer: native_peer::NativePeer,
    host_public: [u8; 32],
    disk: ManagedReservation,
    root: &Path,
) -> Result<(native_peer::NativePeerResult, u64)> {
    let grant = service
        .live
        .as_ref()
        .context("original resident unavailable")?
        .slot(id)?
        .owner
        .grant
        .clone();
    let vm_name = grant.binding.vm_name.clone();
    let state = mvm_core::config::vm_state_dir(&vm_name);
    let deadline = Instant::now() + Duration::from_secs(75);
    let cancelled = AtomicBool::new(false);
    let mut disk = NativeCredit(Some(disk));

    let (guest, peer, unused) = thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let session = loop {
                if cancelled.load(Ordering::Acquire) {
                    bail!("resident launch failed before original peer enrollment");
                }
                if let Some(session) = grant
                    .lifecycle
                    .lock()
                    .map_err(|_| anyhow!("original VM lifecycle poisoned"))?
                    .admitted_session
                {
                    break session;
                }
                ensure!(
                    Instant::now() < deadline,
                    "original endpoint session missing"
                );
                thread::sleep(Duration::from_millis(5));
            };
            peer.enroll(plan, &vm_name, state, session, host_public)?
                .serve(deadline)
        });
        let boot = runtime.block_on(service.drive_once());
        let unused = boot.as_ref().is_ok_and(|progress| {
            progress.id == id && progress.state == ResidentStatus::UnusedRetired
        });
        let guest = (|| {
            let progress = boot?;
            ensure!(
                progress.id == id && progress.state == ResidentStatus::Running,
                "original native launch failed: {:?}, diagnostics={:?}",
                progress.state,
                service.last_error(id).ok().flatten()
            );
            let (name, process) = launched_process(service, id, &vm_name)?;
            read_guest_marker(name, process)
        })();
        if guest.is_err() {
            cancelled.store(true, Ordering::Release);
        }
        let peer = worker
            .join()
            .map_err(|_| anyhow!("native peer panicked"))
            .and_then(|result| result);
        (guest, peer, unused)
    });
    drop(grant);

    // Stop through the original owner even if either half of transfer failed.
    // Do not treat an uncertain cleanup as success or release its custody.
    let stopped = (|| {
        if unused {
            return Ok(());
        }
        service.request_stop(id)?;
        let progress = runtime.block_on(service.drive_once())?;
        ensure!(
            progress.id == id && progress.state == ResidentStatus::Retired,
            "original native retirement failed: {:?}, diagnostics={:?}",
            progress.state,
            service.last_error(id).ok().flatten()
        );
        Ok::<(), anyhow::Error>(())
    })();
    if let Err(error) = &guest {
        eprintln!("guest failure before original shutdown: {error:#}");
    }
    if let Err(error) = &peer {
        eprintln!("peer failure before original shutdown: {error:#}");
    }
    if guest.is_ok()
        && let Ok(peer) = &peer
    {
        eprintln!(
            "byte-exact guest and peer readback: sent={} received={}",
            peer.sent_bytes, peer.received_bytes
        );
        eprintln!("exact peer measurements: {peer:?}");
    }
    stopped?;
    let runtime_allocated = if guest.is_ok() && peer.is_ok() && !unused {
        ensure!(
            std::fs::read_dir(root.join("tmp"))?
                .next()
                .transpose()?
                .is_none(),
            "fixture staging remains after stop"
        );
        retained_runtime_bytes(root, &vm_name)?
    } else {
        0
    };
    if runtime_allocated != 0 || unused {
        disk.release();
    }
    if !unused {
        eprintln!("original native and endpoint retirement confirmed");
    }
    match (guest, peer) {
        (Ok(()), Ok(peer)) => Ok((peer, runtime_allocated)),
        (Err(error), _) | (_, Err(error)) => {
            if let Err(cleanup) = service.try_finish() {
                eprintln!("original shutdown after launch failure: {cleanup:#}");
            }
            Err(error)
        }
    }
}

fn finish_with_sealed_telemetry(service: &mut InstalledResident, parents: &[u64]) -> Result<u64> {
    use crate::{
        installed_retained::{FIXED_DISK_BYTES, RECORD_BYTES, SIDECAR_BYTES},
        installed_supervisor::SupervisorError,
    };
    let inventory = service
        .supervisor
        .as_ref()
        .context("supervisor missing")?
        .retained_telemetry_inventory()?;
    let mut sealed = 0;
    for parent in parents {
        sealed += inventory
            .witness_sealed_row(*parent, parents.len())
            .context("verify the sealed original telemetry row")?;
    }
    drop(inventory);
    let stopped = service.try_finish();
    ensure!(
        service.live.is_none(),
        "original helper or resident still live"
    );
    match stopped {
        Err(error)
            if matches!(
                error.downcast_ref::<SupervisorError>(),
                Some(SupervisorError::Busy)
            ) => {}
        other => bail!("expected sealed telemetry to retain original installation: {other:?}"),
    }
    let snapshot = service.usage()?;
    // A successful snapshot already ruled out a corrupt ledger. Its
    // compatibility field marks admission closed after try_finish.
    ensure!(
        snapshot.fenced && snapshot.active_reservations == 1 + 2 * parents.len(),
        "post-retirement managed snapshot differs: {snapshot:?}"
    );
    ensure!(
        snapshot.usage
            == ManagedClaim {
                host_metadata_bytes: MIB,
                cache_disk_bytes: FIXED_DISK_BYTES
                    + (RECORD_BYTES + SIDECAR_BYTES + RETAINED_RUNTIME_QUOTE)
                        * parents.len() as u64,
                ..Default::default()
            },
        "non-retained native credit remains: {:?}",
        snapshot.usage
    );
    Ok(sealed)
}

const MIB: u64 = 1024 * 1024;
const RETAINED_RUNTIME_QUOTE: u64 = 112 * MIB;

fn retained_runtime_bytes(root: &Path, name: &str) -> Result<u64> {
    let vm = root.join("mvm/runtime-evidence").join(name);
    let mut generation = None;
    for item in std::fs::read_dir(&vm)? {
        let item = item?;
        if item.file_name() == "lifecycle.lock" {
            continue;
        }
        ensure!(
            generation.is_none()
                && item.file_type()?.is_dir()
                && item
                    .file_name()
                    .to_string_lossy()
                    .starts_with("portable-runtime-"),
            "unexpected retained runtime generation"
        );
        generation = Some(item.path());
    }
    let generation = generation.context("no retained runtime generation")?;
    let mut blocks = 0_u64;
    let mut charge = |path: &Path, directory: bool| -> Result<()> {
        let metadata = std::fs::symlink_metadata(path)?;
        mvm_core::private_fs::require_current_owner(&metadata)?;
        ensure!(
            if directory {
                metadata.is_dir() && metadata.mode() & 0o7777 == 0o700
            } else {
                metadata.is_file() && metadata.mode() & 0o7777 == 0o400 && metadata.nlink() == 1
            },
            "retained runtime object differs"
        );
        blocks = blocks
            .checked_add(metadata.blocks())
            .context("runtime blocks overflow")?;
        Ok(())
    };
    charge(&vm, true)?;
    charge(&generation, true)?;
    let mut count = 0;
    for file in std::fs::read_dir(&generation)? {
        let file = file?;
        ensure!(
            count < 7
                && matches!(
                    file.file_name().to_str(),
                    Some(
                        "bundle.mvmpkg"
                            | "initramfs.cpio.gz"
                            | "kernel"
                            | "overlay.ext4"
                            | "overlay.verity"
                            | "rootfs"
                            | "rootfs.verity"
                    )
                ),
            "unexpected retained runtime file"
        );
        charge(&file.path(), false)?;
        count += 1;
    }
    ensure!(count == 7, "incomplete retained runtime generation");
    let bytes = blocks
        .checked_mul(512)
        .context("runtime disk bytes overflow")?;
    ensure!(
        bytes > 0 && bytes <= RETAINED_RUNTIME_QUOTE,
        "retained runtime quote exceeded"
    );
    Ok(bytes)
}

fn quote(key: &str, cap: u64) -> Result<u64> {
    let value: u64 = std::env::var(key)?.parse()?;
    ensure!(
        value > 0 && value <= cap,
        "native fixture quote out of range"
    );
    Ok(value)
}

#[test]
#[ignore = "requires the pinned Firecracker installation and measured startup quotes"]
fn native_signed_guest_roundtrip_and_original_retirement() -> Result<()> {
    use crate::{
        installed_owner::{InstallationKey, InstallationOwner},
        installed_supervisor::InstalledConfig,
    };
    use mvm_core::host_lifetime::EndpointPartitionLimits;
    use mvm_core::user_config::TrustedPlanSigner;

    ensure!(std::env::var("SPIRALCORE_RUN_INSTALLED_WITNESS")? == "1");
    let root = PathBuf::from(
        std::env::var_os("SPIRALCORE_NATIVE_ASSET_ROOT").context("fixture root missing")?,
    );
    let runner_home = PathBuf::from(std::env::var_os("HOME").context("runner home missing")?);
    ensure!(root.parent() == Some(runner_home.canonicalize()?.as_path()));
    ensure!(
        root.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".scw-"))
    );
    ensure!(root.canonicalize()? == root);
    let home = mvm_core::config::mvm_home_strict()?;
    ensure!(home == root.join("mvm"));
    let names: &[&str] = match std::env::var("SPIRALCORE_NATIVE_WITNESS_GUESTS").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("1") => &[native_plan::NAME],
        Ok("2") => &[native_plan::NAME, "spiralcore-installed-witness-b"],
        _ => bail!("native witness admits one or two sequential guests"),
    };
    for name in names {
        ensure!(!mvm_core::config::vm_state_dir(name).exists());
    }
    let bind: Ipv4Addr = std::env::var("SPIRALCORE_NATIVE_PRIVATE_IP")?.parse()?;
    let startup_bytes = quote("SPIRALCORE_NATIVE_STARTUP_RESIDENT_BYTES", 160 * MIB)?;
    let disk_bytes = quote("SPIRALCORE_NATIVE_STARTUP_DISK_BYTES", 224 * MIB)?;

    let elapsed = Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let config = InstalledConfig::canonical_native_profile();
    let owner = InstallationOwner::initialize(
        &root.join("installation"),
        InstallationKey {
            installation: [83; 32],
            profile: config.profile_hash(),
        },
    )?;
    let supervisor = InstalledSupervisor::start(owner, config)?;
    let mut service =
        InstalledResident::prepare(supervisor, LocalBackend::with_hypervisor("firecracker"))?;
    let operator = mvm_core::config::mvm_config_dir_at(&home);
    ensure!(std::fs::read(operator.join("config.toml"))? == b"[egress_proxy]\n");
    let issuer = mvm_hostd::audit::host_keypair::load_or_init_at(&root.join("fleet-plan-issuer"))?;
    const FLEET_SIGNER: &str = "spiralcore-native-sdk-fixture";
    let mut host_policy = mvm_core::user_config::load(None);
    ensure!(host_policy.trusted_plan_signers.is_empty());
    host_policy.trusted_plan_signers.push(TrustedPlanSigner {
        signer_id: FLEET_SIGNER.into(),
        ed25519_pubkey_hex: issuer
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    });
    mvm_core::user_config::save(&host_policy, None)?;
    let keys = mvm_core::config::mvm_keys_dir();
    let signer = mvm_hostd::audit::host_keypair::load_or_init_at(&keys)?;
    let host_public = signer.verifying_key().to_bytes();
    drop(signer);

    let limits = EndpointPartitionLimits {
        host_plaintext_bytes: MIB,
        host_codec_bytes: 8 * MIB,
        host_carrier_bytes: 8 * MIB,
        host_metadata_bytes: 70 * MIB,
        host_stack_bytes: 24 * MIB,
        cache_disk_bytes: 32 * MIB,
        staging_disk_bytes: 16 * MIB,
        workers: 7,
        max_streams: 1,
    };
    let supervisor = service.supervisor.as_ref().unwrap();
    let live = service.live.as_mut().unwrap();
    let provider = Arc::get_mut(&mut live.provider).context("provider prematurely published")?;
    provider.configure_endpoints(
        supervisor,
        super::super::endpoint::QualifiedEndpointProfile::fixture(limits),
    )?;
    provider.configure_host_agent(
        supervisor,
        "local",
        super::super::host_agent::QualifiedHostAgentProfile::fixture_with_cold_operator(
            ManagedClaim {
                host_plaintext_bytes: 40 * MIB,
                host_metadata_bytes: 12 * MIB,
                host_stack_bytes: 32 * MIB,
                workers: 2,
                ..Default::default()
            },
            operator,
            Some("on".into()),
        ),
    )?;
    let pool = provider.shared.pool.clone();
    mvm_hostd::host_lifetime_parent::install(live.provider.clone())?;
    service.start_owned_host_agent()?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut retained = Vec::with_capacity(names.len());
    let mut parents = Vec::with_capacity(names.len());
    let mut results = Vec::with_capacity(names.len());
    for (index, &name) in names.iter().enumerate() {
        let guest_elapsed = Instant::now();
        let value = u8::try_from(index + 1)?;
        let peer = native_peer::NativePeer::bind(bind, value)?;
        let policy = peer.network_policy()?;
        let selection = pool.reserve(
            tenant_tag("local"),
            ManagedClaim {
                host_metadata_bytes: MIB,
                ..Default::default()
            },
        )?;
        let native = native_plan::NativeRequest::verified(&root, peer.address(), name, value)?;
        let peer_plan = native.plan(&policy)?;
        let request = native.request(mvm_core::plan::sign_plan(
            &peer_plan,
            &issuer.signing,
            FLEET_SIGNER,
        ))?;
        let mapping = policy
            .payload_mapping()
            .context("fixture peer mapping missing")?
            .clone();
        drop(selection);
        // Verify the endpoint and boot tail fit while admission owns the token.
        // The original grant funds startup separately until Runtime completes it.
        let prepare = || {
            let disk = pool.reserve(
                tenant_tag("local"),
                ManagedClaim {
                    host_metadata_bytes: 32 * MIB,
                    staging_disk_bytes: disk_bytes,
                    ..Default::default()
                },
            )?;
            let endpoint = pool.reserve(
                tenant_tag("local"),
                ManagedClaim {
                    host_plaintext_bytes: limits.host_plaintext_bytes,
                    host_codec_bytes: limits.host_codec_bytes,
                    host_carrier_bytes: limits.host_carrier_bytes,
                    host_metadata_bytes: limits.host_metadata_bytes,
                    host_stack_bytes: limits.host_stack_bytes,
                    cache_disk_bytes: limits.cache_disk_bytes,
                    staging_disk_bytes: limits.staging_disk_bytes,
                    workers: u64::from(limits.workers),
                    ..Default::default()
                },
            )?;
            drop(endpoint);
            let retained_runtime = pool.reserve(
                tenant_tag("local"),
                ManagedClaim {
                    cache_disk_bytes: RETAINED_RUNTIME_QUOTE,
                    ..Default::default()
                },
            )?;
            Ok::<_, anyhow::Error>((startup_bytes, (disk, retained_runtime)))
        };
        let (_startup_quote, (disk, retained_runtime)) = prepare()?;
        let admission_credit = pool.reserve(
            tenant_tag("local"),
            ManagedClaim {
                host_metadata_bytes: 160 * MIB,
                ..Default::default()
            },
        )?;
        let id = match service.begin_launch_with_selection(request, "local", Some(mapping)) {
            Ok(id) => id,
            Err(error) => {
                if let Err(cleanup) = service.try_finish() {
                    eprintln!("original shutdown after scheduling refusal: {cleanup:#}");
                }
                return Err(error);
            }
        };
        let mut admission_credit = NativeCredit(Some(admission_credit));
        let admission = guest_elapsed.elapsed();

        // This bounded fixture keeps the retained charge even after the native
        // reader exits. Reopening or reclaiming it needs product-level custody.
        retained.push(NativeCredit(Some(retained_runtime)));
        let (result, runtime_allocated) = run_roundtrip(
            &mut service,
            &runtime,
            id,
            &peer_plan,
            peer,
            host_public,
            disk,
            &root,
        )?;
        admission_credit.release();
        ensure!(result.received_bytes == 11_600 && result.sent_bytes == 11_600);
        let snapshot = service.usage()?;
        use crate::installed_retained::{FIXED_DISK_BYTES, RECORD_BYTES, SIDECAR_BYTES};
        let expected_cache = FIXED_DISK_BYTES
            + (RETAINED_RUNTIME_QUOTE + RECORD_BYTES + SIDECAR_BYTES) * u64::from(value);
        ensure!(
            !snapshot.fenced
                && snapshot.usage.cache_disk_bytes == expected_cache
                && snapshot.usage.guests == 0
                && snapshot.usage.guest_memory_bytes == 0
                && snapshot.usage.staging_disk_bytes == 0,
            "inter-guest ledger did not drain: {snapshot:?}"
        );
        parents.push(id.serial);
        let row = serde_json::json!({
            "name": name,
            "grant_serial": id.serial,
            "peer": result,
            "guest_readback_exact": true,
            "original_retirement": true,
            "admission_elapsed_ns": admission.as_nanos(),
            "wall_with_retirement_ns": guest_elapsed.elapsed().as_nanos(),
            "retained_runtime_allocated_bytes": runtime_allocated,
            "cache_disk_after_retirement": snapshot.usage.cache_disk_bytes,
            "staging_disk_after_retirement": snapshot.usage.staging_disk_bytes,
            "guest_count_after_retirement": snapshot.usage.guests,
        });
        println!("native guest result {}", row);
        results.push(row);
    }
    if results.len() == 2 {
        for key in ["scope_commitment", "payload_sha256"] {
            ensure!(
                results[0]["peer"][key] != results[1]["peer"][key],
                "sequential guests share an expected-unique peer identity: {key}"
            );
        }
        ensure!(parents[0] != parents[1]);
    }
    let sealed_bytes = finish_with_sealed_telemetry(&mut service, &parents)?;
    println!(
        "{}",
        serde_json::json!({
            "guests": results,
            "guest_readback_exact": true,
            "original_retirement": true,
            "sealed_telemetry_bytes": sealed_bytes,
            "host_process_cpu_ns": cpu.elapsed().as_nanos(),
            "wall_with_retirement_ns": elapsed.elapsed().as_nanos(),
            "startup_resident_quote": startup_bytes,
            "startup_disk_quote": disk_bytes,
            "retained_runtime_disk_quote_each": RETAINED_RUNTIME_QUOTE,
        })
    );
    Ok(())
}
