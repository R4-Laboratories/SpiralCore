#![forbid(unsafe_code)]
//! One synthetic installed-MVM peer. Stdin carries a single trusted enrollment.
//! The endpoint key stays local. TCP carries the existing authenticated mapping.
use ring::signature::Ed25519KeyPair;
use serde::Deserialize;
use serde_json::json;
use spiralcore_mvm_flowmux::{
    Duplex, MappedStream, ObjectChannel, Role,
    scope::{PinnedScopePolicy, ScopeFacts},
    tls::{PinnedIdentity, TlsCarrier},
};
use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
};
use std::{
    io::{self, BufRead, Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener},
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const ACCEPT: Duration = Duration::from_secs(75);
const TRANSFER: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Enrollment {
    control_id: [u8; 32],
    listener: SocketAddr,
    peer_spki: [u8; 32],
    host_spki: [u8; 32],
    policy_sha256: [u8; 32],
    plan_id: String,
    plan_version: u32,
    plan_nonce: [u8; 16],
    bundle_sha256: [u8; 32],
    tenant: String,
    instance: String,
    session: String,
    guest_key: [u8; 32],
    expires_unix_ms: u64,
}

fn require(ok: bool, error: &'static str) -> Result<()> {
    if !ok {
        return Err(error.into());
    }
    Ok(())
}

fn utc_ms() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .try_into()?)
}

fn registration(
    input: impl BufRead,
    control_id: [u8; 32],
    listener: SocketAddr,
    peer_spki: [u8; 32],
) -> Result<Enrollment> {
    let mut line = Vec::new();
    input.take(4097).read_until(b'\n', &mut line)?;
    require(
        line.len() <= 4096 && line.last() == Some(&b'\n'),
        "bad enrollment frame",
    )?;
    let row: Enrollment = serde_json::from_slice(&line)?;
    let now = utc_ms()?;
    require(
        row.control_id == control_id
            && row.listener == listener
            && row.peer_spki == peer_spki
            && row.host_spki != [0; 32]
            && row.expires_unix_ms > now
            && row.expires_unix_ms.saturating_sub(now) <= 75_000,
        "enrollment does not match live listener or validity",
    )?;
    Ok(row)
}

