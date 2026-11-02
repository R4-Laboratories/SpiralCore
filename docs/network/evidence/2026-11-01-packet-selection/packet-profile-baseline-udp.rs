//! Deterministic packet microbenchmark: no network, authentication, or delivery timing claim.
use cpu_time::ProcessTime;
use serde_json::{json, Value};
use spiralcore_gateway_core::packet::{PacketDecoder, PacketEncoder, PacketMode, HEADER_LEN};
use std::{
    hint::black_box,
    io::{self, Write},
    time::{Duration, Instant},
};
use zstd::zstd_safe::{self, CCtx, CParameter};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn vectors(ipv6: bool, size: usize, workload: &str, count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|index| {
            let mut packet = vec![0; size];
            let header = if ipv6 { 40 } else { 20 };
            let opaque = workload == "opaque"
                || workload == "mixed" && index % 2 == 1
                || workload == "guard_miss_transition" && index < count / 2;
            let mut state = 0xd1b54a32d192ed03u64.wrapping_mul(index as u64 + 1);
            let mut pattern = [0; 16];
            for byte in &mut pattern {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *byte = state as u8;
            }
            for (offset, byte) in packet[header + 8..].iter_mut().enumerate() {
                if opaque {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    *byte = state as u8;
                } else if workload == "repeated" {
                    *byte = b"SpiralCore packet pattern"[offset % 24];
                } else if workload == "guard_miss_transition" {
                    // Seven-byte matches compress, but the changing eighth byte avoids
                    // repeated eight-byte words at the selector's four-byte scan steps.
                    *byte = if offset % 8 == 7 {
                        (offset / 8) as u8
                    } else {
                        b"SCGWAY!"[offset % 8]
                    };
                } else {
                    *byte = pattern[offset % 16];
                }
            }
            if ipv6 {
                packet[0] = 0x60;
                packet[4..6].copy_from_slice(&((size - 40) as u16).to_be_bytes());
                packet[6] = 17;
                packet[7] = 64;
                packet[8] = 0xfd;
                packet[23] = 1;
                packet[24] = 0xfd;
                packet[39] = 2;
            } else {
                packet[0] = 0x45;
                packet[2..4].copy_from_slice(&(size as u16).to_be_bytes());
                packet[8] = 64;
                packet[9] = 17;
                packet[12..16].copy_from_slice(&[10, 0, 0, 1]);
                packet[16..20].copy_from_slice(&[10, 0, 0, 2]);
                let mut sum: u32 = packet[..20]
                    .chunks_exact(2)
                    .map(|b| u16::from_be_bytes([b[0], b[1]]) as u32)
                    .sum();
                while sum > 0xffff {
                    sum = (sum & 0xffff) + (sum >> 16);
                }
                packet[10..12].copy_from_slice(&(!(sum as u16)).to_be_bytes());
            }
            let udp_len = (size - header) as u16;
            packet[header..header + 2].copy_from_slice(&41000u16.to_be_bytes());
            packet[header + 2..header + 4].copy_from_slice(&42000u16.to_be_bytes());
            packet[header + 4..header + 6].copy_from_slice(&udp_len.to_be_bytes());
            let words = |bytes: &[u8]| -> u32 {
                bytes
                    .chunks(2)
                    .map(|b| u16::from_be_bytes([b[0], b.get(1).copied().unwrap_or(0)]) as u32)
                    .sum()
            };
            let mut sum = if ipv6 {
                words(&packet[8..40])
            } else {
                words(&packet[12..20])
            } + u32::from(udp_len)
                + 17
                + words(&packet[header..]);
            while sum > 0xffff {
                sum = (sum & 0xffff) + (sum >> 16);
            }
            let checksum = !(sum as u16);
            packet[header + 6..header + 8]
                .copy_from_slice(&if checksum == 0 { u16::MAX } else { checksum }.to_be_bytes());
            packet
        })
        .collect()
}

fn timed<T>(operation: impl FnOnce() -> Result<T>) -> Result<(T, u128, u128)> {
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let value = operation()?;
    let cpu_ns = cpu.elapsed().as_nanos();
    let wall_ns = wall.elapsed().as_nanos();
    Ok((value, cpu_ns, wall_ns))
}

fn decode(decoder: &mut PacketDecoder, frames: &[Vec<u8>]) -> Result<Vec<u8>> {
    let mut output = None;
    for frame in frames {
        if let Some(bytes) = decoder.receive(black_box(frame), Duration::ZERO)? {
            if output.replace(bytes).is_some() {
                return Err("multiple packet outputs".into());
            }
        }
    }
    output.ok_or_else(|| "incomplete packet".into())
}

