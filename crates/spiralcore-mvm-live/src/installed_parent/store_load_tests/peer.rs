//! Independent cooperating plaintext endpoint using the shared ObjectEndpoint/v1 core.
use super::*;
use ed25519_dalek::pkcs8::EncodePrivateKey;
use mvm_contract::peer::{PeerBinding, PeerName};
use mvm_hostd::supervisor::endpoint::flowmux::{
    PayloadResourceLimits, PayloadResourcePool, PreparedPayloadSession, accept_enrolled_payload,
};
use spiralcore_mvm_flowmux::{
    Role,
    scope::{PinnedScopePolicy, ScopeFacts},
    tls::{PinnedIdentity, TlsCarrier},
};
use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
};
use std::{
    net::{Shutdown, SocketAddr},
    sync::mpsc::{Receiver, SyncSender},
};

pub(super) struct BlockGate {
    pub index: usize,
    pub flow: [u8; 32],
    pub namespace: [u8; 32],
}

pub(super) struct AdmittedGuest {
    pub plan: AdmittedNetworkPlan,
    pub session: [u8; 32],
    pub guest: [u8; 32],
}

pub(super) fn identity(key: &SigningKey) -> Result<PinnedIdentity> {
    Ok(PinnedIdentity::from_pkcs8(
        key.to_pkcs8_der()?.as_bytes().to_vec(),
    )?)
}

pub(super) fn policy(address: SocketAddr, spki: [u8; 32]) -> Result<NetworkPolicy> {
    Ok(NetworkPolicy::deny_all()
        .with_peers(vec![PeerBinding {
            name: PeerName::parse(TARGET)?,
            port: 443,
            host_addr: address.ip().to_string(),
            host_port: address.port(),
        }])
        .with_payload_mapping(Some(CooperatingPayloadPolicy::new(vec![
            CooperatingTcpPeer {
                host: TARGET.into(),
                port: 443,
                server_name: TARGET.into(),
                spki_sha256: spki,
                profile: PayloadMappingProfile::SpiralCoreObjectStreamSmallV1,
            },
        ])?)))
}

pub(super) fn block(index: usize) -> Vec<u8> {
    let mut bytes: Vec<_> = (0..BLOCK_BYTES)
        .map(|n| ((n * 19 + index * 71) % 251) as u8)
        .collect();
    bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
    bytes[BLOCK_BYTES - 4..].fill(0);
    bytes
}

pub(super) fn serve(
    listener: TcpListener,
    key: PinnedIdentity,
    initiator: [u8; 32],
    admitted: AdmittedGuest,
    at_block: SyncSender<BlockGate>,
    resumed: Receiver<()>,
) -> Result<serde_json::Value> {
    let AdmittedGuest {
        plan,
        session,
        guest,
    } = admitted;
    let start = Instant::now();
    let cpu = ThreadTime::now();
    let address = listener.local_addr()?;
    let socket = loop {
        match listener.accept() {
            Ok((socket, _)) => break socket,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    && start.elapsed() < IO_TIMEOUT =>
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error.into()),
        }
    };
    let until = Instant::now() + IO_TIMEOUT;
    let tls =
        TlsCarrier::accept(socket, &key, initiator, until).context("cooperator TLS accept")?;
    let meter = tls.try_clone()?;
    let digest = |value: &str| -> Result<[u8; 32]> {
        hex::decode(value)?
            .try_into()
            .map_err(|_| anyhow!("scope digest size differs"))
    };
    let facts = ScopeFacts {
        object_profile: 2,
        policy_sha256: digest(&plan.policy_sha256)?,
        plan_id: plan.plan_id.0,
        plan_version: plan.plan_version,
        plan_nonce: hex::decode(plan.plan_nonce.as_hex())?
            .try_into()
            .map_err(|_| anyhow!("nonce size"))?,
        bundle_sha256: digest(&plan.bundle_sha256)?,
        tenant: plan.tenant_id,
        instance: plan.instance_id,
        session: hex::encode(session),
        guest_key: guest,
        stream_id: 1,
        requested_host: TARGET.into(),
        requested_port: 443,
        actual_peer: address,
        route: "peer".into(),
        initiator_spki: tls.peer_spki_sha256(),
        acceptor_spki: tls.local_spki_sha256(),
        tls_exporter: tls.exporter(),
    };
    let scopes = PinnedScopePolicy::new(1)?;
    scopes.enroll(&facts, initiator, Role::Acceptor, until)?;
    let authority = scopes.authorize_scope(&facts, initiator, Role::Acceptor)?;
    let flow = facts.commitment()?;
    let namespace = authority.context().namespace()?;
    let resource = ResourceProfile::SmallV1;
    // This separate test-controller peer is outside the selected child's Root
    // partition. It neither lends credit to that child nor represents a VM.
    let pool = PayloadResourcePool::for_qualification(PayloadResourceLimits {
        max_active: 1,
        resident_bytes: 8 << 20,
        disk_bytes: 32 << 20,
    })?;
    let permit = pool.reserve_profile(resource)?;
    let store = tempfile::tempdir()?;
    let path = store.path().join("peer");
    let profile = Capabilities::small_v1().negotiate(&Capabilities::small_v1())?;
    let sender = ObjectSender::new_with_resource_profile(
        ObjectMode::Whole,
        authority.context(),
        None,
        profile,
        resource,
    )?;
    let endpoint = ObjectEndpoint::open_with_resource_profile(
        &path,
        resource.receiver_limits(),
        authority.clone(),
        profile,
        resource,
    )?;
    let prepared =
        PreparedPayloadSession::new(authority, sender, endpoint, permit.with_owned_store(path)?)?;
    let mut mapped =
        accept_enrolled_payload(tls, prepared, until).context("cooperator object session")?;
    for index in 0..BLOCKS {
        let mut body = vec![0; BLOCK_BYTES];
        mapped
            .read_exact(&mut body)
            .with_context(|| format!("cooperator block {index} read"))?;
        ensure!(
            body == block(index),
            "cooperator received altered block {index}"
        );
        at_block.send(BlockGate {
            index,
            flow,
            namespace,
        })?;
        resumed.recv_timeout(IO_TIMEOUT)?;
        mapped.write_all(&body)?;
        mapped.flush()?;
    }
    ensure!(
        mapped.read(&mut [0])? == 0,
        "unexpected trailing application byte"
    );
    mapped.shutdown(Shutdown::Write)?;
    drop(mapped);
    pool.close_admission()?;
    ensure!(pool.drained()?);
    let totals = meter.counters();
    ensure!(totals.sent > 0 && totals.received > 0);
    Ok(
        serde_json::json!({"app_bytes_each_direction":BLOCKS*BLOCK_BYTES,
        "tls_sent":totals.sent,"tls_received":totals.received,
        "wall_ns":start.elapsed().as_nanos(),"thread_cpu_ns":cpu.elapsed().as_nanos(),
        "scope":"independent cooperating peer in test-controller thread"}),
    )
}
