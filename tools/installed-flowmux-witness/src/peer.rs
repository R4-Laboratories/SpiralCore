use anyhow::{Context, Result, ensure};
use ed25519_dalek::{
    VerifyingKey,
    pkcs8::{DecodePublicKey, EncodePublicKey},
};
use installed_flowmux_witness::input::{
    Profile, funded_scratch, parse_inputs, protected_json, staged,
};
use mvm_contract::policy::network_policy::CooperatingPayloadPolicy;
use mvm_core::plan::{PlanArtifact, SignedExecutionPlan, verify_plan};
use mvm_core::policy::direct_plan::AdmittedNetworkPlan;
use mvm_hostd::supervisor::flowmux::{
    PayloadResourceLimits, PayloadResourcePool, PreparedPayloadSession, accept_enrolled_payload,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
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
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener},
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    signed_plan: SignedExecutionPlan,
    bundle_pin: PlanArtifact,
    decoded_archive_bytes: u64,
    #[serde(rename = "archive")]
    _archive: PathBuf,
    #[serde(rename = "rootfs")]
    _rootfs: PathBuf,
    mapping: CooperatingPayloadPolicy,
    peers: Vec<mvm_contract::peer::PeerBinding>,
    peer_spki_sha256: String,
    workload: String,
    vm_name: String,
    tenant: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observed {
    plan_id: String,
    session: [u8; 32],
    guest: [u8; 32],
    host: [u8; 32],
}

pub fn payload() -> Vec<u8> {
    let mut bytes = b"{\"kind\":\"flowmux\",\"value\":1}\n".repeat(200);
    bytes.extend_from_slice(&[0, 255, 0, 0]);
    bytes
}

fn receive_original(reader: impl Read) -> Result<Vec<u8>> {
    let expected = payload();
    let mut body = Vec::with_capacity(expected.len() + 1);
    reader
        .take((expected.len() + 1) as u64)
        .read_to_end(&mut body)?;
    ensure!(body == expected, "incorrect echo body or EOF");
    Ok(body)
}

fn require_peer(
    profile: &Profile,
    mapping: &CooperatingPayloadPolicy,
    spki: [u8; 32],
    packet_spki: &str,
) -> Result<()> {
    let selected = mapping
        .select(&profile.allowed_ip.to_string(), profile.port)
        .context("peer outside original mapping")?;
    ensure!(mapping.peers().len()==1 && selected.spki_sha256==spki && packet_spki==hex::encode(spki)
            && selected.server_name==profile.server_name && selected.profile==mvm_contract::policy::network_policy::PayloadMappingProfile::SpiralCoreObjectStreamSmallV1,
            "original peer identity or route mismatch");
    Ok(())
}

