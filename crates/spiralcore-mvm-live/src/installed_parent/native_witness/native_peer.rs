//! Private Linux witness peer. This module holds one cooperating endpoint.

use anyhow::{Context, Result, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{
    SigningKey, VerifyingKey,
    pkcs8::{EncodePrivateKey as _, EncodePublicKey as _},
};
use mvm_contract::policy::network_policy::{
    CooperatingPayloadPolicy, CooperatingTcpPeer, HostPort, NetworkPolicy, PayloadMappingProfile,
};
use mvm_core::plan::ExecutionPlan;
use mvm_hostd::supervisor::flowmux::{
    PayloadResourceLimits, PayloadResourcePool, PreparedPayloadSession, accept_enrolled_payload,
};
use mvm_vmm::host::{
    flowmux_identity::{InheritableIdentity, PUBLIC_IDENTITY_FILE},
    network_endpoint_spawn::AdmittedNetworkPlan,
};
use rand::{TryRng as _, rngs::SysRng};
use sha2::{Digest as _, Sha256};
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
    fs::{File, OpenOptions},
    io::{ErrorKind, Read as _, Write as _},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener},
    os::unix::fs::OpenOptionsExt as _,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

const REPETITIONS: usize = 400;
const IDENTITY_BYTES: u64 = 1024;
const TRANSFER_WINDOW: Duration = Duration::from_secs(10);

pub(super) struct NativePeer {
    listener: TcpListener,
    address: SocketAddr,
    identity: PinnedIdentity,
    value: u8,
}

impl NativePeer {
    pub(super) fn bind(ip: Ipv4Addr, value: u8) -> Result<Self> {
        ensure!(matches!(value, 1 | 2));
        ensure!(
            ip.is_private(),
            "witness needs its exact private LAN address"
        );
        let listener = TcpListener::bind((ip, 0))?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let mut seed = [0; 32];
        SysRng.try_fill_bytes(&mut seed)?;
        let signing_key = SigningKey::from_bytes(&seed);
        let pkcs8 = signing_key.to_pkcs8_der()?;
        let identity = PinnedIdentity::from_pkcs8(pkcs8.as_bytes().to_vec())?;
        Ok(Self {
            listener,
            address,
            identity,
            value,
        })
    }

    pub(super) fn address(&self) -> SocketAddr {
        self.address
    }

    pub(super) fn network_policy(&self) -> Result<NetworkPolicy> {
        let host = self.address.ip().to_string();
        let port = self.address.port();
        let mapping = CooperatingPayloadPolicy::new(vec![CooperatingTcpPeer {
            host: host.clone(),
            port,
            server_name: host.clone(),
            spki_sha256: self.identity.spki_sha256(),
            profile: PayloadMappingProfile::SpiralCoreObjectStreamSmallV1,
        }])?;
        Ok(NetworkPolicy::allow_list(vec![HostPort::new(&host, port)])
            .with_payload_mapping(Some(mapping)))
    }

    /// Bind the exact policy this socket advertised to the original signed plan.
    /// The parent supplies the actual launch session and original helper's public key.
    pub(super) fn enroll(
        self,
        plan: &ExecutionPlan,
        instance: &str,
        state_dir: PathBuf,
        session: [u8; 32],
        host_signer_public: [u8; 32],
    ) -> Result<EnrolledNativePeer> {
        let plan = AdmittedNetworkPlan::from_plan(plan, instance, &self.network_policy()?)?;
        ensure!(
            state_dir.is_absolute(),
            "original state directory must be absolute"
        );
        ensure!(session != [0; 32], "original endpoint session missing");
        let public_key = VerifyingKey::from_bytes(&host_signer_public)?;
        let host_spki = Sha256::digest(public_key.to_public_key_der()?.as_bytes()).into();
        Ok(EnrolledNativePeer {
            peer: self,
            plan,
            state_dir,
            session,
            host_spki,
        })
    }
}

pub(super) struct EnrolledNativePeer {
    peer: NativePeer,
    plan: AdmittedNetworkPlan,
    state_dir: PathBuf,
    session: [u8; 32],
    host_spki: [u8; 32],
}

