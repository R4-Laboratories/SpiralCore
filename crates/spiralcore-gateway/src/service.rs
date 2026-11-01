use crate::measurement::{invalid, Result};
use serde::{Deserialize, Serialize};
use spiralcore_gateway_core::packet::inspect_ip;
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub role: String,
    pub bind: SocketAddr,
    pub peer: SocketAddr,
    pub server_name: String,
    pub certificate: PathBuf,
    pub private_key: PathBuf,
    pub trust_anchor: PathBuf,
    pub peer_certificate: PathBuf,
    pub tun: String,
    pub mode: String,
    pub local_addresses: Vec<IpAddr>,
    pub peer_addresses: Vec<IpAddr>,
    pub stats_path: PathBuf,
    pub ready_path: PathBuf,
    pub duration_secs: u64,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if !["listen", "connect"].contains(&self.role.as_str())
            || !["raw", "zstd"].contains(&self.mode.as_str())
            || !(1..=3600).contains(&self.duration_secs)
            || self.tun.is_empty()
            || self.tun.len() > 15
            || !self
                .tun
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            || self.local_addresses.is_empty()
            || self.peer_addresses.is_empty()
            || self.local_addresses.len() > 16
            || self.peer_addresses.len() > 16
            || self.server_name.is_empty()
            || self.stats_path == self.ready_path
        {
            return Err(invalid("invalid gateway configuration").into());
        }
        if self
            .local_addresses
            .iter()
            .chain(&self.peer_addresses)
            .any(|a| a.is_unspecified() || a.is_multicast())
            || self
                .local_addresses
                .iter()
                .any(|a| self.peer_addresses.contains(a))
        {
            return Err(invalid("ambiguous address authorization").into());
        }
        Ok(())
    }
}
pub fn allowed_packet(config: &Config, packet: &[u8], outbound: bool) -> bool {
    let Ok(a) = inspect_ip(packet) else {
        return false;
    };
    let (src, dst) = if outbound {
        (&config.local_addresses, &config.peer_addresses)
    } else {
        (&config.peer_addresses, &config.local_addresses)
    };
    src.contains(&a.source) && dst.contains(&a.destination)
}
/// Only an authenticated normal peer close or our elapsed run duration ends normally.
pub fn normal_transport_end(error: &crate::transport::Error, duration_elapsed: bool) -> bool {
    use crate::transport::Error;
    match error {
        Error::Deadline => duration_elapsed,
        Error::Connection(connection)
        | Error::Send(quinn::SendDatagramError::ConnectionLost(connection)) => {
            matches!(connection, quinn::ConnectionError::ApplicationClosed(close) if close.error_code == quinn::VarInt::from_u32(0))
        }
        _ => false,
    }
}
#[cfg(target_os = "linux")]
pub async fn run(config: Config) -> Result<()> {
    use crate::{
        measurement::{cpu_us, write_json},
        transport::{Identity, Listener, Peer},
        tun::Tun,
    };
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use sha2::{Digest, Sha256};
    use spiralcore_gateway_core::packet::{PacketDecoder, PacketEncoder, PacketMode};
    use tokio::time::{Duration, Instant};
    config.validate()?;
    let start = Instant::now();
    let cpu = cpu_us()?;
    let read = |p: &std::path::Path| -> Result<Vec<u8>> {
        let m = std::fs::metadata(p)?;
        if m.len() > 64 * 1024 {
            return Err(invalid("identity file too large").into());
        }
        Ok(std::fs::read(p)?)
    };
    let identity = Identity {
        certificate_chain: vec![CertificateDer::from(read(&config.certificate)?)],
        private_key: PrivateKeyDer::try_from(read(&config.private_key)?).map_err(invalid)?,
        trusted_roots: vec![CertificateDer::from(read(&config.trust_anchor)?)],
        peer_leaf_sha256: Sha256::digest(read(&config.peer_certificate)?).into(),
    };
    let tun = Tun::open(&config.tun)?;
    let mut encoder = PacketEncoder::new(if config.mode == "raw" {
        PacketMode::Raw
    } else {
        PacketMode::Zstd
    })?;
    let mut decoder = PacketDecoder::new()?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let peer = if config.role == "listen" {
        Listener::bind(config.bind, identity)?
            .accept(deadline)
            .await?
    } else {
        Peer::connect(
            config.bind,
            config.peer,
            &config.server_name,
            identity,
            deadline,
        )
        .await?
    };
    let setup_wall_us = start.elapsed().as_micros() as u64;
    let setup_cpu_us = cpu_us()? - cpu;
    write_json(
        &config.ready_path,
        &serde_json::json!({"ready":true,"mtu":1280,"mode":config.mode,"setup_wall_us":setup_wall_us,"setup_cpu_us":setup_cpu_us}),
    )?;
    let end = Instant::now() + Duration::from_secs(config.duration_secs);
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sequence = 0u64;
    let mut input_bytes = 0u64;
    let mut output_bytes = 0u64;
    let mut framed_sent = 0u64;
    let mut framed_received = 0u64;
    let mut denied = 0u64;
    let mut invalid_frames = 0u64;
    let mut encode_cpu = 0u64;
    let mut decode_cpu = 0u64;
    let result:Result<()> = async {
 'run: loop {tokio::select! {
 _=term.recv()=>break,
 _=tokio::signal::ctrl_c()=>break,
 _=tokio::time::sleep_until(end)=>break,
 packet=tun.read_packet()=>{
 let packet=packet?;if !allowed_packet(&config,&packet,true){denied+=1;continue;}
 let c=cpu_us()?;let frames=encoder.encode(&packet,sequence,peer.max_datagram_size())?;encode_cpu+=cpu_us()? - c;
 sequence=sequence.checked_add(1).ok_or_else(||invalid("packet sequence exhausted"))?;input_bytes+=packet.len() as u64;
 for frame in frames {
 let length=frame.len() as u64;
 match peer.send_datagram(frame.into(),(Instant::now()+Duration::from_secs(5)).min(end)).await {
 Ok(())=>{framed_sent+=length;},
 Err(error) if normal_transport_end(&error,Instant::now()>=end)=>break 'run,
 Err(error)=>return Err(error.into()),
 }
 }
 },
 wire=peer.receive_datagram(end)=>{
 let wire=match wire {Ok(wire)=>wire,Err(error) if normal_transport_end(&error,Instant::now()>=end)=>break 'run,Err(error)=>return Err(error.into())};framed_received+=wire.len() as u64;let c=cpu_us()?;
 let decoded=decoder.receive(&wire,start.elapsed());decode_cpu+=cpu_us()? - c;
 match decoded {Ok(Some(packet))=>{
 if !allowed_packet(&config,&packet,false){denied+=1;continue;}
 match tokio::time::timeout_at(end,tun.write_packet(&packet)).await {Ok(result)=>result?,Err(_)=>break 'run}
 output_bytes+=packet.len() as u64;
 },Ok(None)=>{},Err(_)=>{invalid_frames+=1;}}
 }
 }} Ok(()) }.await;
    let u = peer.udp_counters();
    let p = decoder.counters();
    write_json(
        &config.stats_path,
        &serde_json::json!({"version":1,"mode":config.mode,"status":if result.is_ok(){"ok"}else{"error"},"error":result.as_ref().err().map(ToString::to_string),"setup_wall_us":setup_wall_us,"setup_cpu_us":setup_cpu_us,"wall_us":start.elapsed().as_micros() as u64,"process_cpu_us":cpu_us()? - cpu,"encode_cpu_us":encode_cpu,"decode_cpu_us":decode_cpu,"input_ip_bytes":input_bytes,"output_ip_bytes":output_bytes,"framed_sent_bytes":framed_sent,"framed_received_bytes":framed_received,"denied_packets":denied,"invalid_frames":invalid_frames,"accepted_packets":p.accepted_packets,"expired_packets":p.expired_packets,"capacity_drops":p.capacity_drops,"replay_drops":p.replay_drops,"udp_sent_bytes":u.sent_bytes,"udp_received_bytes":u.received_bytes,"udp_sent_datagrams":u.sent_datagrams,"udp_received_datagrams":u.received_datagrams,"counter_interval":"startup through snapshot before close; capture includes close separately"}),
    )?;
    peer.close();
    result
}