fn decode_original<const N: usize>(value: &str) -> Result<[u8; N]> {
    let mut bytes = [0; N];
    hex::decode_to_slice(value, &mut bytes)
        .context("signed scope field differs from its canonical size")?;
    Ok(bytes)
}
pub fn run() -> Result<()> {
    let path = parse_inputs(std::env::args_os().skip(1))?;
    let inputs = staged(&path)?;
    let profile: Profile =
        serde_json::from_slice(&inputs.read_private_member("profile.json", 8192)?)?;
    profile.check()?;
    let host = profile.allowed_ip.to_string();
    let port = profile.port;
    let home = installed_flowmux_witness::input::explicit_home()?;
    let scratch = funded_scratch(&home)?;
    let packet: Packet = protected_json(&home.join("flowmux-operator.json"), 256 << 10)?;
    let issuer =
        VerifyingKey::from_public_key_der(&inputs.read_private_member("issuer.spki", 4096)?)?;
    let plan = verify_plan(
        &packet.signed_plan,
        &[(profile.issuer_id.as_str(), &issuer)],
    )?;
    let original_until: SystemTime = plan.valid_until.into();
    ensure!(
        SystemTime::now() >= SystemTime::from(plan.valid_from),
        "signed plan is premature"
    );
    let valid = original_until.duration_since(SystemTime::now())?;
    let until = Instant::now() + Duration::from_secs(90).min(valid);
    let mapping = packet.mapping;
    let identity = PinnedIdentity::from_pkcs8(inputs.read_private_member("peer.der", 4096)?)?;
    ensure!(packet.peers.is_empty(), "unexpected peer redirect");
    require_peer(
        &profile,
        &mapping,
        identity.spki_sha256(),
        &packet.peer_spki_sha256,
    )?;
    let network = mvm_contract::grants::projection::network_policy_from_grants(
        plan.grants.as_ref().context("signed grants required")?,
    )
    .with_payload_mapping(Some(mapping));
    ensure!(
        network.resolve_rules()
            == Some(vec![mvm_contract::policy::network_policy::HostPort::new(
                host.as_str(),
                port
            )]),
        "expected one exact signed destination"
    );
    let verbs = [
        mvm_core::plan::VerbId::new("activate-environment")?,
        mvm_core::plan::VerbId::new("run-entrypoint")?,
    ];
    ensure!(
        plan.agent_verbs.as_deref() == Some(&verbs)
            && plan.workload.0 == "flowmux-app"
            && plan.stream_edges.is_empty()
            && plan.stream_retention == mvm_core::plan::StreamRetention::Ephemeral
            && plan.secrets.is_empty()
            && plan.mailbox.is_none(),
        "unexpected signed workload"
    );
    let admitted = AdmittedNetworkPlan::from_plan(&plan, "fm-a", &network)?;
    profile.matches_archive(
        &packet.bundle_pin.bundle_sha256,
        packet.decoded_archive_bytes,
    )?;
    ensure!(
        plan.bundle.as_ref() == Some(&packet.bundle_pin)
            && admitted.bundle_sha256 == packet.bundle_pin.bundle_sha256
            && admitted.tenant_id == "local"
            && admitted.instance_id == "fm-a"
            && packet.vm_name == admitted.instance_id
            && packet.tenant == admitted.tenant_id
            && packet.workload == plan.workload.0,
        "original scope mismatch"
    );
    let address = SocketAddr::V4(profile.address());
    let listener = TcpListener::bind(address)?;
    listener.set_nonblocking(true)?;
    println!("READY {address}");
    std::io::stdout().flush()?;
    let fd = rustix::event::PollFd::new(&listener, rustix::event::PollFlags::IN);
    let timeout =
        rustix::event::Timespec::try_from(until.saturating_duration_since(Instant::now()))?;
    ensure!(
        rustix::event::poll(&mut [fd], Some(&timeout))? == 1,
        "no original caller within deadline"
    );
    let (socket, _) = listener.accept()?;
    drop(listener);
    let start = Instant::now();
    let until = until.min(start + Duration::from_secs(14));
    // The original controller syncs this record before dispatching RunEntrypoint.
    let original: Observed = protected_json(&home.join("flowmux-guest.json"), 4096)?;
    let plan_id = admitted.plan_id.0.clone();
    ensure!(
        original.plan_id == plan_id,
        "observation belongs to a different plan"
    );
    let host_spki = Sha256::digest(
        VerifyingKey::from_bytes(&original.host)?
            .to_public_key_der()?
            .as_bytes(),
    )
    .into();
    let tls = TlsCarrier::accept(socket, &identity, host_spki, until)?;
    let meter = tls.try_clone()?;
    let facts = ScopeFacts {
        object_profile: 2,
        policy_sha256: decode_original(&admitted.policy_sha256)?,
        plan_id: plan_id.clone(),
        plan_version: admitted.plan_version,
        plan_nonce: decode_original(admitted.plan_nonce.as_hex())?,
        bundle_sha256: decode_original(&admitted.bundle_sha256)?,
        tenant: admitted.tenant_id,
        instance: admitted.instance_id,
        session: hex::encode(original.session),
        guest_key: original.guest,
        stream_id: 3,
        requested_host: host,
        requested_port: port,
        actual_peer: address,
        route: "egress".into(),
        initiator_spki: tls.peer_spki_sha256(),
        acceptor_spki: tls.local_spki_sha256(),
        tls_exporter: tls.exporter(),
    };
    let scopes = PinnedScopePolicy::new(1)?;
    scopes.enroll(&facts, facts.initiator_spki, Role::Acceptor, until)?;
    let authority = scopes.authorize_scope(&facts, facts.initiator_spki, Role::Acceptor)?;
    let resources = ResourceProfile::SmallV1;
    // A separate cooperating endpoint in the test controller, explicitly not
    // another local child subdivision or a production host capacity grant.
    let pool = PayloadResourcePool::for_qualification(PayloadResourceLimits {
        max_active: 1,
        resident_bytes: 8 << 20,
        disk_bytes: 32 << 20,
    })?;
    let permit = pool.reserve_profile(resources)?;
    let store = tempfile::tempdir_in(scratch)?;
    let path = store.path().join("peer");
    let profile = Capabilities::small_v1().negotiate(&Capabilities::small_v1())?;
    let sender = ObjectSender::new_with_resource_profile(
        ObjectMode::Whole,
        authority.context(),
        None,
        profile,
        resources,
    )?;
    let endpoint = ObjectEndpoint::open_with_resource_profile(
        &path,
        resources.receiver_limits(),
        authority.clone(),
        profile,
        resources,
    )?;
    let prepared =
        PreparedPayloadSession::new(authority, sender, endpoint, permit.with_owned_store(path)?)?;
    let mut mapped = accept_enrolled_payload(tls, prepared, until)?;
    let body = receive_original(&mut mapped)?;
    mapped.write_all(&body)?;
    mapped.shutdown(Shutdown::Write)?;
    drop(mapped);
    pool.close_admission()?;
    ensure!(pool.drained()?);
    let bytes = meter.counters();
    println!(
        "{}",
        serde_json::json!({"scope":"external one-session cooperating endpoint", "plan_id":plan_id, "stream_id":3,"echo_received":body.len(),"echo_written":body.len(),"tls_rx":bytes.received,"tls_tx":bytes.sent,
        "accept_to_half_close_ns":start.elapsed().as_nanos()})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mvm_contract::policy::network_policy::{CooperatingTcpPeer, PayloadMappingProfile};

    #[test]
    fn selected_identity_requires_the_original_host_port_spki_and_server_name() -> Result<()> {
        let p: Profile = serde_json::from_value(serde_json::json!({
            "issuer_id":"fixture","allowed_ip":"192.0.2.7","port":38443,"server_name":"peer.example",
            "archive":{"sha256":"aa".repeat(32),"decoded_bytes":30}
        }))?;
        let mut peer = CooperatingTcpPeer {
            host: p.allowed_ip.to_string(),
            port: p.port,
            server_name: p.server_name.clone(),
            spki_sha256: [2; 32],
            profile: PayloadMappingProfile::SpiralCoreObjectStreamSmallV1,
        };
        let mapping = CooperatingPayloadPolicy::new(vec![peer.clone()])?;
        require_peer(&p, &mapping, [2; 32], &hex::encode([2; 32]))?;
        assert!(require_peer(&p, &mapping, [3; 32], &hex::encode([2; 32])).is_err());
        assert!(require_peer(&p, &mapping, [2; 32], &hex::encode([3; 32])).is_err());
        peer.port += 1;
        let extra = CooperatingPayloadPolicy::new(vec![mapping.peers()[0].clone(), peer.clone()])?;
        assert!(require_peer(&p, &extra, [2; 32], &hex::encode([2; 32])).is_err());
        peer.port = p.port;
        peer.server_name = "other.example".into();
        let other = CooperatingPayloadPolicy::new(vec![peer])?;
        assert!(require_peer(&p, &other, [2; 32], &hex::encode([2; 32])).is_err());
        Ok(())
    }

    #[test]
    fn full_body_requires_binary_suffix_exact_length_and_eof() -> Result<()> {
        let expected = payload();
        assert_eq!(expected.len(), 5804);
        assert_eq!(&expected[5800..], [0, 255, 0, 0]);
        assert_eq!(receive_original(&expected[..])?, expected);
        assert!(receive_original(&expected[..5803]).is_err());
        let mut other = expected.clone();
        other[5801] = 0;
        assert!(receive_original(&other[..]).is_err());
        other = expected;
        other.push(0);
        assert!(receive_original(&other[..]).is_err());
        Ok(())
    }
}