#[derive(Debug, serde::Serialize)]
pub(super) struct NativePeerResult {
    pub received_bytes: usize,
    pub sent_bytes: usize,
    pub payload_sha256: String,
    pub tls_received_bytes: u64,
    pub tls_sent_bytes: u64,
    pub tls_setup_ns: u128,
    pub mapped_setup_ns: u128,
    pub elapsed_after_accept_ns: u128,
    pub application_thread_cpu_ns: u128,
    pub scope_commitment: String,
}

impl EnrolledNativePeer {
    pub(super) fn serve(self, until: Instant) -> Result<NativePeerResult> {
        let EnrolledNativePeer {
            peer,
            plan,
            state_dir,
            session,
            host_spki,
        } = self;
        let socket = loop {
            ensure!(
                Instant::now() < until,
                "original peer accept deadline expired"
            );
            match peer.listener.accept() {
                Ok((socket, _)) => break socket,
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(error.into()),
            }
        };
        let accepted_at = Instant::now();
        // The boot/accept budget can exceed the canonical TCP carrier cap.
        // Subdivide it once at acceptance, before identity and TLS effects.
        let until = until.min(accepted_at + TRANSFER_WINDOW);
        let cpu = cpu_time::ThreadTime::now();
        ensure!(
            socket.local_addr()? == peer.address,
            "wrong original peer listener"
        );
        let expected_session = spiralcore_transfer::hex(&session);
        // Runtime creates this public projection before the original endpoint
        // can dial. Read it from the parent's enrolled state, never from TLS.
        let guest_key = read_original_identity(&state_dir, &expected_session)?;
        let tls_started_at = Instant::now();
        let tls = TlsCarrier::accept(socket, &peer.identity, host_spki, until)?;
        let tls_setup_ns = tls_started_at.elapsed().as_nanos();
        let carrier_meter = tls.try_clone()?;
        let caps = Capabilities::small_v1();
        let facts = ScopeFacts {
            object_profile: caps.profile,
            policy_sha256: digest(&plan.policy_sha256)?,
            plan_id: plan.plan_id.0,
            plan_version: plan.plan_version,
            plan_nonce: digest(plan.plan_nonce.as_hex())?,
            bundle_sha256: digest(&plan.bundle_sha256)?,
            tenant: plan.tenant_id,
            instance: plan.instance_id,
            session: expected_session,
            guest_key,
            stream_id: 1,
            requested_host: peer.address.ip().to_string(),
            requested_port: peer.address.port(),
            actual_peer: peer.address,
            route: "egress".into(),
            initiator_spki: tls.peer_spki_sha256(),
            acceptor_spki: tls.local_spki_sha256(),
            tls_exporter: tls.exporter(),
        };
        let scopes = PinnedScopePolicy::new(1)?;
        scopes.enroll(&facts, host_spki, Role::Acceptor, until)?;
        let authority = scopes.authorize_scope(&facts, host_spki, Role::Acceptor)?;
        let scope_commitment = spiralcore_transfer::hex(&facts.commitment()?);
        let namespace = spiralcore_transfer::hex(&authority.context().namespace()?);

        // One separate cooperating peer, using the existing SmallV1 fixture
        // envelope. This is not part of the installed child's partition.
        let resource_profile = ResourceProfile::SmallV1;
        let pool = PayloadResourcePool::for_qualification(PayloadResourceLimits {
            max_active: 1,
            resident_bytes: 8 << 20,
            disk_bytes: 32 << 20,
        })?;
        let permit = pool.reserve_profile(resource_profile)?;
        let store = tempfile::tempdir()?;
        let path = store.path().join("peer");
        let profile = caps.negotiate(&caps)?;
        let sender = ObjectSender::new_with_resource_profile(
            ObjectMode::Whole,
            authority.context(),
            None,
            profile,
            resource_profile,
        )?;
        let receiver = ObjectEndpoint::open_with_resource_profile(
            &path,
            resource_profile.receiver_limits(),
            authority.clone(),
            profile,
            resource_profile,
        )?;
        let prepared = PreparedPayloadSession::new(
            authority,
            sender,
            receiver,
            permit.with_owned_store(path.clone())?,
        )?;
        let mapped_started_at = Instant::now();
        let mut mapped = accept_enrolled_payload(tls, prepared, until)?;
        let mapped_setup_ns = mapped_started_at.elapsed().as_nanos();

        let line = format!("{{\"kind\":\"flowmux\",\"value\":{}}}\n", peer.value);
        let expected = line.as_bytes().repeat(REPETITIONS);
        let mut received = vec![0; expected.len()];
        mapped.read_exact(&mut received)?;
        ensure!(Instant::now() < until, "peer request deadline expired");
        ensure!(received == expected, "guest request bytes differ");
        check_setup_artifacts(&path.join(namespace))?;
        // Echo once the complete expected request is verified. A client may
        // wait for the reply before closing its proxy input.
        mapped.write_all(&received)?;
        mapped.shutdown(Shutdown::Write)?;
        let mut extra = [0; 1];
        ensure!(
            mapped.read(&mut extra)? == 0,
            "guest sent trailing application data"
        );
        drop(mapped);
        ensure!(Instant::now() < until, "peer transfer deadline expired");
        pool.close_admission()?;
        ensure!(pool.drained()?, "peer reader or permit still retained");
        ensure!(
            !path.exists(),
            "peer namespace survived resource retirement"
        );
        match peer.listener.accept() {
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
            Ok(_) => anyhow::bail!("witness opened a second connection"),
        }
        let tls = carrier_meter.counters();
        Ok(NativePeerResult {
            received_bytes: received.len(),
            sent_bytes: received.len(),
            payload_sha256: hex::encode(Sha256::digest(&received)),
            tls_received_bytes: tls.received,
            tls_sent_bytes: tls.sent,
            tls_setup_ns,
            mapped_setup_ns,
            elapsed_after_accept_ns: accepted_at.elapsed().as_nanos(),
            application_thread_cpu_ns: cpu.elapsed().as_nanos(),
            scope_commitment,
        })
    }
}

