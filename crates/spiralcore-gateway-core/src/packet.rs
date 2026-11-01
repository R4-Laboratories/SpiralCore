//! Independent packet encoding; no cross-packet bases or delivery acknowledgments.
use spiralcore_transfer::{hash, zstd_whole, Error, Id, Result};
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};

pub const MAX_IP_PACKET: usize = 1280;
pub const HEADER_LEN: usize = 56;
pub const MAX_DATAGRAM: usize = MAX_IP_PACKET + HEADER_LEN;
pub const MIN_DATAGRAM: usize = 700;
const MAGIC: &[u8; 8] = b"SCGPKT01";
const MAX_PENDING: usize = 64;
const MAX_REASSEMBLY: usize = 128 * 1024;
const TTL: Duration = Duration::from_secs(2);
// Conservative per-entry accounting for Assembly, map links and allocation metadata;
// the shared outer process budget remains distinct from this reassembly charge.
const METADATA_CHARGE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketMode {
    Raw,
    Zstd,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketAddresses {
    pub source: IpAddr,
    pub destination: IpAddr,
}
pub fn inspect_ip(packet: &[u8]) -> Result<PacketAddresses> {
    if packet.len() > MAX_IP_PACKET {
        return Err(Error::Limit);
    }
    match packet.first().map(|b| b >> 4) {
        Some(4) if packet.len() >= 20 => {
            let header = usize::from(packet[0] & 15) * 4;
            let length = usize::from(u16::from_be_bytes([packet[2], packet[3]]));
            if header < 20 || header > packet.len() || length != packet.len() {
                return Err(Error::Malformed);
            }
            Ok(PacketAddresses {
                source: IpAddr::V4(Ipv4Addr::new(
                    packet[12], packet[13], packet[14], packet[15],
                )),
                destination: IpAddr::V4(Ipv4Addr::new(
                    packet[16], packet[17], packet[18], packet[19],
                )),
            })
        }
        Some(6) if packet.len() >= 40 => {
            if usize::from(u16::from_be_bytes([packet[4], packet[5]])) + 40 != packet.len() {
                return Err(Error::Malformed);
            }
            let source: [u8; 16] = packet[8..24].try_into().map_err(|_| Error::Malformed)?;
            let destination: [u8; 16] = packet[24..40].try_into().map_err(|_| Error::Malformed)?;
            Ok(PacketAddresses {
                source: IpAddr::V6(Ipv6Addr::from(source)),
                destination: IpAddr::V6(Ipv6Addr::from(destination)),
            })
        }
        _ => Err(Error::Malformed),
    }
}
fn digest(packet: &[u8]) -> Id {
    hash(
        b"spiralcore.gateway.packet.v1\0",
        &[&(packet.len() as u16).to_be_bytes(), packet],
    )
}
pub struct PacketEncoder {
    encoder: Option<zstd_whole::PreparedEncoder>,
}
impl PacketEncoder {
    pub fn new(mode: PacketMode) -> Result<Self> {
        Ok(Self {
            encoder: match mode {
                PacketMode::Raw => None,
                PacketMode::Zstd => Some(zstd_whole::PreparedEncoder::new()?),
            },
        })
    }
    pub fn encode(
        &mut self,
        packet: &[u8],
        sequence: u64,
        max_datagram: usize,
    ) -> Result<Vec<Vec<u8>>> {
        inspect_ip(packet)?;
        if max_datagram < MIN_DATAGRAM {
            return Err(Error::Limit);
        }
        let capacity = max_datagram.min(MAX_DATAGRAM) - HEADER_LEN;
        let compressed = self
            .encoder
            .as_mut()
            .map(|e| e.encode(packet))
            .transpose()?;
        let (codec, body) = match compressed.as_deref() {
            Some(bytes) if bytes.len() < packet.len() => (1u8, bytes),
            _ => (0u8, packet),
        };
        let count = body.len().div_ceil(capacity);
        if !(1..=2).contains(&count) {
            return Err(Error::Limit);
        }
        let digest = digest(packet);
        let mut frames = Vec::with_capacity(count);
        for (index, chunk) in body.chunks(capacity).enumerate() {
            let mut frame = Vec::with_capacity(HEADER_LEN + chunk.len());
            frame.extend_from_slice(MAGIC);
            frame.extend_from_slice(&sequence.to_be_bytes());
            frame.extend_from_slice(&(packet.len() as u16).to_be_bytes());
            frame.extend_from_slice(&(body.len() as u16).to_be_bytes());
            frame.extend_from_slice(&[codec, index as u8, count as u8, 0]);
            frame.extend_from_slice(&digest);
            frame.extend_from_slice(chunk);
            frames.push(frame);
        }
        Ok(frames)
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PacketCounters {
    pub accepted_packets: u64,
    pub duplicate_fragments: u64,
    pub conflicting_fragments: u64,
    pub expired_packets: u64,
    pub capacity_drops: u64,
    pub replay_drops: u64,
    pub invalid_packets: u64,
}
#[derive(Clone, PartialEq, Eq)]
struct Metadata {
    sequence: u64,
    original: usize,
    encoded: usize,
    codec: u8,
    count: usize,
    digest: Id,
}
fn parse(wire: &[u8]) -> Result<(Metadata, usize, &[u8])> {
    if !(HEADER_LEN + 1..=MAX_DATAGRAM).contains(&wire.len()) || &wire[..8] != MAGIC {
        return Err(Error::Malformed);
    }
    let sequence = u64::from_be_bytes(wire[8..16].try_into().map_err(|_| Error::Malformed)?);
    let original = usize::from(u16::from_be_bytes([wire[16], wire[17]]));
    let encoded = usize::from(u16::from_be_bytes([wire[18], wire[19]]));
    let codec = wire[20];
    let index = usize::from(wire[21]);
    let count = usize::from(wire[22]);
    if !(20..=MAX_IP_PACKET).contains(&original)
        || encoded == 0
        || encoded > original
        || !(1..=2).contains(&count)
        || index >= count
        || wire[23] != 0
        || codec > 1
        || (codec == 0 && encoded != original)
        || (codec == 1 && encoded >= original)
    {
        return Err(Error::Malformed);
    }
    let body = &wire[HEADER_LEN..];
    if body.len() > encoded
        || (count == 1 && body.len() != encoded)
        || (count == 2 && body.len() >= encoded)
    {
        return Err(Error::Malformed);
    }
    let digest = wire[24..56].try_into().map_err(|_| Error::Malformed)?;
    Ok((
        Metadata {
            sequence,
            original,
            encoded,
            codec,
            count,
            digest,
        },
        index,
        body,
    ))
}
struct Assembly {
    metadata: Metadata,
    since: Duration,
    parts: [Option<Vec<u8>>; 2],
}
pub struct PacketDecoder {
    decoder: zstd_whole::PreparedDecoder,
    pending: BTreeMap<u64, Assembly>,
    charged: usize,
    now: Duration,
    highest: Option<u64>,
    seen: u64,
    counters: PacketCounters,
}
impl PacketDecoder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            decoder: zstd_whole::PreparedDecoder::new()?,
            pending: BTreeMap::new(),
            charged: 0,
            now: Duration::ZERO,
            highest: None,
            seen: 0,
            counters: PacketCounters::default(),
        })
    }
    pub fn counters(&self) -> PacketCounters {
        self.counters
    }
    pub fn receive(&mut self, wire: &[u8], now: Duration) -> Result<Option<Vec<u8>>> {
        let result = self.receive_inner(wire, now);
        if result.is_err() {
            self.counters.invalid_packets = self.counters.invalid_packets.saturating_add(1);
        }
        result
    }
    fn remove(&mut self, sequence: u64) -> Option<Assembly> {
        let a = self.pending.remove(&sequence)?;
        self.charged -= a.metadata.encoded + METADATA_CHARGE;
        Some(a)
    }
    fn receive_inner(&mut self, wire: &[u8], now: Duration) -> Result<Option<Vec<u8>>> {
        if now < self.now {
            return Err(Error::Sequence);
        }
        self.now = now;
        let expired: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, a)| now.saturating_sub(a.since) >= TTL)
            .map(|(seq, _)| *seq)
            .collect();
        for seq in expired {
            self.remove(seq);
            self.counters.expired_packets = self.counters.expired_packets.saturating_add(1);
        }
        let (metadata, index, body) = parse(wire)?;
        let sequence = metadata.sequence;
        if let Some(high) = self.highest {
            if sequence <= high {
                let distance = high - sequence;
                if distance >= 64 || self.seen & (1u64 << distance) != 0 {
                    self.counters.replay_drops = self.counters.replay_drops.saturating_add(1);
                    return Ok(None);
                }
            }
        }
        if let Some(a) = self.pending.get(&sequence) {
            if a.metadata != metadata
                || a.parts[index]
                    .as_deref()
                    .is_some_and(|existing| existing != body)
            {
                self.remove(sequence);
                self.counters.conflicting_fragments =
                    self.counters.conflicting_fragments.saturating_add(1);
                return Err(Error::Integrity);
            }
            if a.parts[index].is_some() {
                self.counters.duplicate_fragments =
                    self.counters.duplicate_fragments.saturating_add(1);
                return Ok(None);
            }
        } else {
            let charge = metadata.encoded + METADATA_CHARGE;
            if self.pending.len() >= MAX_PENDING
                || self
                    .charged
                    .checked_add(charge)
                    .is_none_or(|n| n > MAX_REASSEMBLY)
            {
                self.counters.capacity_drops = self.counters.capacity_drops.saturating_add(1);
                return Err(Error::Limit);
            }
            self.charged += charge;
            self.pending.insert(
                sequence,
                Assembly {
                    metadata: metadata.clone(),
                    since: now,
                    parts: [None, None],
                },
            );
        }
        let a = self.pending.get_mut(&sequence).ok_or(Error::Integrity)?;
        a.parts[index] = Some(body.to_vec());
        let received: usize = a.parts.iter().flatten().map(Vec::len).sum();
        if received > metadata.encoded {
            self.remove(sequence);
            return Err(Error::Integrity);
        }
        if a.parts[..metadata.count].iter().any(Option::is_none) {
            return Ok(None);
        }
        let a = self.remove(sequence).ok_or(Error::Integrity)?;
        if received != metadata.encoded {
            return Err(Error::Integrity);
        }
        let mut encoded = Vec::with_capacity(received);
        for part in a.parts.into_iter().flatten() {
            encoded.extend_from_slice(&part);
        }
        let packet = if metadata.codec == 0 {
            encoded
        } else {
            self.decoder.decode(&encoded, metadata.original)?
        };
        if packet.len() != metadata.original || digest(&packet) != metadata.digest {
            return Err(Error::Integrity);
        }
        inspect_ip(&packet)?;
        match self.highest {
            None => {
                self.highest = Some(sequence);
                self.seen = 1;
            }
            Some(high) if sequence > high => {
                let d = sequence - high;
                self.seen = if d >= 64 { 1 } else { (self.seen << d) | 1 };
                self.highest = Some(sequence);
            }
            Some(high) => {
                self.seen |= 1u64 << (high - sequence);
            }
        }
        self.counters.accepted_packets = self.counters.accepted_packets.saturating_add(1);
        Ok(Some(packet))
    }
}