fn packet_row(mode: PacketMode, packets: &[Vec<u8>], cap: usize) -> Result<Value> {
    let ((mut encoder, mut decoder), setup_cpu, setup_wall) =
        timed(|| Ok((PacketEncoder::new(mode)?, PacketDecoder::new()?)))?;
    let (first, first_encode_cpu, first_encode_wall) =
        timed(|| Ok(encoder.encode(black_box(&packets[0]), 0, cap)?))?;
    let (first_exact, first_decode_cpu, first_decode_wall) =
        timed(|| decode(&mut decoder, &first))?;
    if first_exact != packets[0] {
        return Err("first packet fidelity loss".into());
    }
    let mut frames = Vec::with_capacity(packets.len());
    let (_, encode_cpu, encode_wall) = timed(|| {
        for (index, packet) in packets.iter().enumerate() {
            frames.push(encoder.encode(black_box(packet), index as u64 + 1, cap)?);
        }
        Ok(())
    })?;
    let mut outputs = Vec::with_capacity(packets.len());
    let (_, decode_cpu, decode_wall) = timed(|| {
        for packet_frames in &frames {
            outputs.push(decode(&mut decoder, packet_frames)?);
        }
        Ok(())
    })?;
    if outputs != packets {
        return Err("packet fidelity loss".into());
    }
    let framed_bytes: usize = frames.iter().flatten().map(Vec::len).sum();
    let fragments: usize = frames.iter().map(Vec::len).sum();
    let compressed = frames.iter().filter(|f| f[0][20] == 1).count();
    Ok(json!({
        "stage":"packet_codec", "mode":format!("{mode:?}").to_lowercase(),
        "packets":packets.len(), "exact_packets":outputs.len(), "input_bytes":packets.iter().map(Vec::len).sum::<usize>(),
        "body_bytes":framed_bytes - fragments * HEADER_LEN,"framed_bytes":framed_bytes,"fragments":fragments,
        "raw_packets":packets.len() - compressed,"compressed_packets":compressed,
        "setup_cpu_ns":setup_cpu,"setup_wall_ns":setup_wall,
        "first_encode_cpu_ns":first_encode_cpu,"first_encode_wall_ns":first_encode_wall,
        "first_decode_cpu_ns":first_decode_cpu,"first_decode_wall_ns":first_decode_wall,
        "first_framed_bytes":first.iter().map(Vec::len).sum::<usize>(),
        "encode_cpu_ns":encode_cpu,"encode_wall_ns":encode_wall,
        "decode_cpu_ns":decode_cpu,"decode_wall_ns":decode_wall
    }))
}

// This diagnostic isolates zstd parameters. Both profiles use identical allocation,
// validation and context reuse; it deliberately excludes packet digest/framing CPU.
fn profile_row(packets: &[Vec<u8>], tuned: bool) -> Result<Value> {
    let mut encoder = CCtx::try_create().ok_or("zstd allocation")?;
    for parameter in [
        CParameter::CompressionLevel(if tuned { 1 } else { 3 }),
        CParameter::WindowLog(if tuned { 11 } else { 25 }),
        CParameter::EnableLongDistanceMatching(!tuned),
        CParameter::NbWorkers(0),
        CParameter::ContentSizeFlag(true),
        CParameter::ChecksumFlag(true),
        CParameter::DictIdFlag(false),
    ] {
        encoder
            .set_parameter(parameter)
            .map_err(|_| "zstd parameter")?;
    }
    let mut encoded = Vec::with_capacity(packets.len());
    let (_, cpu, wall) = timed(|| {
        for packet in packets {
            let mut output = Vec::with_capacity(zstd_safe::compress_bound(packet.len()));
            encoder
                .compress2(&mut output, black_box(packet))
                .map_err(|_| "zstd encode")?;
            if encoder.sizeof() > 128 * 1024 * 1024
                || output.get(..4) != Some(&[0x28, 0xb5, 0x2f, 0xfd])
                || output
                    .get(4)
                    .is_none_or(|descriptor| descriptor & 0x1b != 0 || descriptor & 4 == 0)
                || zstd_safe::get_frame_content_size(&output).map_err(|_| "zstd size")?
                    != Some(packet.len() as u64)
                || zstd_safe::find_frame_compressed_size(&output).map_err(|_| "zstd frame")?
                    != output.len()
            {
                return Err("zstd profile guard".into());
            }
            encoded.push(output);
        }
        Ok(())
    })?;
    let mut decoder = spiralcore_transfer::zstd_whole::PreparedDecoder::new()?;
    for (source, compressed) in packets.iter().zip(&encoded) {
        if decoder.decode(compressed, source.len())? != *source {
            return Err("zstd diagnostic fidelity loss".into());
        }
    }
    Ok(
        json!({"stage":"compression_profile_only","profile":if tuned {"level1_window11_no_ldm"} else {"level3_window25_ldm"},
        "packets":packets.len(),"exact_packets":packets.len(),"input_bytes":packets.iter().map(Vec::len).sum::<usize>(),
        "compressed_bytes_before_literal_fallback":encoded.iter().map(Vec::len).sum::<usize>(),
        "body_bytes_after_literal_fallback":encoded.iter().zip(packets).map(|(e,p)| e.len().min(p.len())).sum::<usize>(),
        "encode_cpu_ns":cpu,"encode_wall_ns":wall,"retained_zstd_bytes":encoder.sizeof()}),
    )
}

