//! Original Root ledger through a real selected child. No VM/backend is booted.
use super::*;
use anyhow::Context;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use cpu_time::ThreadTime;
use ed25519_dalek::SigningKey;
use mvm_contract::{
    policy::network_policy::{CooperatingPayloadPolicy, CooperatingTcpPeer, PayloadMappingProfile},
    protocol::network_flow::{Opcode, hello::Handshake},
};
use mvm_core::{
    config,
    host_lifetime::{EndpointPartitionGuard, EndpointPartitionLimits},
    net::session::{Session, read_sealed_frame, write_sealed_frame},
    policy::network_policy::NetworkPolicy,
    protocol::broker::ServiceId,
};
use mvm_vmm::host::{
    host_agent_spawn::HostAgentServicesParams,
    network_endpoint_spawn::{
        AdmittedNetworkPlan, EgressProxySpawnConfig, EndpointLifetime, EndpointTransport,
        FlowMuxIdentitySpawnConfig, InstalledHostServices, SubstitutionSpawnParams,
        spawn_network_endpoint_installed_registered,
    },
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    num::NonZeroU64,
    os::unix::net::UnixStream,
    sync::mpsc,
    time::{Duration, Instant},
};

#[path = "store_load_tests/peer.rs"]
mod peer;
#[path = "store_load_tests/process.rs"]
mod process;
#[path = "store_load_tests/store.rs"]
mod store;

const VM: &str = "original-store-load";
const TARGET: &str = "cooperator.mvm.peer";
const BLOCKS: usize = 64;
const BLOCK_BYTES: usize = 16 << 10;
const IO_TIMEOUT: Duration = Duration::from_secs(3);
// Separate parent-side charge in installed_disk::EndpointDiskInventory::prepare.
const DISPOSABLE_STORE_METADATA_BYTES: u64 = 4096;

fn partition() -> EndpointPartitionLimits {
    // Same explicit child shape as the canonical registered TCP positive.
    // Its 282 MiB address-space declaration is not a minimum RSS measurement.
    EndpointPartitionLimits {
        host_plaintext_bytes: 1 << 20,
        host_codec_bytes: 16 << 20,
        host_carrier_bytes: 1 << 20,
        host_metadata_bytes: 240 << 20,
        host_stack_bytes: 24 << 20,
        cache_disk_bytes: (24 << 20) + crate::installed_retained::SIDECAR_BYTES,
        staging_disk_bytes: 16 << 20,
        workers: 7,
        max_streams: 1,
    }
}

#[test]
#[ignore = "requires exact-source host-agent, signer and network-endpoint binaries on Linux"]
fn original_parent_funds_selected_child_populated_store_and_retirement() {
    // The operator runs this one ignored selector in its own bounded process.
    // isolated_lock_probe respawns without --ignored and cannot run this lane.
    run().unwrap_or_else(|error| panic!("original parent/store load: {error:#}"));
}