fn run(ip: Ipv4Addr, root: &Path, altered_scope: bool) -> Result<bool> {
    use ring::rand::{SecureRandom, SystemRandom};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    require(ip.is_private(), "expected explicit private LAN address")?;
    let metadata = std::fs::symlink_metadata(root)?;
    require(
        root.is_absolute()
            && root.canonicalize()? == root
            && metadata.is_dir()
            && metadata.permissions().mode() & 0o777 == 0o700
            && metadata.nlink() >= 2,
        "private peer workspace required",
    )?;
    let path = root.join("peer-store");
    require(!path.exists(), "fresh peer workspace required")?;
    let random = SystemRandom::new();
    let mut control_id = [0; 32];
    random
        .fill(&mut control_id)
        .map_err(|_| "random source failed")?;
    let pkcs8 =
        Ed25519KeyPair::generate_pkcs8(&random).map_err(|_| "identity generation failed")?;
    let identity = PinnedIdentity::from_pkcs8(pkcs8.as_ref().to_vec())?;
    let listener = TcpListener::bind((ip, 0))?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let started = Instant::now();
    println!(
        "{}",
        json!({"event":"ready","listener":address,"peer_spki":identity.spki_sha256(),"control_id":control_id})
    );
    io::stdout().flush()?;

    // The process supervisor also bounds this one private stdin read.
    let e = registration(
        io::stdin().lock(),
        control_id,
        address,
        identity.spki_sha256(),
    )?;
    let until = started + ACCEPT;
    let (socket, peer) = loop {
        require(
            Instant::now() < until && utc_ms()? < e.expires_unix_ms,
            "accept window expired",
        )?;
        match listener.accept() {
            Ok(pair) => break pair,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1))
            }
            Err(error) => return Err(error.into()),
        }
    };
    require(
        socket.local_addr()? == address,
        "connected socket does not match enrollment",
    )?;
    let accepted_at = Instant::now();
    let lifetime = Duration::from_millis(e.expires_unix_ms.saturating_sub(utc_ms()?));
    let deadline = until
        .min(accepted_at + TRANSFER)
        .min(accepted_at + lifetime);
    let process_cpu = cpu_time::ProcessTime::now();
    let thread_cpu = cpu_time::ThreadTime::now();
    let tls = TlsCarrier::accept(socket, &identity, e.host_spki, deadline)?;
    let tls_setup_ns = accepted_at.elapsed().as_nanos();
    let carrier_meter = tls.try_clone()?;
    let caps = Capabilities::small_v1();
    let facts = ScopeFacts {
        object_profile: caps.profile,
        policy_sha256: e.policy_sha256,
        plan_id: e.plan_id,
        plan_version: e.plan_version,
        plan_nonce: e.plan_nonce,
        bundle_sha256: e.bundle_sha256,
        tenant: e.tenant,
        instance: e.instance,
        session: e.session,
        guest_key: e.guest_key,
        stream_id: 1,
        requested_host: address.ip().to_string(),
        requested_port: address.port(),
        actual_peer: address,
        route: "egress".into(),
        initiator_spki: tls.peer_spki_sha256(),
        acceptor_spki: tls.local_spki_sha256(),
        tls_exporter: tls.exporter(),
    };
    let scopes = PinnedScopePolicy::new(1)?;
    scopes.enroll(&facts, e.host_spki, Role::Acceptor, deadline)?;
    let authority = scopes.authorize_scope(&facts, e.host_spki, Role::Acceptor)?;
    let context = authority.context().clone();
    let resource = ResourceProfile::SmallV1;
    let profile = caps.negotiate(&caps)?;
    let sender = ObjectSender::new_with_resource_profile(
        ObjectMode::Whole,
        &context,
        None,
        profile,
        resource,
    )?;
    let mut receiver = ObjectEndpoint::open_with_resource_profile(
        &path,
        resource.receiver_limits(),
        authority.clone(),
        profile,
        resource,
    )?;
    let mapped_at = Instant::now();
    let mut channel = ObjectChannel::from_authenticated(
        tls,
        &authority,
        deadline.saturating_duration_since(mapped_at),
        Role::Acceptor,
        caps,
        caps,
    )?;
    if altered_scope {
        require(
            matches!(
                channel.serve_once(&mut receiver, &context, 0),
                Err(spiralcore_mvm_flowmux::Error::InvalidBinding)
            ),
            "altered scope did not produce authenticated binding refusal",
        )?;
        drop(channel);
        drop(receiver);
        std::fs::remove_dir_all(&path)?;
        require(!path.exists(), "refused peer store remains")?;
        println!(
            "{}",
            json!({
                "event":"refused","stage":"authenticated_scope","error":"InvalidBinding",
                "application_bytes":0,"peer_store_removed":true,
            })
        );
        return Ok(true);
    }
    let mut mapped = MappedStream::from_duplex(Duplex::spawn(
        channel,
        sender,
        receiver,
        context.clone(),
        Role::Acceptor,
    )?);
    let mapped_setup_ns = mapped_at.elapsed().as_nanos();
    let expected = b"{\"kind\":\"flowmux\",\"value\":1}\n".repeat(400);
    let mut bytes = vec![0; expected.len()];
    mapped.read_exact(&mut bytes)?;
    require(bytes == expected, "remote readback differs")?;
    mapped.write_all(&bytes)?;
    mapped.shutdown(Shutdown::Write)?;
    require(mapped.read(&mut [0])? == 0, "trailing application bytes")?;
    mapped.close_and_join_until(deadline)?;
    drop(mapped);
    let mut readback = ObjectEndpoint::open_with_resource_profile(
        &path,
        resource.receiver_limits(),
        authority,
        profile,
        resource,
    )?;
    for artifact in [
        spiralcore_transfer::schema::fit(&[b"{\"kind\":\"flowmux\",\"value\":0}\n"])?,
        spiralcore_transfer::shared_runtime::State::seeded(32)?.encode()?,
    ] {
        require(
            readback.read_object(&context, spiralcore_transfer::object_id(&artifact))? == artifact,
            "exact setup artifact readback differs",
        )?;
    }
    drop(readback);
    std::fs::remove_dir_all(&path)?;
    require(
        !path.exists() && Instant::now() < deadline,
        "peer cleanup incomplete",
    )?;
    match listener.accept() {
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
        _ => return Err("unexpected second connection".into()),
    }
    let tls = carrier_meter.counters();
    use sha2::{Digest, Sha256};
    println!(
        "{}",
        json!({
            "event":"done","control_id":control_id,"peer":peer,"listener":address,
            "peer_store_removed":true,"setup_artifacts_exact":true,
        "result": {"received_bytes":bytes.len(),"sent_bytes":bytes.len(),
            "payload_sha256":spiralcore_transfer::hex(&Sha256::digest(&bytes).into()),
            "tls_received_bytes":tls.received,"tls_sent_bytes":tls.sent,"tls_setup_ns":tls_setup_ns,
            "mapped_setup_ns":mapped_setup_ns,"elapsed_after_accept_ns":accepted_at.elapsed().as_nanos(),
            "application_thread_cpu_ns":thread_cpu.elapsed().as_nanos(),
            "process_cpu_ns":process_cpu.elapsed().as_nanos(),
            "scope_commitment":spiralcore_transfer::hex(&facts.commitment()?),
            }
        })
    );
    Ok(false)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    require(
        args.len() == 4,
        "expected private IPv4, workspace and fixture case",
    )?;
    let altered_scope = match args[3].as_str() {
        "crosshost" => false,
        "altered-scope" => true,
        _ => return Err("unknown peer fixture case".into()),
    };
    if run(args[1].parse()?, Path::new(&args[2]), altered_scope)? {
        std::process::exit(3);
    }
    Ok(())
}