fn main() -> Result<()> {
    let mut trials = 3usize;
    let mut count = 512usize;
    let mut label = String::from("unlabelled");
    let mut modes = vec![PacketMode::Raw, PacketMode::Zstd];
    let mut args = std::env::args().skip(1);
    while let Some(option) = args.next() {
        let value = args.next().ok_or("missing argument value")?;
        match option.as_str() {
            "--trials" => trials = value.parse()?,
            "--packets" => count = value.parse()?,
            "--label" => label = value,
            "--modes" => {
                modes = value
                    .split(',')
                    .map(|mode| match mode {
                        "raw" => Ok(PacketMode::Raw),
                        "zstd" => Ok(PacketMode::Zstd),
                        _ => Err("unknown mode"),
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?;
            }
            _ => return Err("unknown option".into()),
        }
    }
    if !(1..=30).contains(&trials) || !(1..=4096).contains(&count) || modes.is_empty() {
        return Err("benchmark bounds".into());
    }
    let mut output = io::BufWriter::new(io::stdout().lock());
    writeln!(
        output,
        "{}",
        json!({"event":"start","version":1,"label":label,"arch":std::env::consts::ARCH,"os":std::env::consts::OS,
        "trials":trials,"packets_per_block":count,"warmup_trials":1,
        "timing":"process CPU and monotonic wall; packet blocks exclude generation, byte accounting, final equality comparison and JSON; include allocation and production verification; no network, QUIC or durability",
        "profiles":"compression-only diagnostic uses matched fresh output allocations and prepared CCtx; excludes packet digest/framing and context construction; includes first compression"})
    )?;
    for trial in 0..=trials {
        for ipv6 in [false, true] {
            for size in [64, 256, 1280] {
                for workload in [
                    "repeated",
                    "new_pattern",
                    "opaque",
                    "mixed",
                    "guard_miss_transition",
                ] {
                    let packets = vectors(ipv6, size, workload, count);
                    let slices: Vec<&[u8]> = packets.iter().map(Vec::as_slice).collect();
                    let fingerprint = spiralcore_transfer::hex(&spiralcore_transfer::hash(
                        b"spiralcore.packet.microbench.v1",
                        &slices,
                    ));
                    for cap in [700, 1336] {
                        for offset in 0..modes.len() {
                            let mode = modes[(offset + trial) % modes.len()];
                            let mut row = packet_row(mode, &packets, cap)?;
                            row["trial"] = json!(trial);
                            row["ipv6"] = json!(ipv6);
                            row["packet_bytes"] = json!(size);
                            row["workload"] = json!(workload);
                            row["max_datagram"] = json!(cap);
                            row["input_sha256"] = json!(fingerprint);
                            if trial > 0 {
                                writeln!(output, "{row}")?;
                            }
                        }
                    }
                    for offset in 0..2 {
                        let mut row = profile_row(&packets, (offset + trial) % 2 == 0)?;
                        row["trial"] = json!(trial);
                        row["ipv6"] = json!(ipv6);
                        row["packet_bytes"] = json!(size);
                        row["workload"] = json!(workload);
                        row["input_sha256"] = json!(fingerprint);
                        if trial > 0 {
                            writeln!(output, "{row}")?;
                        }
                    }
                }
            }
        }
    }
    writeln!(output, "{}", json!({"event":"complete","ok":true}))?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spiralcore_gateway_core::packet::{inspect_ip, PacketMode};

    #[test]
    fn vectors_are_repeatable_valid_ip_and_change_only_as_requested() {
        for ipv6 in [false, true] {
            for size in [64, 256, 1280] {
                for workload in ["repeated", "new_pattern", "opaque", "mixed"] {
                    let packets = vectors(ipv6, size, workload, 16);
                    assert_eq!(packets, vectors(ipv6, size, workload, 16));
                    assert!(packets.iter().all(|packet| packet.len() == size));
                    assert!(packets
                        .iter()
                        .all(|packet| packet[if ipv6 { 6 } else { 9 }] == 17));
                    assert!(packets.iter().all(|packet| inspect_ip(packet)
                        .unwrap()
                        .source
                        .is_ipv6()
                        == ipv6));
                    if workload == "repeated" {
                        assert_eq!(packets[0], packets[1]);
                    } else {
                        assert_ne!(packets[0], packets[1]);
                    }
                }
            }
        }
    }

    #[test]
    fn raw_accounting_includes_each_fragment_and_verifies_every_packet() {
        let packets = vectors(true, 1280, "opaque", 16);
        let row = packet_row(PacketMode::Raw, &packets, 700).unwrap();
        assert_eq!(row["input_bytes"], 16 * 1280);
        assert_eq!(row["body_bytes"], 16 * 1280);
        assert_eq!(row["framed_bytes"], 16 * (1280 + 2 * 56));
        assert_eq!(row["exact_packets"], 16);
        assert_eq!(row["raw_packets"], 16);
        assert_eq!(row["compressed_packets"], 0);
    }
}
