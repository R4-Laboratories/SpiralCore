//! Explicit Linux qualification entry point. The installer provisions and signs the
//! original bundle and peer policy before invoking this test in its funded home.
#![cfg(all(target_os = "linux", feature = "canonical-host"))]

use anyhow::{Context, Result, ensure};
use mvm_client::{LaunchRequest, LifecycleMode, LocalBackend, RootfsSource};
use mvm_contract::{peer::PeerBinding, policy::network_policy::CooperatingPayloadPolicy};
use mvm_core::{
    host_lifetime::EndpointPartitionLimits,
    plan::{PlanArtifact, SignedExecutionPlan},
    vm_backend::BackendKind,
};
use mvm_hostd::run::DirectTcpPolicy;
use serde::Deserialize;
use serde_json::json;
use spiralcore_mvm_live::{
    installed_owner::{InstallationKey, InstallationOwner},
    installed_parent::{
        InstalledParentProvider, ObservedFlowMuxGuest, PortableStartupQuote, SingleGuestRetention,
        TenantHelperLimits,
    },
    installed_supervisor::{InstalledConfig, InstalledSupervisor, SupervisorError},
};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    num::NonZeroU64,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct OperatorInputs {
    signed_plan: SignedExecutionPlan,
    bundle_pin: PlanArtifact,
    decoded_archive_bytes: NonZeroU64,
    rootfs: PathBuf,
    mapping: CooperatingPayloadPolicy,
    peers: Vec<PeerBinding>,
}

fn operator_inputs(home: &Path) -> Result<OperatorInputs> {
    const CAP: u64 = 256 << 10;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(home.join("flowmux-operator.json"))?;
    let meta = file.metadata()?;
    mvm_core::private_fs::require_current_owner(&meta)?;
    ensure!(
        meta.is_file() && meta.nlink() == 1 && meta.mode() & 0o077 == 0 && meta.len() <= CAP,
        "operator inputs must be an owned, private bounded regular file"
    );
    let mut buf = Vec::new();
    file.take(CAP + 1).read_to_end(&mut buf)?;
    ensure!(
        buf.len() as u64 == meta.len(),
        "operator packet length changed"
    );
    Ok(serde_json::from_slice(&buf)?)
}

fn publish_guest(home: &Path, plan_id: &str, observed: ObservedFlowMuxGuest) -> Result<()> {
    let mut pending = tempfile::Builder::new()
        .prefix("flowmux-public-observation-")
        .tempfile_in(home)?;
    serde_json::to_writer(
        &mut pending,
        &json!({ "plan_id":plan_id, "session":observed.session, "guest":observed.guest, "host":observed.host }),
    )?;
    pending.flush()?;
    pending.as_file().sync_all()?;
    pending.persist_noclobber(home.join("flowmux-guest.json"))?;
    File::open(home)?.sync_all()?;
    Ok(())
}

fn partition() -> EndpointPartitionLimits {
    EndpointPartitionLimits {
        host_plaintext_bytes: 16 << 20,
        host_codec_bytes: 16 << 20,
        host_carrier_bytes: 8 << 20,
        host_metadata_bytes: 48 << 20,
        host_stack_bytes: 48 << 20,
        cache_disk_bytes: 32 << 20,
        staging_disk_bytes: 4 << 20,
        workers: 6,
        max_streams: 1,
    }
}

