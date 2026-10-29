//! Bounded classical-PCAP accounting. No packet payload is emitted or treated as a wire-byte proxy.
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::Path,
};
type Result<T> = std::result::Result<T, String>;
const MAX_PCAP: usize = 256 * 1024 * 1024;
const MAX_ROWS_FILE: usize = 16 * 1024 * 1024;
const MAX_CONNECTIONS: usize = 65536;
#[derive(Clone, Copy)]
struct Format {
    big: bool,
    nano: bool,
    snaplen: usize,
    link: u32,
}
impl Format {
    fn u16(self, b: &[u8]) -> u16 {
        let b = [b[0], b[1]];
        if self.big {
            u16::from_be_bytes(b)
        } else {
            u16::from_le_bytes(b)
        }
    }
    fn u32(self, b: &[u8]) -> u32 {
        let b = [b[0], b[1], b[2], b[3]];
        if self.big {
            u32::from_be_bytes(b)
        } else {
            u32::from_le_bytes(b)
        }
    }
}
#[derive(Default, Clone, PartialEq, Eq, Serialize)]
struct Counters {
    upload_ip_bytes: u64,
    download_ip_bytes: u64,
    unclassified_ip_bytes: u64,
    total_ip_bytes: u64,
    upload_packets: u64,
    download_packets: u64,
    unclassified_packets: u64,
    packets: u64,
    unknown_ip_length_packets: u64,
}
fn add(a: &mut u64, b: u64) -> Result<()> {
    *a = a.checked_add(b).ok_or("counter overflow")?;
    Ok(())
}
impl Counters {
    fn packet(&mut self, p: &Packet) -> Result<()> {
        add(&mut self.packets, 1)?;
        if let Some(n) = p.ip_length {
            add(&mut self.total_ip_bytes, n)?;
        } else {
            add(&mut self.unknown_ip_length_packets, 1)?;
        }
        match p.upload {
            Some(true) => {
                add(&mut self.upload_packets, 1)?;
                add(
                    &mut self.upload_ip_bytes,
                    p.ip_length.ok_or("missing IP length")?,
                )?;
            }
            Some(false) => {
                add(&mut self.download_packets, 1)?;
                add(
                    &mut self.download_ip_bytes,
                    p.ip_length.ok_or("missing IP length")?,
                )?;
            }
            None => {
                add(&mut self.unclassified_packets, 1)?;
                add(&mut self.unclassified_ip_bytes, p.ip_length.unwrap_or(0))?;
            }
        }
        Ok(())
    }
    fn sum(&mut self, other: &Self) -> Result<()> {
        add(&mut self.upload_ip_bytes, other.upload_ip_bytes)?;
        add(&mut self.download_ip_bytes, other.download_ip_bytes)?;
        add(&mut self.unclassified_ip_bytes, other.unclassified_ip_bytes)?;
        add(&mut self.total_ip_bytes, other.total_ip_bytes)?;
        add(&mut self.upload_packets, other.upload_packets)?;
        add(&mut self.download_packets, other.download_packets)?;
        add(&mut self.unclassified_packets, other.unclassified_packets)?;
        add(&mut self.packets, other.packets)?;
        add(
            &mut self.unknown_ip_length_packets,
            other.unknown_ip_length_packets,
        )
    }
}
struct Row {
    id: String,
    phase: String,
    start: u64,
    end: u64,
    counters: Counters,
    measurement_packets: u64,
    status: Option<String>,
    body_bytes: Option<(u64, u64)>,
}
fn decimal(v: Option<&Value>) -> Result<u64> {
    let s = v
        .and_then(Value::as_str)
        .ok_or("timestamp must be a decimal string")?;
    if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
        return Err("invalid decimal timestamp".into());
    }
    s.parse().map_err(|_| "timestamp overflow".into())
}
fn read_rows(text: &str) -> Result<Vec<Row>> {
    if text.len() > MAX_ROWS_FILE {
        return Err("rows file exceeds 16 MiB".into());
    }
    let mut rows: Vec<Row> = Vec::new();
    let mut keys = BTreeSet::new();
    for line in text.lines().filter(|s| !s.trim().is_empty()) {
        if rows.len() >= 4096 {
            return Err("more than 4096 rows".into());
        }
        let v: Value = serde_json::from_str(line).map_err(|e| format!("invalid row JSON: {e}"))?;
        let id = v
            .get("task")
            .and_then(|t| t.get("id"))
            .and_then(Value::as_str)
            .ok_or("missing task.id")?;
        let phase = v
            .get("phase")
            .and_then(Value::as_str)
            .ok_or("missing phase")?;
        if id.is_empty() || id.len() > 512 || !matches!(phase, "seed" | "transfer") {
            return Err("invalid task.id or phase".into());
        }
        if !keys.insert((id.to_string(), phase.to_string())) {
            return Err("duplicate task.id/phase".into());
        }
        let start = decimal(v.get("unix_start_nanos"))?;
        let end = decimal(v.get("unix_end_nanos"))?;
        if start > end {
            return Err("row ends before it starts".into());
        }
        if let Some(previous) = rows.last() {
            if start <= previous.start || start <= previous.end {
                return Err("rows are unordered or overlap".into());
            }
        }
        let status = v
            .get("status")
            .map(|s| s.as_str().map(str::to_owned).ok_or("invalid row status"))
            .transpose()?;
        let metric = |name| -> Result<Option<u64>> {
            v.get("metrics")
                .and_then(|m| m.get(name))
                .map(|n| n.as_u64().ok_or_else(|| "invalid body-byte metric".into()))
                .transpose()
        };
        let body_bytes = metric("request_body_bytes")?.zip(metric("response_body_bytes")?);
        rows.push(Row {
            id: id.into(),
            phase: phase.into(),
            start,
            end,
            counters: Counters::default(),
            measurement_packets: 0,
            status,
            body_bytes,
        });
    }
    if rows.is_empty() {
        return Err("at least one measurement row is required".into());
    }
    Ok(rows)
}
struct Packet {
    upload: Option<bool>,
    ip_length: Option<u64>,
    remote: Option<IpAddr>,
    unclassified: Option<&'static str>,
    tcp: Option<Tcp>,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ConnectionKey {
    local: IpAddr,
    remote: IpAddr,
    local_port: u16,
    remote_port: u16,
}
#[derive(Clone, Copy)]
struct Tcp {
    key: ConnectionKey,
    sequence: u32,
    flags: u8,
}
#[derive(Clone, Copy)]
enum Owner {
    Setup,
    Row(usize),
    OutsideMeasurement,
}
struct Connection {
    initial_sequence: u32,
    syn_nanos: u64,
    owner: Owner,
    ambiguous: bool,
}
fn syn_owner(rows: &[Row], nanos: u64) -> Owner {
    let index = rows.partition_point(|row| row.start <= nanos);
    if index == 0 {
        Owner::Setup
    } else if nanos <= rows[index - 1].end {
        Owner::Row(index - 1)
    } else {
        Owner::OutsideMeasurement
    }
}
fn unknown(length: Option<usize>, reason: &'static str) -> Packet {
    Packet {
        upload: None,
        ip_length: length.map(|n| n as u64),
        remote: None,
        unclassified: Some(reason),
        tcp: None,
    }
}
fn be16(b: &[u8], at: usize) -> Result<usize> {
    let b = b.get(at..at + 2).ok_or("truncated header")?;
    Ok(u16::from_be_bytes([b[0], b[1]]) as usize)
}
fn original_length(original: usize, offset: usize, ip: usize, ethernet: bool) -> Result<()> {
    let expected = offset.checked_add(ip).ok_or("packet length overflow")?;
    // Ethernet can retain minimum-frame padding and a four-byte FCS. Cooked headers have neither.
    let largest = if ethernet {
        expected
            .max(offset.checked_add(46).ok_or("packet length overflow")?)
            .checked_add(4)
            .ok_or("packet length overflow")?
    } else {
        expected
    };
    if original < expected || original > largest {
        return Err("original packet length conflicts with IP total length".into());
    }
    Ok(())
}
fn packet(
    b: &[u8],
    original: usize,
    link: u32,
    local: IpAddr,
    expected_remote: Option<IpAddr>,
) -> Result<Packet> {
    let (mut offset, mut ethertype) = match link {
        1 => {
            if b.len() < 14 {
                return Err("truncated Ethernet header".into());
            }
            (14, be16(b, 12)?)
        }
        113 => {
            if b.len() < 16 {
                return Err("truncated Linux SLL header".into());
            }
            (16, be16(b, 14)?)
        }
        276 => {
            if b.len() < 20 {
                return Err("truncated Linux SLL2 header".into());
            }
            (20, be16(b, 0)?)
        }
        _ => return Err("unsupported link type".into()),
    };
    let mut tags = 0;
    while link == 1 && matches!(ethertype, 0x8100 | 0x88a8 | 0x9100) {
        tags += 1;
        if tags > 2 {
            return Ok(unknown(None, "unsupported_vlan_depth"));
        }
        ethertype = be16(b, offset + 2)?;
        offset += 4;
    }
    let (ip_length, source, destination, mut protocol, mut transport, fragmented) = match ethertype
    {
        0x0800 => {
            let h = b.get(offset..offset + 20).ok_or("truncated IPv4 header")?;
            if h[0] >> 4 != 4 {
                return Err("invalid IPv4 version".into());
            }
            let header = (h[0] & 15) as usize * 4;
            if header < 20 {
                return Err("invalid IPv4 IHL".into());
            }
            b.get(offset..offset + header)
                .ok_or("truncated IPv4 options")?;
            let total = be16(h, 2)?;
            if total < header {
                return Err("IPv4 total length below header".into());
            }
            let source = IpAddr::V4(Ipv4Addr::new(h[12], h[13], h[14], h[15]));
            let destination = IpAddr::V4(Ipv4Addr::new(h[16], h[17], h[18], h[19]));
            (
                total,
                source,
                destination,
                h[9],
                offset + header,
                be16(h, 6)? & 0x3fff != 0,
            )
        }
        0x86dd => {
            let h = b.get(offset..offset + 40).ok_or("truncated IPv6 header")?;
            if h[0] >> 4 != 6 {
                return Err("invalid IPv6 version".into());
            }
            let payload = be16(h, 4)?;
            if payload == 0 {
                return Err("IPv6 jumbogram/zero payload is unsupported".into());
            }
            let source = IpAddr::V6(Ipv6Addr::from(
                <[u8; 16]>::try_from(&h[8..24]).map_err(|_| "invalid IPv6 source")?,
            ));
            let destination = IpAddr::V6(Ipv6Addr::from(
                <[u8; 16]>::try_from(&h[24..40]).map_err(|_| "invalid IPv6 destination")?,
            ));
            (40 + payload, source, destination, h[6], offset + 40, false)
        }
        _ => return Ok(unknown(None, "non_ip_packet")),
    };
    original_length(original, offset, ip_length, link == 1)?;
    let (upload, remote) = match (source == local, destination == local) {
        (true, false) => (true, destination),
        (false, true) => (false, source),
        _ => return Err("unexpected flow: exactly one IP endpoint must match local-ip".into()),
    };
    if expected_remote.is_some_and(|expected| expected != remote) {
        return Err("unexpected flow: remote IP differs from --remote-ip".into());
    }
    if fragmented {
        return Ok(unknown(Some(ip_length), "fragmented_ipv4"));
    }
    if ethertype == 0x86dd {
        let mut extensions = 0;
        while matches!(protocol, 0 | 43 | 44 | 51 | 60) {
            extensions += 1;
            if extensions > 8 {
                return Ok(unknown(Some(ip_length), "too_many_ipv6_extensions"));
            }
            let prefix = b
                .get(transport..transport + 2)
                .ok_or("truncated IPv6 extension")?;
            let length = match protocol {
                44 => 8,
                51 => (usize::from(prefix[1]) + 2) * 4,
                _ => (usize::from(prefix[1]) + 1) * 8,
            };
            if transport.checked_add(length).ok_or("extension overflow")? > offset + ip_length {
                return Err("IPv6 extension exceeds packet length".into());
            }
            b.get(transport..transport + length)
                .ok_or("truncated IPv6 extension")?;
            if protocol == 44 && be16(b, transport + 2)? & 0xfff9 != 0 {
                return Ok(unknown(Some(ip_length), "fragmented_ipv6"));
            }
            protocol = prefix[0];
            transport += length;
        }
    }
    if protocol == 50 {
        return Ok(unknown(Some(ip_length), "encrypted_ip_transport"));
    }
    if protocol != 6 {
        return Err("unexpected flow: non-TCP IP packet".into());
    }
    if transport + 20 > offset + ip_length {
        return Err("TCP header exceeds IP length".into());
    }
    let tcp = b
        .get(transport..transport + 20)
        .ok_or("truncated TCP base header")?;
    let header = usize::from(tcp[12] >> 4) * 4;
    if header < 20 || transport + header > offset + ip_length {
        return Err("invalid TCP data offset".into());
    }
    let source_port = be16(tcp, 0)?;
    let destination_port = be16(tcp, 2)?;
    if source_port == 0
        || destination_port == 0
        || (if upload {
            destination_port
        } else {
            source_port
        }) != 443
    {
        return Err("unexpected flow: remote TCP port is not 443".into());
    }
    Ok(Packet {
        upload: Some(upload),
        ip_length: Some(ip_length as u64),
        remote: Some(remote),
        unclassified: None,
        tcp: Some(Tcp {
            key: ConnectionKey {
                local,
                remote,
                local_port: (if upload {
                    source_port
                } else {
                    destination_port
                }) as u16,
                remote_port: 443,
            },
            sequence: u32::from_be_bytes([tcp[4], tcp[5], tcp[6], tcp[7]]),
            flags: tcp[13],
        }),
    })
}
// Walk the bounded input twice without retaining every packet. The first pass discovers SYN
// ownership and invalidates reused tuples; the second assigns bytes, including late teardown.
fn scan(
    pcap: &[u8],
    format: Format,
    local: IpAddr,
    expected_remote: Option<IpAddr>,
    mut visit: impl FnMut(u64, Packet) -> Result<()>,
) -> Result<()> {
    let mut pos = 24;
    while pos < pcap.len() {
        let h = pcap
            .get(pos..pos + 16)
            .ok_or("truncated PCAP packet header")?;
        pos += 16;
        let seconds = u64::from(format.u32(&h[..4]));
        let fraction = u64::from(format.u32(&h[4..8]));
        if fraction
            >= if format.nano {
                1_000_000_000
            } else {
                1_000_000
            }
        {
            return Err("packet timestamp fraction out of range".into());
        }
        let nanos = seconds
            .checked_mul(1_000_000_000)
            .and_then(|s| {
                fraction
                    .checked_mul(if format.nano { 1 } else { 1000 })
                    .and_then(|f| s.checked_add(f))
            })
            .ok_or("timestamp overflow")?;
        let captured = format.u32(&h[8..12]) as usize;
        let original = format.u32(&h[12..16]) as usize;
        if captured == 0 || captured > format.snaplen || captured > original || original > 1_048_576
        {
            return Err("invalid captured/original packet lengths".into());
        }
        let end = pos.checked_add(captured).ok_or("capture offset overflow")?;
        let data = pcap
            .get(pos..end)
            .ok_or("truncated captured packet bytes")?;
        pos = end;
        visit(
            nanos,
            packet(data, original, format.link, local, expected_remote)?,
        )?;
    }
    Ok(())
}
#[cfg(test)]
fn account(pcap: &[u8], text: &str, local: IpAddr, drops: Option<u64>) -> Result<Value> {
    account_expected(pcap, text, local, drops, None)
}
fn account_expected(
    pcap: &[u8],
    text: &str,
    local: IpAddr,
    drops: Option<u64>,
    expected_remote: Option<IpAddr>,
) -> Result<Value> {
    if pcap.len() > MAX_PCAP {
        return Err("pcap exceeds 256 MiB".into());
    }
    if pcap.len() < 24 {
        return Err("truncated pcap global header".into());
    }
    let (big, nano) = match &pcap[..4] {
        [0xd4, 0xc3, 0xb2, 0xa1] => (false, false),
        [0xa1, 0xb2, 0xc3, 0xd4] => (true, false),
        [0x4d, 0x3c, 0xb2, 0xa1] => (false, true),
        [0xa1, 0xb2, 0x3c, 0x4d] => (true, true),
        _ => return Err("unsupported PCAP magic; classical PCAP required".into()),
    };
    let mut format = Format {
        big,
        nano,
        snaplen: 0,
        link: 0,
    };
    if format.u16(&pcap[4..6]) != 2 || format.u16(&pcap[6..8]) != 4 {
        return Err("unsupported PCAP version".into());
    }
    format.snaplen = format.u32(&pcap[16..20]) as usize;
    format.link = format.u32(&pcap[20..24]);
    if format.snaplen == 0 || format.snaplen > 262144 || !matches!(format.link, 1 | 113 | 276) {
        return Err("unsupported snaplen or link type".into());
    }
    let mut rows = read_rows(text)?;
    let mut connections = BTreeMap::<ConnectionKey, Connection>::new();
    scan(pcap, format, local, expected_remote, |ns, packet| {
        if let Some(tcp) = packet
            .tcp
            .filter(|tcp| packet.upload == Some(true) && tcp.flags & 0x12 == 0x02)
        {
            if let Some(connection) = connections.get_mut(&tcp.key) {
                if connection.initial_sequence != tcp.sequence {
                    // Do not guess TCP epochs: quarantine every packet on the reused tuple.
                    connection.ambiguous = true;
                } else if ns < connection.syn_nanos {
                    connection.syn_nanos = ns;
                    connection.owner = syn_owner(&rows, ns);
                }
            } else {
                if connections.len() >= MAX_CONNECTIONS {
                    return Err("more than 65536 TCP connections".into());
                }
                connections.insert(
                    tcp.key,
                    Connection {
                        initial_sequence: tcp.sequence,
                        syn_nanos: ns,
                        owner: syn_owner(&rows, ns),
                        ambiguous: false,
                    },
                );
            }
        }
        Ok(())
    })?;
    let mut setup = Counters::default();
    let mut unattributed = Counters::default();
    let mut campaign = Counters::default();
    let mut reasons = BTreeMap::<&str, u64>::new();
    let mut attribution_reasons = BTreeMap::<&str, u64>::new();
    let mut remotes = BTreeSet::new();
    let mut first = None;
    let mut last = None;
    scan(pcap, format, local, expected_remote, |ns, packet| {
        first = Some(first.map_or(ns, |old: u64| old.min(ns)));
        last = Some(last.map_or(ns, |old: u64| old.max(ns)));
        if let Some(ip) = packet.remote {
            remotes.insert(ip.to_string());
        }
        if let Some(reason) = packet.unclassified {
            add(reasons.entry(reason).or_default(), 1)?;
        }
        campaign.packet(&packet)?;
        let owner = match packet.tcp {
            None => Err("unclassified_tcp_connection"),
            Some(tcp) => match connections.get(&tcp.key) {
                None => Err("no_observed_outbound_syn"),
                Some(connection) if connection.ambiguous => {
                    Err("reused_tuple_with_different_initial_sequence")
                }
                Some(connection) if ns < connection.syn_nanos => {
                    Err("packet_precedes_observed_syn")
                }
                Some(connection) => match connection.owner {
                    Owner::OutsideMeasurement => Err("syn_outside_active_measurement"),
                    owner => Ok(owner),
                },
            },
        };
        match owner {
            Ok(Owner::Setup) => setup.packet(&packet)?,
            Ok(Owner::Row(index)) => {
                let row = &mut rows[index];
                row.counters.packet(&packet)?;
                if ns >= row.start && ns <= row.end {
                    add(&mut row.measurement_packets, 1)?;
                }
            }
            Err(reason) => {
                unattributed.packet(&packet)?;
                add(attribution_reasons.entry(reason).or_default(), 1)?;
            }
            Ok(Owner::OutsideMeasurement) => return Err("invalid attribution state".into()),
        }
        Ok(())
    })?;
    let mut sum = setup.clone();
    sum.sum(&unattributed)?;
    for row in &rows {
        sum.sum(&row.counters)?;
    }
    if sum != campaign {
        return Err("campaign/partition sum mismatch".into());
    }
    let mut incomplete = Vec::new();
    match drops {
        None => incomplete.push("tcpdump_kernel_drop_counter_unavailable"),
        Some(0) => {}
        Some(_) => incomplete.push("tcpdump_reported_kernel_drops"),
    }
    if campaign.unclassified_packets > 0 {
        incomplete.push("unclassified_packets");
    }
    if unattributed.packets > 0 {
        incomplete.push("packets_without_unambiguous_connection_owner");
    }
    if campaign.packets == 0 {
        incomplete.push("empty_capture");
    }
    if rows.iter().any(|row| row.measurement_packets == 0) {
        incomplete.push("measurement_row_without_captured_packets");
    }
    if rows.iter().any(|row| {
        row.status.is_none()
            || (row.status.as_deref() == Some("published") && row.body_bytes.is_none())
    }) {
        incomplete.push("application_body_metrics_unavailable");
    }
    if rows.iter().any(|row| {
        row.status.as_deref() == Some("published")
            && row.body_bytes.is_some_and(|(upload, download)| {
                row.counters.upload_ip_bytes < upload || row.counters.download_ip_bytes < download
            })
    }) {
        incomplete.push("capture_below_published_application_body_bytes");
    }
    let output_rows = rows
        .iter()
        .map(|row| {
            let mut v = serde_json::to_value(&row.counters).map_err(|e| e.to_string())?;
            let map = v.as_object_mut().ok_or("invalid counter serialization")?;
            map.insert("task".into(), json!({"id":row.id}));
            map.insert("phase".into(), json!(row.phase));
            map.insert("unix_start_nanos".into(), json!(row.start.to_string()));
            map.insert("unix_end_nanos".into(), json!(row.end.to_string()));
            map.insert(
                "packets_within_measurement".into(),
                json!(row.measurement_packets),
            );
            map.insert("status".into(), json!(row.status));
            map.insert(
                "expected_application_body_bytes".into(),
                json!(row
                    .body_bytes
                    .map(|(upload, download)| json!({"upload":upload,"download":download}))),
            );
            map.insert(
                "application_body_coverage_verified".into(),
                json!(if row.status.as_deref() == Some("published") {
                    row.body_bytes.map(|(up, down)| {
                        row.counters.upload_ip_bytes >= up && row.counters.download_ip_bytes >= down
                    })
                } else {
                    None
                }),
            );
            Ok(v)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({
        "schema":"spiralcore.pcap-accounting.v1","scope":"runner_to_relay_leg_only","local_ip":local.to_string(),
        "byte_layer":"IP total lengths, not captured snapshot lengths or physical Ethernet wire bytes",
        "includes":"captured TLS/HTTP traffic, TCP ACKs and retransmissions","excludes":"relay-to-receiver leg, relay internals, uncaptured/dropped traffic",
        "capture_complete":incomplete.is_empty(),"completeness_scope":"parsed selected-flow records have unambiguous observed-SYN ownership; every measurement has packets and published body-byte coverage; tcpdump reports zero kernel drops; offload state and hostname filter require separate provenance",
        "incomplete_reasons":incomplete,"kernel_reported_drops":drops,"observed_remote_ips":remotes,"expected_remote_ip":expected_remote.map(|ip|ip.to_string()),
        "capture_first_packet_unix_nanos":first.map(|n|n.to_string()),"capture_last_packet_unix_nanos":last.map(|n|n.to_string()),
        "timestamp_resolution":if nano{"nanoseconds"}else{"microseconds"},"link_type":format.link,"snaplen":format.snaplen,
        "partition_rule":"outbound SYN assigns a TCP four-tuple to the containing inclusive measurement interval, or setup before the first row; all subsequent connection packets retain that owner through PCAP EOF; missing SYN, changed initial sequence, and SYN outside active intervals are unattributed",
        "attribution_precondition":"client opens fresh connections per measurement; connections are not shared between operations",
        "observed_syn_connections":connections.len(),"ambiguous_connections":connections.values().filter(|connection|connection.ambiguous).count(),
        "setup":setup,"rows":output_rows,"unattributed":unattributed,"campaign":campaign,"partition_sum_verified":true,"unclassified_reasons":reasons,"unattributed_reasons":attribution_reasons
    }))
}
fn dropped(text: &str) -> Result<Option<u64>> {
    let mut found = None;
    for line in text.lines() {
        let words = line.split_whitespace().collect::<Vec<_>>();
        if words.len() == 5
            && matches!(words[1], "packet" | "packets")
            && words[2..] == ["dropped", "by", "kernel"]
        {
            if found.is_some() {
                return Err("multiple tcpdump drop counters".into());
            }
            found = Some(
                words[0]
                    .parse()
                    .map_err(|_| "invalid tcpdump drop counter")?,
            );
        }
    }
    Ok(found)
}
fn bounded_read(path: &Path, max: usize) -> Result<Vec<u8>> {
    let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    if len > max as u64 {
        return Err("input file exceeds size limit".into());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len as usize)
        .map_err(|_| "input allocation failed")?;
    let mut buffer = [0; 65536];
    loop {
        let remaining = max + 1 - bytes.len();
        let n = file
            .read(&mut buffer[..remaining.min(65536)])
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(bytes);
        }
        bytes
            .try_reserve_exact(n)
            .map_err(|_| "input allocation failed")?;
        bytes.extend_from_slice(&buffer[..n]);
        if bytes.len() > max {
            return Err("input file exceeds size limit".into());
        }
    }
}
fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let mut options = BTreeMap::new();
    while let Some(k) = args.next() {
        let k = k.into_string().map_err(|_| "option must be UTF-8")?;
        if k == "--help" {
            println!("spiralcore-capture --pcap FILE --rows JSONL --local-ip IP --out NEW_JSON [--capture-stats FILE] [--remote-ip IP]\nClassical PCAP only, at most256 MiB; <=4096 ordered rows; header-only IP-byte accounting for the runner-to-relay leg.");
            return Ok(());
        }
        if !matches!(
            k.as_str(),
            "--pcap" | "--rows" | "--local-ip" | "--out" | "--capture-stats" | "--remote-ip"
        ) {
            return Err(format!("unknown option {k}"));
        }
        let v = args.next().ok_or("missing option value")?;
        if options.insert(k, v).is_some() {
            return Err("duplicate option".into());
        }
    }
    let pcap = options.remove("--pcap").ok_or("missing --pcap")?;
    let rows = options.remove("--rows").ok_or("missing --rows")?;
    let local = options
        .remove("--local-ip")
        .ok_or("missing --local-ip")?
        .into_string()
        .map_err(|_| "invalid IP")?
        .parse()
        .map_err(|_| "invalid IP")?;
    let out = options.remove("--out").ok_or("missing --out")?;
    let drops = options
        .remove("--capture-stats")
        .map(|p| {
            bounded_read(Path::new(&p), 1024 * 1024)
                .and_then(|b| {
                    String::from_utf8(b).map_err(|_| "capture stats must be UTF-8".into())
                })
                .and_then(|s| dropped(&s))
        })
        .transpose()?
        .flatten();
    let bytes = bounded_read(Path::new(&pcap), MAX_PCAP)?;
    let rows = String::from_utf8(bounded_read(Path::new(&rows), MAX_ROWS_FILE)?)
        .map_err(|_| "rows must be UTF-8")?;
    let remote = options
        .remove("--remote-ip")
        .map(|ip| {
            ip.into_string()
                .map_err(|_| "invalid remote IP".to_string())
                .and_then(|s| {
                    s.parse::<IpAddr>()
                        .map_err(|_| "invalid remote IP".to_string())
                })
        })
        .transpose()?;
    let report = account_expected(&bytes, &rows, local, drops, remote)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut output, &report).map_err(|e| e.to_string())?;
    output.write_all(b"\n").map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("ERROR {error}");
        std::process::exit(2);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn rows() -> String {
        [serde_json::json!({"task":{"id":"a"},"phase":"seed","status":"published","metrics":{"request_body_bytes":100,"response_body_bytes":0},"unix_start_nanos":"1000","unix_end_nanos":"1900"}),
         serde_json::json!({"task":{"id":"a"},"phase":"transfer","status":"published","metrics":{"request_body_bytes":0,"response_body_bytes":100},"unix_start_nanos":"3000","unix_end_nanos":"3900"})]
            .iter().map(Value::to_string).collect::<Vec<_>>().join("\n")
    }
    fn local() -> IpAddr {
        "192.0.2.10".parse().unwrap()
    }
    fn packet(upload: bool, ip_length: u16) -> Vec<u8> {
        let mut b = vec![0; 14 + usize::from(ip_length)];
        b[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        b[14] = 0x45;
        b[16..18].copy_from_slice(&ip_length.to_be_bytes());
        b[23] = 6;
        let (source, destination, sp, dp) = if upload {
            ([192, 0, 2, 10], [198, 51, 100, 1], 50000u16, 443u16)
        } else {
            ([198, 51, 100, 1], [192, 0, 2, 10], 443u16, 50000u16)
        };
        b[26..30].copy_from_slice(&source);
        b[30..34].copy_from_slice(&destination);
        b[34..36].copy_from_slice(&sp.to_be_bytes());
        b[36..38].copy_from_slice(&dp.to_be_bytes());
        b[46] = 0x50;
        b
    }
    fn put16(out: &mut Vec<u8>, n: u16, big: bool) {
        out.extend_from_slice(&if big {
            n.to_be_bytes()
        } else {
            n.to_le_bytes()
        });
    }
    fn put32(out: &mut Vec<u8>, n: u32, big: bool) {
        out.extend_from_slice(&if big {
            n.to_be_bytes()
        } else {
            n.to_le_bytes()
        });
    }
    fn pcap(records: &[(u64, Vec<u8>)], big: bool, nano: bool) -> Vec<u8> {
        let mut b = Vec::new();
        put32(&mut b, if nano { 0xa1b23c4d } else { 0xa1b2c3d4 }, big);
        put16(&mut b, 2, big);
        put16(&mut b, 4, big);
        put32(&mut b, 0, big);
        put32(&mut b, 0, big);
        put32(&mut b, 96, big);
        put32(&mut b, 1, big);
        for (ns, packet) in records {
            put32(&mut b, (ns / 1_000_000_000) as u32, big);
            put32(
                &mut b,
                ((ns % 1_000_000_000) / if nano { 1 } else { 1000 }) as u32,
                big,
            );
            put32(&mut b, packet.len().min(96) as u32, big);
            put32(&mut b, packet.len() as u32, big);
            b.extend_from_slice(&packet[..packet.len().min(96)]);
        }
        b
    }
    #[test]
    fn snaplen_is_not_ip_length_and_connection_totals_include_delayed_packets() {
        let b = pcap(
            &[
                (0, connection_packet(true, 1000, 49999, 1, 0x02)),
                (1000, connection_packet(true, 900, 50000, 2, 0x02)),
                (2000, connection_packet(false, 800, 50000, 3, 0x10)),
                (3000, connection_packet(true, 40, 50001, 4, 0x02)),
                (3100, connection_packet(false, 700, 50001, 5, 0x10)),
            ],
            false,
            true,
        );
        let r = account(&b, &rows(), local(), Some(0)).unwrap();
        assert_eq!(r["setup"]["total_ip_bytes"], 1000);
        assert_eq!(r["rows"][0]["upload_ip_bytes"], 900);
        assert_eq!(r["rows"][0]["download_ip_bytes"], 800);
        assert_eq!(r["rows"][1]["total_ip_bytes"], 740);
        assert_eq!(r["campaign"]["total_ip_bytes"], 3440);
        assert_eq!(r["campaign"]["packets"], 5);
        assert_eq!(r["rows"][0]["unix_end_nanos"], "1900");
        assert_eq!(r["rows"][0]["packets_within_measurement"], 1);
        assert_eq!(r["unattributed"]["packets"], 0);
        assert_eq!(r["capture_complete"], true);
    }
    #[test]
    fn both_endiannesses_and_timestamp_resolutions_are_supported() {
        let rows=serde_json::json!({"task":{"id":"x"},"phase":"transfer","unix_start_nanos":"1000000000","unix_end_nanos":"1300000000"}).to_string();
        for big in [false, true] {
            for nano in [false, true] {
                let b = pcap(&[(1_234_567_000, packet(true, 700))], big, nano);
                let r = account(&b, &rows, local(), Some(0)).unwrap();
                assert_eq!(r["capture_last_packet_unix_nanos"], "1234567000");
                assert_eq!(r["campaign"]["total_ip_bytes"], 700);
            }
        }
    }
    #[test]
    fn malformed_or_truncated_packet_headers_are_rejected() {
        let good = pcap(&[(1000, packet(true, 900))], false, true);
        for end in [0, 3, 23, 25, 39, 40, 41, good.len() - 1] {
            assert!(account(&good[..end], &rows(), local(), Some(0)).is_err());
        }
        let mut bad = good.clone();
        bad[36..40].copy_from_slice(&10u32.to_le_bytes());
        assert!(account(&bad, &rows(), local(), Some(0)).is_err());
        let mut bad = good.clone();
        bad[28..32].copy_from_slice(&1_000_000_000u32.to_le_bytes());
        assert!(account(&bad, &rows(), local(), Some(0)).is_err());
        let mut bad = good;
        bad[40 + 16..40 + 18].copy_from_slice(&10000u16.to_be_bytes());
        assert!(account(&bad, &rows(), local(), Some(0)).is_err());
    }
    #[test]
    fn drops_or_missing_drop_summary_never_become_complete() {
        let b = pcap(
            &[(1000, packet(true, 900)), (3000, packet(false, 700))],
            false,
            true,
        );
        for drops in [None, Some(3)] {
            let r = account(&b, &rows(), local(), drops).unwrap();
            assert_eq!(r["capture_complete"], false);
        }
    }

    #[test]
    fn cooked_headers_vlan_and_ipv6_use_the_ip_length() {
        for link in [113u32, 276] {
            let ip = packet(true, 700);
            let mut cooked = vec![0; if link == 113 { 16 } else { 20 }];
            let position = if link == 113 { 14 } else { 0 };
            cooked[position..position + 2].copy_from_slice(&0x0800u16.to_be_bytes());
            cooked.extend_from_slice(&ip[14..]);
            let mut capture = pcap(&[(1000, cooked)], false, true);
            capture[20..24].copy_from_slice(&link.to_le_bytes());
            let report = account(&capture, &rows(), local(), Some(0)).unwrap();
            assert_eq!(report["campaign"]["upload_ip_bytes"], 700);
        }
        let ip = packet(true, 900);
        let mut vlan = ip[..12].to_vec();
        vlan.extend_from_slice(&[0x81, 0, 0, 7]);
        vlan.extend_from_slice(&ip[12..]);
        let report = account(
            &pcap(&[(1000, vlan)], false, true),
            &rows(),
            local(),
            Some(0),
        )
        .unwrap();
        assert_eq!(report["campaign"]["total_ip_bytes"], 900);

        let source: Ipv6Addr = "2001:db8::1".parse().unwrap();
        let destination: Ipv6Addr = "2001:db8::2".parse().unwrap();
        let mut ipv6 = vec![0u8; 14 + 40 + 1000];
        ipv6[12..14].copy_from_slice(&0x86ddu16.to_be_bytes());
        ipv6[14] = 0x60;
        ipv6[18..20].copy_from_slice(&1000u16.to_be_bytes());
        ipv6[20] = 6;
        ipv6[22..38].copy_from_slice(&source.octets());
        ipv6[38..54].copy_from_slice(&destination.octets());
        ipv6[54..56].copy_from_slice(&443u16.to_be_bytes());
        ipv6[56..58].copy_from_slice(&50000u16.to_be_bytes());
        ipv6[66] = 0x50;
        let report = account(
            &pcap(&[(1000, ipv6)], true, true),
            &rows(),
            destination.into(),
            Some(0),
        )
        .unwrap();
        assert_eq!(report["campaign"]["download_ip_bytes"], 1040);
    }

    #[test]
    fn unknown_packets_are_visible_and_unexpected_flows_are_rejected() {
        let mut arp = packet(true, 900);
        arp[12..14].copy_from_slice(&0x0806u16.to_be_bytes());
        let report = account(
            &pcap(&[(1000, arp)], false, true),
            &rows(),
            local(),
            Some(0),
        )
        .unwrap();
        assert_eq!(report["capture_complete"], false);
        assert_eq!(report["campaign"]["unclassified_packets"], 1);
        assert_eq!(report["campaign"]["unknown_ip_length_packets"], 1);
        let capture = pcap(&[(1000, packet(true, 900))], false, true);
        assert!(account(&capture, &rows(), "203.0.113.1".parse().unwrap(), Some(0)).is_err());
        assert!(account_expected(
            &capture,
            &rows(),
            local(),
            Some(0),
            Some("198.51.100.2".parse().unwrap())
        )
        .is_err());
        assert!(account_expected(
            &capture,
            &rows(),
            local(),
            Some(0),
            Some("198.51.100.1".parse().unwrap())
        )
        .is_ok());
        let mut wrong_port = packet(true, 900);
        wrong_port[36..38].copy_from_slice(&80u16.to_be_bytes());
        assert!(account(
            &pcap(&[(1000, wrong_port)], false, true),
            &rows(),
            local(),
            Some(0)
        )
        .is_err());
    }

    #[test]
    fn row_order_duplicates_and_timestamp_overflow_are_rejected() {
        let rows = rows();
        let lines: Vec<_> = rows.lines().collect();
        assert!(read_rows(&format!("{}\n{}", lines[1], lines[0])).is_err());
        assert!(read_rows(&format!("{}\n{}", lines[0], lines[0])).is_err());
        assert!(read_rows(&rows.replace("\"3000\"", "\"1800\"")).is_err());
        assert!(read_rows(&rows.replace("\"3000\"", "\"1900\"")).is_err());
        assert!(read_rows(&rows.replace("\"3000\"", "\"18446744073709551616\"")).is_err());
        let too_many = (0..4097).map(|i| json!({"task":{"id":i.to_string()},"phase":"transfer","unix_start_nanos":(i*1000).to_string(),"unix_end_nanos":(i*1000+500).to_string()}).to_string()).collect::<Vec<_>>().join("\n");
        assert!(read_rows(&too_many).is_err());
    }

    #[test]
    fn published_body_coverage_is_a_required_lower_bound() {
        let capture = pcap(
            &[
                (1000, connection_packet(true, 900, 50000, 2, 0x02)),
                (3000, connection_packet(true, 40, 50001, 4, 0x02)),
                (3100, connection_packet(false, 700, 50001, 5, 0x10)),
            ],
            false,
            true,
        );
        let rows = rows().replace("\"request_body_bytes\":100", "\"request_body_bytes\":901");
        let report = account(&capture, &rows, local(), Some(0)).unwrap();
        assert_eq!(report["unattributed"]["packets"], 0);
        assert_eq!(report["capture_complete"], false);
        assert_eq!(
            report["rows"][0]["application_body_coverage_verified"],
            false
        );
        assert!(report["incomplete_reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("capture_below_published_application_body_bytes")));
        let unavailable = json!({"task":{"id":"x"},"phase":"transfer","status":"published","unix_start_nanos":"1000","unix_end_nanos":"2000"}).to_string();
        assert_eq!(
            account(&capture, &unavailable, local(), Some(0)).unwrap()["capture_complete"],
            false
        );
    }

    #[test]
    fn tcpdump_summary_counter_is_preserved_without_guessing() {
        assert_eq!(
            dropped(
                "8 packets captured\n8 packets received by filter\n0 packets dropped by kernel\n"
            )
            .unwrap(),
            Some(0)
        );
        assert_eq!(dropped("5 packets dropped by kernel\n").unwrap(), Some(5));
        assert_eq!(
            dropped("capture interrupted without summary").unwrap(),
            None
        );
        assert!(dropped("invalid packets dropped by kernel").is_err());
        assert!(dropped("0 packets dropped by kernel\n1 packets dropped by kernel").is_err());
    }

    #[test]
    fn double_vlan_minimum_frame_padding_is_not_counted_as_ip_bytes() {
        let ip = packet(true, 40);
        let mut tagged = ip[..12].to_vec();
        tagged.extend_from_slice(&[0x88, 0xa8, 0, 1, 0x81, 0, 0, 2]);
        tagged.extend_from_slice(&ip[12..]);
        tagged.resize(68, 0);
        let report = account(
            &pcap(&[(1000, tagged)], false, true),
            &rows(),
            local(),
            Some(0),
        )
        .unwrap();
        assert_eq!(report["campaign"]["total_ip_bytes"], 40);
    }

    fn connection_packet(
        upload: bool,
        length: u16,
        local_port: u16,
        sequence: u32,
        flags: u8,
    ) -> Vec<u8> {
        let mut p = packet(upload, length);
        let port = if upload { 34 } else { 36 };
        p[port..port + 2].copy_from_slice(&local_port.to_be_bytes());
        p[38..42].copy_from_slice(&sequence.to_be_bytes());
        p[47] = flags;
        p
    }

    #[test]
    fn delayed_fin_is_charged_to_the_connection_owner_not_the_next_row() {
        let capture = pcap(
            &[
                (1000, connection_packet(true, 60, 50001, 100, 0x02)),
                (1500, connection_packet(false, 400, 50001, 900, 0x10)),
                (3000, connection_packet(true, 60, 50002, 200, 0x02)),
                (3500, connection_packet(false, 800, 50002, 901, 0x10)),
                (4000, connection_packet(false, 40, 50001, 1300, 0x11)),
            ],
            false,
            true,
        );
        let report = account(&capture, &rows(), local(), Some(0)).unwrap();
        assert_eq!(report["rows"][0]["download_ip_bytes"], 440);
        assert_eq!(report["rows"][1]["download_ip_bytes"], 800);
        assert_eq!(report["campaign"]["total_ip_bytes"], 1360);
        assert_eq!(report["unattributed"]["packets"], 0);
    }

    #[test]
    fn retransmitted_syn_keeps_its_first_owner() {
        let capture = pcap(
            &[
                (1000, connection_packet(true, 60, 50001, 100, 0x02)),
                (3000, connection_packet(true, 60, 50001, 100, 0x02)),
            ],
            false,
            true,
        );
        let report = account(&capture, &rows(), local(), Some(0)).unwrap();
        assert_eq!(report["rows"][0]["upload_ip_bytes"], 120);
        assert_eq!(report["rows"][1]["upload_ip_bytes"], 0);
    }

    #[test]
    fn missing_syn_and_reused_tuple_are_unattributed_and_incomplete() {
        for packets in [
            vec![(1000, connection_packet(true, 1000, 50001, 100, 0x10))],
            vec![
                (1000, connection_packet(true, 60, 50001, 100, 0x02)),
                (3000, connection_packet(true, 60, 50001, 200, 0x02)),
            ],
        ] {
            let report = account(&pcap(&packets, false, true), &rows(), local(), Some(0)).unwrap();
            assert_eq!(report["unattributed"]["packets"], packets.len());
            assert_eq!(report["capture_complete"], false);
            assert_eq!(report["rows"][0]["total_ip_bytes"], 0);
            assert_eq!(report["rows"][1]["total_ip_bytes"], 0);
            assert_eq!(report["partition_sum_verified"], true);
        }
    }

    #[test]
    fn setup_and_multiple_connections_keep_ownership_while_gap_syn_is_unattributed() {
        let capture = pcap(
            &[
                (0, connection_packet(true, 60, 49999, 1, 0x02)),
                (1000, connection_packet(true, 60, 50001, 2, 0x02)),
                (1200, connection_packet(true, 60, 50002, 3, 0x02)),
                (2000, connection_packet(true, 60, 50004, 5, 0x02)),
                (3000, connection_packet(true, 60, 50003, 4, 0x02)),
                (3500, connection_packet(false, 100, 50003, 8, 0x10)),
                (4000, connection_packet(false, 40, 50002, 9, 0x11)),
                (5000, connection_packet(false, 40, 49999, 10, 0x11)),
            ],
            false,
            true,
        );
        let report = account(&capture, &rows(), local(), Some(0)).unwrap();
        assert_eq!(report["setup"]["total_ip_bytes"], 100);
        assert_eq!(report["rows"][0]["total_ip_bytes"], 160);
        assert_eq!(report["rows"][1]["total_ip_bytes"], 160);
        assert_eq!(report["unattributed"]["total_ip_bytes"], 60);
        assert_eq!(report["campaign"]["total_ip_bytes"], 480);
        assert_eq!(
            report["unattributed_reasons"]["syn_outside_active_measurement"],
            1
        );
        assert_eq!(report["capture_complete"], false);
    }

    #[test]
    fn packet_record_order_does_not_change_owner_and_pre_syn_packets_are_unattributed() {
        let capture = pcap(
            &[
                (3000, connection_packet(true, 60, 50001, 100, 0x02)),
                (1000, connection_packet(true, 60, 50001, 100, 0x02)),
                (900, connection_packet(false, 40, 50001, 900, 0x10)),
            ],
            false,
            true,
        );
        let report = account(&capture, &rows(), local(), Some(0)).unwrap();
        assert_eq!(report["rows"][0]["upload_ip_bytes"], 120);
        assert_eq!(report["rows"][1]["packets"], 0);
        assert_eq!(report["unattributed"]["download_ip_bytes"], 40);
        assert_eq!(
            report["unattributed_reasons"]["packet_precedes_observed_syn"],
            1
        );
        assert_eq!(report["campaign"]["total_ip_bytes"], 160);
        assert_eq!(report["capture_first_packet_unix_nanos"], "900");
    }
}