fn run() -> Result<()> {
    let setup = Instant::now();
    let setup_cpu = ThreadTime::now();
    for name in [
        "MVM_HOST_AGENT_PATH",
        "MVM_SIGNER_HELPER_PATH",
        "MVM_SUBSTITUTION_ENDPOINT_PATH",
    ] {
        ensure!(
            std::env::var_os(name).is_some_and(|p| std::path::Path::new(&p).is_file()),
            "exact-source helper missing: {name}"
        );
    }
    let baseline = crate::installed_supervisor::InstalledConfig::two_endpoint_witness_profile();
    let (mut host, mut tenant) = baseline.managed_limits();
    ensure!(host.host_resident_bytes == 512 << 20 && host.workers == 24);
    // Reuse the already explicit installed FlowMux operator profile. Neither
    // this fixture nor a guest can enlarge the default 512 MiB profile.
    host.host_resident_bytes = 2 << 30;
    tenant.host_resident_bytes = host.host_resident_bytes;
    let funded =
        crate::installed_supervisor::InstalledConfig::with_funded_pair_limits(host, tenant)?;
    let (home, mut supervisor, mut provider) = endpoint_tests::fixture_with_config(funded);
    let home = home.keep();
    println!("ROOT_STORE_HOME {}", home.display());
    let mut env = mvm_core::util::test_env::TestEnv::new();
    env.set("MVM_HOME", &home);
    env.set("MVM_HOST_AGENT_DAEMON", "1");
    let host_key =
        mvm_hostd::audit::host_keypair::load_or_init_at(&config::mvm_keys_dir())?.signing;
    provider.configure_object_tcp(&supervisor, partition())?;
    provider.start_tenant_helper(
        "local",
        TenantHelperLimits {
            process_address_space_bytes: NonZeroU64::new(192 << 20).unwrap(),
            audit_chain_bytes: NonZeroU64::new(1 << 20).unwrap(),
        },
    )?;
    let guest_key = SigningKey::from_bytes(&[31; 32]);
    let peer_key = peer::identity(&SigningKey::from_bytes(&[32; 32]))?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let policy = peer::policy(address, peer_key.spki_sha256())?;
    let services = vec![ServiceId::parse("host.audit.v1")?];
    let admitted =
        config_tests::admit_with_network_services(&home, VM, Some(&policy), services.clone());
    let mut config = VmStartConfig {
        name: VM.into(),
        tenant_id: Some("local".into()),
        cpus: 1,
        memory_mib: 1024,
        network_policy: policy.clone(),
        plan_json: Some(serde_json::to_string(admitted.signed())?),
        ..Default::default()
    };
    let prepared = provider.prepare_vm(&config, "local", BackendKind::Firecracker)?;
    let (lifetime, mut owner) = prepared.begin_launch()?;
    let anchor = lifetime.into_anchor();
    anchor.bind_to_config(&mut config)?;
    let permit = provider.bind_config(&admitted, &config, BackendKind::Firecracker)?;
    anchor.bind_parent_permit(&config, BackendKind::Firecracker, permit)?;
    let state = config::vm_state_dir(VM);
    config::create_private_dir(&state)?;
    let broker =
        config::vm_vsock_port_socket_at(&state, mvm_net::channel::GuestService::Broker.port());
    let registrar = owner.original_registrar()?;
    let params = || HostAgentServicesParams {
        workload_id: VM,
        tenant_id: Some("local"),
        vm_name: VM,
        state_dir: &state,
        broker_listen_socket: &broker,
        services: &services,
        capability_bindings: &[],
        service_proxies: &[],
    };
    let mut registration = registrar.register(params())?;
    let original = InstalledHostServices::new(&registrar, &registration, params())?;
    let pool = supervisor.managed_resources().clone();
    let before = pool.snapshot()?;
    let handoff = anchor.prepare_endpoint_handoff(&config, BackendKind::Firecracker)?;
    let projection = handoff.partition().projection().clone();
    let root = owner
        .grant
        .lifecycle
        .lock()
        .unwrap()
        .endpoint_owner
        .clone()
        .unwrap();
    let guard: Arc<dyn EndpointPartitionGuard> = root;
    ensure!(
        handoff.partition().retains_guard(&guard),
        "foreign Root partition"
    );
    let charged = pool.snapshot()?;
    ensure!(charged.usage.workers - before.usage.workers == 7);
    ensure!(
        charged.usage.codec_store_file_descriptors - before.usage.codec_store_file_descriptors == 7
    );
    ensure!(
        charged.usage.host_resident_bytes()? - before.usage.host_resident_bytes()?
            == partition().host_resident_bytes()? + DISPOSABLE_STORE_METADATA_BYTES
    );
    ensure!(
        charged.usage.host_resident_bytes()? > 512 << 20,
        "fixture unexpectedly fits the default profile"
    );
    let plan = AdmittedNetworkPlan::from_plan(admitted.plan(), VM, &policy)?;
    let redaction = Default::default();
    let spawn = SubstitutionSpawnParams::builder()
        .vm_name(VM)
        .state_dir(&state)
        .lifetime(EndpointLifetime::Launcher)
        .tenant("local")
        .secrets(&[])
        .redaction(&redaction)
        .transport(EndpointTransport::Uds {
            path: state.join("network.sock"),
        })
        .telemetry(true)
        .egress_proxy(EgressProxySpawnConfig::default())
        .network_policy(&policy)
        .admitted_network_plan(plan.clone())
        .network_limits(Default::default())
        .ingress(&[])
        .flowmux_identity(FlowMuxIdentitySpawnConfig {
            session_id: hex::encode(projection.session),
            host_signing_key_base64: STANDARD.encode(host_key.to_bytes()),
            guest_verifying_key_base64: STANDARD.encode(guest_key.verifying_key().to_bytes()),
        })
        .build()?;
    let mut endpoint = spawn_network_endpoint_installed_registered(spawn, handoff, &original)?;
    let pid = endpoint.id();
    let mut guest = UnixStream::connect(state.join("network.sock"))?;
    guest.set_read_timeout(Some(IO_TIMEOUT))?;
    guest.set_write_timeout(Some(IO_TIMEOUT))?;
    let (mut session, _) =
        Session::guest(&mut guest, guest_key.clone(), &host_key.verifying_key())?;
    let ready_path = config::vm_socket_dir_at(&state)
        .join(mvm_vmm::host::network_endpoint_spawn::SUBST_SESSION_READY_SOCKET);
    let mut ready = UnixStream::connect(ready_path)?;
    ready.set_read_timeout(Some(IO_TIMEOUT))?;
    let mut byte = [0];
    ready.read_exact(&mut byte)?;
    ensure!(byte == [1]);
    send(
        &mut guest,
        &mut session,
        Opcode::Hello,
        0,
        &Handshake::local("root-store-load").encode(),
    )?;
    ensure!(receive(&mut guest, &mut session)?.0 == Opcode::HelloAck);
    let setup_wall_ns = setup.elapsed().as_nanos();
    let setup_sender_cpu_ns = setup_cpu.elapsed().as_nanos();
    let initiator = peer::identity(&host_key)?.spki_sha256();
    let (at_block, observed) = mpsc::sync_channel(0);
    let (resume, resumed) = mpsc::sync_channel(0);
    let scope_session = projection.session;
    let peer = std::thread::spawn(move || {
        peer::serve(
            listener,
            peer_key,
            initiator,
            peer::AdmittedGuest {
                plan,
                session: scope_session,
                guest: guest_key.verifying_key().to_bytes(),
            },
            at_block,
            resumed,
        )
    });
    let exchanged = (|| -> Result<_> {
        let transfer = Instant::now();
        let sender_cpu = ThreadTime::now();
        send(
            &mut guest,
            &mut session,
            Opcode::OpenTcp,
            1,
            format!("{TARGET}:443").as_bytes(),
        )?;
        ensure!(receive(&mut guest, &mut session)?.0 == Opcode::Opened);
        let mut samples = Vec::with_capacity(BLOCKS);
        let mut store_scope = None;
        let mut request_bytes = 0u64;
        let mut response_bytes = 0u64;
        for index in 0..BLOCKS {
            let body = peer::block(index);
            request_bytes += send(&mut guest, &mut session, Opcode::Data, 1, &body)?;
            let observed = observed
                .recv_timeout(IO_TIMEOUT)
                .map_err(|error| barrier_error(&mut guest, &mut session, index, error))?;
            ensure!(observed.index == index);
            store_scope = Some((observed.flow, observed.namespace));
            samples.push(process::sample(
                pid,
                &projection
                    .store
                    .as_ref()
                    .context("original store missing")?
                    .path,
            )?);
            resume.send(())?;
            let mut restored = Vec::with_capacity(BLOCK_BYTES);
            while restored.len() < BLOCK_BYTES {
                let (opcode, id, bytes, wire) = receive(&mut guest, &mut session)?;
                response_bytes += wire;
                ensure!(id == 1);
                match opcode {
                    Opcode::Data => restored.extend_from_slice(&bytes),
                    Opcode::WindowUpdate => ensure!(bytes.len() == 4),
                    _ => anyhow::bail!("unexpected populated-store response {opcode:?}"),
                }
                ensure!(restored.len() <= BLOCK_BYTES);
            }
            ensure!(restored == body, "block {index} lost fidelity");
            // Return only the exact bytes consumed and verified by this guest.
            // Keeping the canonical window prevents the echo from stalling at 256 KiB.
            request_bytes += send(
                &mut guest,
                &mut session,
                Opcode::WindowUpdate,
                1,
                &u32::try_from(restored.len())?.to_be_bytes(),
            )?;
            ensure!(
                pool.snapshot()?.usage == charged.usage,
                "original funding changed during load"
            );
        }
        let (flow, namespace) = store_scope.context("original store scope not observed")?;
        let store = store::observe(
            projection
                .store
                .as_ref()
                .context("original store missing")?,
            flow,
            namespace,
        )?;
        request_bytes += send(&mut guest, &mut session, Opcode::HalfClose, 1, &[])?;
        loop {
            let (opcode, id, bytes, wire) = receive(&mut guest, &mut session)?;
            response_bytes += wire;
            ensure!(id == 1);
            match opcode {
                Opcode::WindowUpdate => ensure!(bytes.len() == 4),
                Opcode::HalfClose => {
                    ensure!(bytes.is_empty());
                    break;
                }
                _ => anyhow::bail!("unexpected final response {opcode:?}"),
            }
        }
        let sender_cpu_ns = sender_cpu.elapsed().as_nanos();
        let verified_wall_ns = transfer.elapsed().as_nanos();
        Ok((
            samples,
            request_bytes,
            response_bytes,
            sender_cpu_ns,
            verified_wall_ns,
            store,
        ))
    })();
    drop((resume, observed));
    if exchanged.is_err() {
        let _ = guest.shutdown(std::net::Shutdown::Both);
    }
    // Always join the independent peer, retaining its original error as well
    // as any consequential guest/transport error.
    let peer_result = peer
        .join()
        .map_err(|_| anyhow!("cooperating peer panicked"))
        .and_then(|r| r);
    let ((samples, request_bytes, response_bytes, sender_cpu_ns, verified_wall_ns, store), peer) =
        match (exchanged, peer_result) {
            (Ok(exchange), Ok(peer)) => (exchange, peer),
            (Err(error), Err(peer)) => {
                return Err(anyhow!("guest/load: {error:#}; independent peer: {peer:#}"));
            }
            (Err(error), Ok(_)) | (Ok(_), Err(error)) => return Err(error),
        };
    for sample in &samples {
        ensure!(sample.address_space_limit == partition().host_resident_bytes()?);
        ensure!(
            sample.threads <= u64::from(partition().workers),
            "unfunded live thread"
        );
        ensure!(sample.virtual_bytes <= sample.address_space_limit);
        ensure!(
            sample.original_store_open_fds <= 7,
            "observed unfunded store descriptor"
        );
        ensure!(
            sample.start_ticks == samples[0].start_ticks,
            "observed process identity changed"
        );
    }
    ensure!(
        samples.iter().map(|s| s.threads).max() == Some(7),
        "concurrent seven-worker boundary not observed"
    );
    let retire = Instant::now();
    let mut children_before = process::direct_children()?;
    ensure!(
        children_before.remove(&pid),
        "original endpoint is not an observed child"
    );
    let cpu_before = process::children_cpu_ns()?;
    endpoint.shutdown()?;
    let endpoint_retirement_cpu_ns = process::children_cpu_ns()?
        .checked_sub(cpu_before)
        .context("child CPU counter regressed")?;
    ensure!(
        process::direct_children()? == children_before,
        "another child changed during CPU collection"
    );
    let receipt = endpoint.take_retirement()?;
    ensure!(receipt.retains_partition_guard(&guard));
    ensure!(endpoint.take_retirement().is_err());
    let endpoint_retirement_wall_ns = retire.elapsed().as_nanos();
    let mut pending = Some(receipt);
    owner.retire_endpoint(&mut pending)?;
    ensure!(pending.is_none());
    store.check_retired()?;
    let after_cleanup = pool.snapshot()?;
    ensure!(
        after_cleanup.usage.codec_store_file_descriptors
            == charged.usage.codec_store_file_descriptors
    );
    ensure!(after_cleanup.usage.workers == charged.usage.workers);
    ensure!(
        after_cleanup.usage.host_resident_bytes()?
            == charged.usage.host_resident_bytes()? - DISPOSABLE_STORE_METADATA_BYTES
    );
    ensure!(
        after_cleanup.usage.cache_disk_bytes
            == charged.usage.cache_disk_bytes
                - (partition().cache_disk_bytes
                    - crate::installed_retained::RECORD_BYTES
                    - crate::installed_retained::SIDECAR_BYTES)
    );
    ensure!(
        after_cleanup.usage.staging_disk_bytes
            == charged.usage.staging_disk_bytes - partition().staging_disk_bytes
    );
    registration.shutdown()?;
    let after = pool.snapshot()?;
    ensure!(
        after.usage.guests == 1,
        "missing VM receipt must retain its debit"
    );
    ensure!(
        after.usage.workers >= 1,
        "missing VM receipt must retain exit capture"
    );
    drop((registration, registrar, endpoint));
    provider.stop_tenant_helper()?;
    provider.close_admission();
    supervisor.close_admission();
    ensure!(
        supervisor.try_finish().is_err(),
        "no VM retirement was established"
    );
    println!(
        "{}",
        serde_json::json!({
            "scope":"Linux original Root ledger and selected host process; no VM or WAN",
            "host_ceiling_bytes":host.host_resident_bytes,"worker_ceiling":host.workers,
            "partition":projection,"before":before,"charged":charged,"after_endpoint":after,
            "app_bytes_each_direction":BLOCKS*BLOCK_BYTES,"exact_blocks":BLOCKS,
            "guest_load_sealed_request_bytes":request_bytes,"guest_load_sealed_response_bytes":response_bytes,
        "peer":peer,"process_samples":samples,"populated_store":store.observation,
        "after_original_endpoint_cleanup":after_cleanup,
            "setup_wall_ns":setup_wall_ns,"setup_sender_thread_cpu_ns":setup_sender_cpu_ns,
            "verified_wall_ns":verified_wall_ns,"sender_thread_cpu_ns":sender_cpu_ns,
            "endpoint_whole_child_cpu_collected_at_retirement_ns":endpoint_retirement_cpu_ns,
            "endpoint_retirement_wall_ns":endpoint_retirement_wall_ns,
            "vm_retirement":"NOT_RUN; original VM reservation remains quarantined"
        })
    );
    // No completed=true, synthetic VMM exit, unused cancellation or refund.
    // This isolated process exits with the original admitted VM debit retained.
    drop((owner, anchor, config, provider));
    Ok(())
}