fn read_original_identity(state_dir: &Path, expected_session: &str) -> Result<[u8; 32]> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
        .open(state_dir.join(PUBLIC_IDENTITY_FILE))
        .context("read original guest public identity")?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= IDENTITY_BYTES,
        "original guest public identity shape differs"
    );
    let mut bytes = Vec::new();
    file.take(IDENTITY_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= IDENTITY_BYTES,
        "original guest public identity grew"
    );
    let identity: InheritableIdentity = serde_json::from_slice(&bytes)?;
    ensure!(
        identity.session_id == expected_session,
        "original guest session differs"
    );
    let mut guest_key = [0; 32];
    ensure!(
        identity.guest_verifying_key_base64.len() == 44
            && STANDARD.decode_slice(identity.guest_verifying_key_base64, &mut guest_key)?
                == guest_key.len(),
        "original guest verifying key size differs"
    );
    VerifyingKey::from_bytes(&guest_key)?;
    Ok(guest_key)
}

fn digest<const N: usize>(encoded: &str) -> Result<[u8; N]> {
    let mut digest = [0; N];
    hex::decode_to_slice(encoded, &mut digest)?;
    Ok(digest)
}

fn check_setup_artifacts(namespace: &Path) -> Result<()> {
    let schema = spiralcore_transfer::schema::fit(&[b"{\"kind\":\"flowmux\",\"value\":0}\n"])?;
    let checkpoint = spiralcore_transfer::shared_runtime::State::seeded(32)?.encode()?;
    for artifact in [schema, checkpoint] {
        let id = spiralcore_transfer::hex(&spiralcore_transfer::object_id(&artifact));
        let mut bytes = Vec::new();
        File::open(namespace.join(format!("{id}.object")))?
            .take((artifact.len() + 129) as u64)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= artifact.len() + 128 && bytes.ends_with(&artifact),
            "verified setup artifact readback differs"
        );
    }
    Ok(())
}
