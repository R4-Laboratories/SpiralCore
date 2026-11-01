//! Bounded public fixtures and independent on-wire byte accounting.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{io, path::Path};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub fn cpu_us() -> io::Result<u64> {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: valid writable timespec; process CPU clock has no external side effects.
    if unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(t.tv_sec as u64 * 1_000_000 + t.tv_nsec as u64 / 1000)
}
pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn fixture(size: usize, kind: &str, seed: u64) -> Result<Vec<u8>> {
    if !(1..=64 * 1024 * 1024).contains(&size) || !["pattern", "opaque"].contains(&kind) {
        return Err(invalid("fixture limit or kind").into());
    }
    let mut out = Vec::with_capacity(size);
    for index in 0..size.div_ceil(32) {
        let block: [u8; 32] = if kind == "opaque" {
            let mut h = Sha256::new();
            h.update(b"spiralcore.gateway.fixture.v1");
            h.update(seed.to_be_bytes());
            h.update((index as u64).to_be_bytes());
            h.finalize().into()
        } else {
            *b"spiralcore patterned packet data"
        };
        out.extend_from_slice(&block[..32.min(size - out.len())]);
    }
    Ok(out)
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temporary, path)?;
    Ok(())
}
#[derive(Debug, Default, Serialize)]
pub struct Capture {
    pub packets: u64,
    pub ip_bytes: u64,
    pub udp_payload_bytes: u64,
    pub ethernet_bytes: u64,
    pub other_packets: u64,
}
pub fn summarize_pcap(data: &[u8]) -> Result<Capture> {
    if data.len() < 24 {
        return Err(invalid("truncated pcap header").into());
    }
    let little = match &data[..4] {
        [0xd4, 0xc3, 0xb2, 0xa1] | [0x4d, 0x3c, 0xb2, 0xa1] => true,
        [0xa1, 0xb2, 0xc3, 0xd4] | [0xa1, 0xb2, 0x3c, 0x4d] => false,
        _ => return Err(invalid("classic pcap required").into()),
    };
    let u32at = |b: &[u8]| -> u32 {
        let a: [u8; 4] = b.try_into().expect("checked slice");
        if little {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        }
    };
    if u32at(&data[20..24]) != 1 {
        return Err(invalid("Ethernet capture required").into());
    }
    let mut offset = 24;
    let mut out = Capture::default();
    while offset < data.len() {
        let header = data
            .get(offset..offset + 16)
            .ok_or_else(|| invalid("truncated record"))?;
        let captured = u32at(&header[8..12]) as usize;
        let original = u32at(&header[12..16]) as usize;
        if captured > original || captured > 65536 {
            return Err(invalid("invalid snap length").into());
        }
        offset += 16;
        let f = data
            .get(offset..offset + captured)
            .ok_or_else(|| invalid("truncated packet"))?;
        offset += captured;
        if f.len() < 14 {
            return Err(invalid("missing Ethernet header").into());
        }
        if f[12..14] != [8, 0] {
            out.other_packets += 1;
            continue;
        }
        if f.len() < 34 || f[14] >> 4 != 4 {
            return Err(invalid("truncated IPv4").into());
        }
        let ihl = usize::from(f[14] & 15) * 4;
        let length = usize::from(u16::from_be_bytes([f[16], f[17]]));
        if ihl < 20 || original < 14 + length || length < ihl || length > 1500 {
            return Err(invalid("invalid IP length").into());
        }
        if f[23] != 17 {
            out.other_packets += 1;
            continue;
        }
        if u16::from_be_bytes([f[20], f[21]]) & 0x3fff != 0 {
            return Err(invalid("fragmented underlay unsupported").into());
        }
        let u = f
            .get(14 + ihl..14 + ihl + 8)
            .ok_or_else(|| invalid("missing UDP header"))?;
        if u16::from_be_bytes([u[0], u[1]]) != 4433 && u16::from_be_bytes([u[2], u[3]]) != 4433 {
            out.other_packets += 1;
            continue;
        }
        let len = u16::from_be_bytes([u[4], u[5]]) as usize;
        if len < 8 || len + ihl != length {
            return Err(invalid("invalid UDP length").into());
        }
        out.packets += 1;
        out.ip_bytes += length as u64;
        out.udp_payload_bytes += (len - 8) as u64;
        out.ethernet_bytes += original as u64;
    }
    Ok(out)
}