fn send(
    io: &mut UnixStream,
    session: &mut Session,
    opcode: Opcode,
    id: u32,
    bytes: &[u8],
) -> Result<u64> {
    let encoded = mvm_contract::protocol::network_flow::encode(opcode, id, bytes)?;
    let sealed = session.seal(&encoded)?;
    let mut meter = Meter { io, bytes: 0 };
    write_sealed_frame(&mut meter, &sealed)?;
    Ok(meter.bytes)
}

fn barrier_error(
    io: &mut UnixStream,
    session: &mut Session,
    index: usize,
    error: mpsc::RecvTimeoutError,
) -> anyhow::Error {
    let until = Instant::now() + IO_TIMEOUT;
    let mut controls = Vec::new();
    for _ in 0..4 {
        let remaining = until.saturating_duration_since(Instant::now());
        if remaining.is_zero() || io.set_read_timeout(Some(remaining)).is_err() {
            break;
        }
        match receive(io, session) {
            Ok((opcode, id, bytes, _)) => {
                let reason = if matches!(opcode, Opcode::GoAway | Opcode::Reset | Opcode::Refused) {
                    String::from_utf8_lossy(&bytes[..bytes.len().min(256)]).into_owned()
                } else {
                    format!("{} payload bytes", bytes.len())
                };
                controls.push(format!("{opcode:?}/{id}: {reason}"));
                if matches!(
                    opcode,
                    Opcode::GoAway | Opcode::Reset | Opcode::Refused | Opcode::HalfClose
                ) {
                    break;
                }
            }
            Err(error) => {
                controls.push(format!("control read: {error:#}"));
                break;
            }
        }
    }
    anyhow!(
        "block {index} barrier: {error}; original host control: {}",
        controls.join("; ")
    )
}

fn receive(io: &mut UnixStream, session: &mut Session) -> Result<(Opcode, u32, Vec<u8>, u64)> {
    let mut meter = Meter { io, bytes: 0 };
    let sealed = read_sealed_frame(&mut meter, 1 << 20)?;
    let wire = meter.bytes;
    let plain = session.open(&sealed)?;
    let frame = mvm_contract::protocol::network_flow::decode(&plain)?;
    Ok((
        frame.header.opcode,
        frame.header.stream_id,
        frame.payload.to_vec(),
        wire,
    ))
}

struct Meter<'a> {
    io: &'a mut UnixStream,
    bytes: u64,
}
impl Read for Meter<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let n = self.io.read(out)?;
        self.bytes += n as u64;
        Ok(n)
    }
}
impl Write for Meter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let n = self.io.write(bytes)?;
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.io.flush()
    }
}
