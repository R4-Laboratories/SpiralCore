use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn read(p: &Path, limit: usize) -> Result<Vec<u8>> {
    let m = fs::symlink_metadata(p)?;
    assert!(m.file_type().is_file() && m.len() <= limit as u64);
    let mut b = Vec::new();
    File::open(p)?.take(limit as u64 + 1).read_to_end(&mut b)?;
    assert_eq!(b.len() as u64, m.len());
    Ok(b)
}
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn n(v: &Value) -> u64 {
    v.as_u64().unwrap()
}
fn le32(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}
fn be16(b: &[u8], i: usize) -> u16 {
    u16::from_be_bytes(b[i..i + 2].try_into().unwrap())
}
fn be32(b: &[u8], i: usize) -> u32 {
    u32::from_be_bytes(b[i..i + 4].try_into().unwrap())
}
#[derive(Clone, Debug)]
struct Packet {
    index: usize,
    ns: u64,
    up: bool,
    local_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    ip_bytes: u64,
    payload: u64,
}
impl Packet {
    fn value(&self) -> Value {
        json!({"record_index":self.index,"unix_nanos":self.ns.to_string(),"direction":if self.up{"upload"}else{"download"},"local_port":self.local_port,"sequence":self.seq,"acknowledgement":self.ack,"flags":self.flags,"ip_bytes":self.ip_bytes,"tcp_payload_bytes":self.payload})
    }
}
#[derive(Default)]
struct Count {
    packets: u64,
    up: u64,
    down: u64,
    up_packets: u64,
    down_packets: u64,
}
impl Count {
    fn add(&mut self, p: &Packet) {
        self.packets += 1;
        if p.up {
            self.up += p.ip_bytes;
            self.up_packets += 1;
        } else {
            self.down += p.ip_bytes;
            self.down_packets += 1;
        }
    }
    fn value(&self) -> Value {
        json!({"packets":self.packets,"upload_ip_bytes":self.up,"download_ip_bytes":self.down,"total_ip_bytes":self.up+self.down,"upload_packets":self.up_packets,"download_packets":self.down_packets})
    }
    fn check(&self, v: &Value) {
        for (k, x) in self.value().as_object().unwrap() {
            assert_eq!(v[k], *x, "{k}");
        }
    }
}
fn scan(bytes: &[u8], mut f: impl FnMut(Packet)) {
    assert!(bytes.len() >= 24);
    assert_eq!(&bytes[..4], &[0x4d, 0x3c, 0xb2, 0xa1]);
    assert_eq!(le32(bytes, 16), 96);
    assert_eq!(le32(bytes, 20), 1);
    let mut at = 24;
    let mut index = 0;
    let mut previous = 0;
    while at < bytes.len() {
        assert!(at + 16 <= bytes.len());
        let ns = u64::from(le32(bytes, at)) * 1_000_000_000 + u64::from(le32(bytes, at + 4));
        assert!(le32(bytes, at + 4) < 1_000_000_000);
        assert!(ns >= previous);
        previous = ns;
        let cap = le32(bytes, at + 8) as usize;
        let orig = le32(bytes, at + 12) as usize;
        at += 16;
        assert!(cap <= 96 && cap <= orig && at + cap <= bytes.len());
        let p = &bytes[at..at + cap];
        at += cap;
        assert!(p.len() >= 54);
        assert_eq!(be16(p, 12), 0x800);
        assert_eq!(p[14] >> 4, 4);
        let ihl = usize::from(p[14] & 15) * 4;
        assert!(ihl >= 20 && 14 + ihl + 20 <= cap);
        let ip_bytes = u64::from(be16(p, 16));
        assert!(ip_bytes >= ihl as u64 + 20);
        assert!(orig == 14 + ip_bytes as usize || orig == (14 + ip_bytes as usize).max(60));
        assert_eq!(p[23], 6);
        assert_eq!(be16(p, 20) & 0x3fff, 0);
        let local = [10, 1, 0, 84];
        let remote = [104, 16, 230, 132];
        let up = p[26..30] == local && p[30..34] == remote;
        if !up {
            assert_eq!(&p[26..30], &remote);
            assert_eq!(&p[30..34], &local);
        }
        let t = 14 + ihl;
        let local_port = be16(p, t + if up { 0 } else { 2 });
        assert_eq!(be16(p, t + if up { 2 } else { 0 }), 443);
        let th = usize::from(p[t + 12] >> 4) * 4;
        assert!(th >= 20 && ip_bytes >= ihl as u64 + th as u64);
        index += 1;
        f(Packet {
            index,
            ns,
            up,
            local_port,
            seq: be32(p, t + 4),
            ack: be32(p, t + 8),
            flags: p[t + 13],
            ip_bytes,
            payload: ip_bytes - ihl as u64 - th as u64,
        });
    }
}
fn owner(rows: &[Value], ns: u64) -> Value {
    let r = rows
        .iter()
        .filter(|r| {
            let start = r["unix_start_nanos"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let end = r["unix_end_nanos"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            start <= ns && ns <= end
        })
        .collect::<Vec<_>>();
    assert!(r.len() <= 1);
    r.first().map_or(Value::Null,|r|json!({"task":r["task"],"phase":r["phase"],"unix_start_nanos":r["unix_start_nanos"],"unix_end_nanos":r["unix_end_nanos"],"metrics":r["metrics"],"status":r["status"]}))
}
fn main() -> Result<()> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(a.len(), 2);
    let root = Path::new(&a[0]);
    let pcap = read(&root.join("headers.pcap"), 256 << 20)?;
    let rows_bytes = read(&root.join("rows.jsonl"), 16 << 20)?;
    let rows = std::str::from_utf8(&rows_bytes)?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    assert_eq!(rows.len(), 48);
    let report_bytes = read(&root.join("capture.json"), 16 << 20)?;
    let report: Value = serde_json::from_slice(&report_bytes)?;
    let life_bytes = read(&root.join("capture-lifecycle.json"), 16 << 20)?;
    let life: Value = serde_json::from_slice(&life_bytes)?;
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["capture_lifecycle_verified"], true);
    assert_eq!(report["capture_lifecycle_sha256"], sha(&life_bytes));
    assert_eq!(life["pcap_sha256"], sha(&pcap));
    assert!(life["errors"].as_array().unwrap().is_empty());
    assert_eq!(report["kernel_reported_drops"], 0);
    assert_eq!(
        report["tcpdump_captured_packets"],
        report["tcpdump_received_by_filter_packets"]
    );
    let mut syns = BTreeMap::<u16, Vec<Packet>>::new();
    let mut ports = BTreeSet::new();
    let mut total = Count::default();
    scan(&pcap, |p| {
        ports.insert(p.local_port);
        total.add(&p);
        if p.up && p.flags & 0x12 == 2 {
            syns.entry(p.local_port).or_default().push(p);
        }
    });
    total.check(&report["campaign"]);
    assert_eq!(total.packets, n(&report["tcpdump_captured_packets"]));
    assert_eq!(ports, syns.keys().copied().collect());
    let reused = syns
        .iter()
        .filter(|(_, v)| v.iter().map(|p| p.seq).collect::<BTreeSet<_>>().len() > 1)
        .collect::<Vec<_>>();
    assert_eq!(reused.len(), 1);
    assert_eq!(syns.len(), 47);
    assert_eq!(syns.values().map(Vec::len).sum::<usize>(), 48);
    let (port, starts) = reused[0];
    assert_eq!(starts.len(), 2);
    let mut affected = Vec::new();
    scan(&pcap, |p| {
        if p.local_port == *port {
            assert!(affected.len() < 65536);
            affected.push(p);
        }
    });
    let mut excluded = Count::default();
    for p in &affected {
        excluded.add(p);
    }
    excluded.check(&report["unattributed"]);
    assert_eq!(
        report["unattributed_reasons"],
        json!({"reused_tuple_with_different_initial_sequence":excluded.packets})
    );
    let mut segments = Vec::new();
    for (i, syn) in starts.iter().enumerate() {
        let end = starts.get(i + 1).map_or(u64::MAX, |p| p.ns);
        let packets = affected
            .iter()
            .filter(|p| p.ns >= syn.ns && p.ns < end)
            .collect::<Vec<_>>();
        let mut count = Count::default();
        for p in &packets {
            count.add(p);
        }
        let last = packets.last().unwrap();
        let selected = owner(&rows, syn.ns);
        assert_eq!(selected["status"], "published");
        let id = selected["task"]["id"].as_str().unwrap();
        let phase = selected["phase"].as_str().unwrap();
        let captured = report["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["task"]["id"] == id && v["phase"] == phase)
            .collect::<Vec<_>>();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0]["packets"], 0);
        assert_eq!(captured[0]["application_body_coverage_verified"], false);
        let controls = packets
            .iter()
            .filter(|p| p.flags & 7 != 0)
            .map(|p| p.value())
            .collect::<Vec<_>>();
        segments.push(json!({"syn":syn.value(),"containing_row":selected,"diagnostic_interval_end_exclusive":end.to_string(),"packet_totals_in_syn_time_segment":count.value(),"first_eight_packets":packets.iter().take(8).map(|p|p.value()).collect::<Vec<_>>(),"last_eight_packets":packets.iter().rev().take(8).rev().map(|p|p.value()).collect::<Vec<_>>(),"syn_fin_rst_events":controls,"last_packet_unix_nanos":last.ns.to_string(),"gap_until_next_syn_nanos":starts.get(i+1).map(|p|p.ns-last.ns)}));
    }
    let result = json!({"schema":"spiralcore.capture-tuple-reuse-diagnosis.v1","status":"causal_parser_condition_independently_confirmed","source":"9dce4c57d1f41cef7365b297cd690849afce34e7","case":"middle_4096","shard":1,"flow":{"local_ip":"10.1.0.84","local_port":port,"remote_ip":"104.16.230.132","remote_port":443,"protocol":"TCP"},"observed_outbound_syns":48,"unique_four_tuples":47,"tuples_without_any_observed_syn":0,"different_isns_on_implicated_tuple":starts.iter().map(|p|p.seq).collect::<Vec<_>>(),"campaign":total.value(),"production_unattributed":excluded.value(),"production_incomplete_reasons":report["incomplete_reasons"],"segments":segments,"pcap_sha256":sha(&pcap),"rows_sha256":sha(&rows_bytes),"capture_report_sha256":sha(&report_bytes),"lifecycle_sha256":sha(&life_bytes),"reviewer_source_sha256":sha(include_bytes!("review.rs")),"reviewer_executable_sha256":sha(&read(&std::env::current_exe()?,16<<20)?),"conclusion":"Both outbound SYNs are captured with different initial sequence numbers on the same four-tuple. The frozen parser quarantines every packet on that tuple by design. This explains both zero-owned-packet rows and their body undercoverage; it is not evidence that these two transfers had no captured packets.","limitations":["Diagnostic SYN-time segments are not approved TCP epoch attribution and do not replace production per-row IP totals","Equal tcpdump counters and zero kernel drops do not prove uncaptured traffic cannot exist","This read-only diagnosis does not relax the parser, rerun or replace any performance trial, or change the incomplete result","Any epoch-aware successor requires a separately specified state machine, sequence/lifecycle ambiguity handling, tests and qualification"]});
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&a[1])?;
    serde_json::to_writer_pretty(&mut f, &result)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    println!(
        "CONFIRMED: {}:{} -> {}:443, two observed SYN ISNs; {} quarantined packets",
        "10.1.0.84", port, "104.16.230.132", excluded.packets
    );
    Ok(())
}