async fn run_guest(
    home: &Path,
    input: OperatorInputs,
    provider: &InstalledParentProvider,
    backend: &LocalBackend,
) -> Result<(SingleGuestRetention, ObservedFlowMuxGuest)> {
    let n = input.decoded_archive_bytes;
    let additional = |total: u64| {
        total
            .checked_sub(n.get())
            .and_then(NonZeroU64::new)
            .context("signed archive exceeds operator startup allowance")
    };
    let quote = PortableStartupQuote::new(
        input.bundle_pin,
        n,
        additional(512 << 20)?,
        additional(256 << 20)?,
    )?;
    let req = LaunchRequest::builder(
        LifecycleMode::Transient,
        RootfsSource::LocalPath(input.rootfs),
    )
    .name("fm-a")
    .profile("standard")
    .backend("firecracker")
    .cpus(1)
    .memory_mib(256)
    .signed_plan(input.signed_plan)
    .build()?;
    let started = Instant::now();
    let (original, _kernel) = provider
        .prepare_request(&req, "local", BackendKind::Firecracker)?
        .prepare_canonical_startup(quote)?;
    let retained = original.single_guest_retention()?;
    let prepared_ns = started.elapsed().as_nanos();
    let mut launch = original.canonical_driver(backend);
    let policy = DirectTcpPolicy {
        mapping: input.mapping,
        peers: input.peers,
    };
    let result = async {
        launch.start(req, policy).await?;
        let launch_ns = started.elapsed().as_nanos();
        let observed = launch.flowmux_guest()?;
        publish_guest(home, &launch.outcome()?.plan_id, observed)?;
        let exit = launch.wait_until(Instant::now() + Duration::from_secs(40))?;
        let evidence = json!({
            "phase": "workload_exit", "prepared_ns": prepared_ns, "launch_ns": launch_ns,
            "original_exit_code": exit.code, "success": exit.success,
            "through_exit_ns": started.elapsed().as_nanos()
        });
        println!("{evidence}");
        ensure!(
            exit.success && exit.code == Some(0),
            "original workload did not return zero"
        );
        Ok(observed)
    }
    .await;
    let retired = launch.retire().await;
    println!(
        "{}",
        json!({"phase":"sdk_retirement","retired":retired.is_ok(),"through_retirement_ns":started.elapsed().as_nanos()})
    );
    if let Err(ref error) = retired {
        eprintln!("original SDK retirement: {error:#}");
    }
    let observed = result?;
    retired?;
    Ok((retained, observed))
}

#[test]
#[ignore = "requires original operator installation, funded private volume, peer, and KVM"]
fn installed_original_flowmux_workload() -> Result<()> {
    let home = mvm_core::config::mvm_home_strict()?;
    let packet = operator_inputs(&home)?;
    let (mut host, mut tenant) = InstalledConfig::two_endpoint_witness_profile().managed_limits();
    host.host_resident_bytes = 2 << 30;
    tenant.host_resident_bytes = 2 << 30;
    host.staging_disk_bytes = 512 << 20;
    tenant.staging_disk_bytes = 512 << 20;
    let config = InstalledConfig::with_funded_pair_limits(host, tenant)?;
    let owner = InstallationOwner::initialize(
        &home.join("installation"),
        InstallationKey {
            installation: rand::random(),
            profile: config.profile_hash(),
        },
    )?;
    let mut supervisor = InstalledSupervisor::start(owner, config)?;
    let mut provider = InstalledParentProvider::new(&supervisor)?;
    provider.configure_object_tcp(&supervisor, partition())?;
    let backend = LocalBackend::with_hypervisor("firecracker");
    let helper = provider.start_tenant_helper(
        "local",
        TenantHelperLimits {
            process_address_space_bytes: NonZeroU64::new(192 << 20).unwrap(),
            audit_chain_bytes: NonZeroU64::new(1 << 20).unwrap(),
        },
    );
    let provider = Arc::new(provider);
    let result = (|| -> Result<_> {
        helper?;
        provider.install_sdk_registrar(&backend, "local")?;
        mvm_hostd::host_lifetime_parent::install(provider.clone())?;
        mvm_client::registry_application::discovery::block_on(run_guest(
            &home, packet, &provider, &backend,
        ))
    })();
    drop(backend);
    let mut original = Arc::try_unwrap(provider)
        .map_err(|_| anyhow::anyhow!("original provider is still held"))?;
    let stopped = original.stop_tenant_helper();
    if let Err(ref error) = stopped {
        eprintln!("original tenant stop: {error:#}");
    }
    drop(original);
    let finished = supervisor.try_finish();
    let charged = supervisor.managed_resources().snapshot()?;
    println!(
        "{}",
        json!({"phase":"installed_resources","tenant_stopped":stopped.is_ok(),
        "all_resources_released":finished.is_ok(),"charged":charged.usage})
    );
    let (retained, observed) = result?;
    stopped?;
    retained.verify(&supervisor, observed)?;
    ensure!(
        matches!(finished, Err(SupervisorError::Busy)),
        "verified retained storage must keep its original charge"
    );
    Ok(())
}
