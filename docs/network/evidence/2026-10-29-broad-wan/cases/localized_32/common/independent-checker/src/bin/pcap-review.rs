// Read-only post-run campaign packet totals. Does not reproduce TCP row ownership.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufReader, Read},
    net::IpAddr,
    path::Path,
};
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn num(v: &Value, k: &str) -> u64 {
    v[k].as_u64().expect(k)
}
fn read_capture_report(path: &Path) -> (Value, String) {
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take((8 << 20) + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 8 << 20);
    (
        serde_json::from_slice(&bytes).unwrap(),
        hex(&Sha256::digest(&bytes)),
    )
}
fn main() {
    let root = std::env::args().nth(1).expect("campaign directory");
    let mut reports = Vec::new();
    for shard in 0..5 {
        let dir = Path::new(&root).join(format!("shard-{shard}/remote"));
        let (report, report_sha256) = read_capture_report(&dir.join("capture.json"));
        assert_eq!(report["schema"], "spiralcore.pcap-accounting.v1");
        assert_eq!(report["scope"], "runner_to_relay_leg_only");
        let local: IpAddr = report["local_ip"].as_str().unwrap().parse().unwrap();
        let remote: IpAddr = report["expected_remote_ip"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let file = File::open(dir.join("headers.pcap")).unwrap();
        let size = file.metadata().unwrap().len();
        assert!(size <= 256 << 20);
        let mut r = BufReader::new(file);
        let mut h = [0u8; 24];
        r.read_exact(&mut h).unwrap();
        let mut digest = Sha256::new();
        digest.update(h);
        let (big, nano) = match &h[..4] {
            [0xd4, 0xc3, 0xb2, 0xa1] => (false, false),
            [0xa1, 0xb2, 0xc3, 0xd4] => (true, false),
            [0x4d, 0x3c, 0xb2, 0xa1] => (false, true),
            [0xa1, 0xb2, 0x3c, 0x4d] => (true, true),
            _ => panic!("unsupported PCAP magic"),
        };
        let u32n = |b: &[u8]| {
            let a = b.try_into().unwrap();
            if big {
                u32::from_be_bytes(a)
            } else {
                u32::from_le_bytes(a)
            }
        };
        let u16n = |b: &[u8]| {
            let a = b.try_into().unwrap();
            if big {
                u16::from_be_bytes(a)
            } else {
                u16::from_le_bytes(a)
            }
        };
        assert_eq!(u16n(&h[4..6]), 2);
        assert_eq!(u16n(&h[6..8]), 4);
        let snap = u32n(&h[16..20]);
        assert_eq!(snap, 96);
        assert_eq!(
            u32n(&h[20..24]),
            1,
            "this independent parser qualifies Ethernet captures only"
        );
        let be16 = |b: &[u8]| u16::from_be_bytes(b.try_into().unwrap());
        let mut consumed = 24u64;
        let mut count = 0u64;
        let mut upload = 0u64;
        let mut download = 0u64;
        let mut up_packets = 0u64;
        let mut down_packets = 0u64;
        let mut first = None;
        let mut last = None;
        let mut syn_out = 0u64;
        loop {
            let mut rec = [0u8; 16];
            let n = r.read(&mut rec[..1]).unwrap();
            if n == 0 {
                break;
            }
            r.read_exact(&mut rec[1..]).unwrap();
            digest.update(rec);
            let sec = u64::from(u32n(&rec[..4]));
            let frac = u64::from(u32n(&rec[4..8]));
            assert!(frac < if nano { 1_000_000_000 } else { 1_000_000 });
            let ts =
                u128::from(sec) * 1_000_000_000 + u128::from(frac) * if nano { 1 } else { 1000 };
            first = Some(first.map_or(ts, |old: u128| old.min(ts)));
            last = Some(last.map_or(ts, |old: u128| old.max(ts)));
            let included = u32n(&rec[8..12]);
            let original = u32n(&rec[12..16]);
            assert!(included <= snap && included <= original);
            consumed = consumed.checked_add(16 + u64::from(included)).unwrap();
            assert!(consumed <= size);
            let mut b = vec![0u8; included as usize];
            r.read_exact(&mut b).unwrap();
            digest.update(&b);
            count += 1;
            assert!(b.len() >= 14);
            let mut ethertype = be16(&b[12..14]);
            let mut offset = 14usize;
            let mut vlans = 0;
            while ethertype == 0x8100 || ethertype == 0x88a8 {
                assert!(b.len() >= offset + 4 && vlans < 2);
                ethertype = be16(&b[offset + 2..offset + 4]);
                offset += 4;
                vlans += 1;
            }
            let (src, dst, ip_len, tcp) = match ethertype {
                0x0800 => {
                    assert!(b.len() >= offset + 20);
                    assert_eq!(b[offset] >> 4, 4);
                    let ihl = usize::from(b[offset] & 15) * 4;
                    assert!(ihl >= 20 && b.len() >= offset + ihl);
                    let len = usize::from(be16(&b[offset + 2..offset + 4]));
                    assert!(len >= ihl + 20);
                    assert_eq!(b[offset + 9], 6);
                    assert_eq!(
                        be16(&b[offset + 6..offset + 8]) & 0x3fff,
                        0,
                        "fragmented filtered flow requires separate analysis"
                    );
                    let src =
                        IpAddr::from(<[u8; 4]>::try_from(&b[offset + 12..offset + 16]).unwrap());
                    let dst =
                        IpAddr::from(<[u8; 4]>::try_from(&b[offset + 16..offset + 20]).unwrap());
                    (src, dst, len, offset + ihl)
                }
                0x86dd => {
                    assert!(b.len() >= offset + 40);
                    assert_eq!(b[offset] >> 4, 6);
                    assert_eq!(
                        b[offset + 6],
                        6,
                        "IPv6 extensions require separate analysis"
                    );
                    let len = usize::from(be16(&b[offset + 4..offset + 6])) + 40;
                    assert!(len >= 60);
                    let src =
                        IpAddr::from(<[u8; 16]>::try_from(&b[offset + 8..offset + 24]).unwrap());
                    let dst =
                        IpAddr::from(<[u8; 16]>::try_from(&b[offset + 24..offset + 40]).unwrap());
                    (src, dst, len, offset + 40)
                }
                _ => panic!("unexpected non-IP packet in filtered capture"),
            };
            // Linux Ethernet PCAP may omit minimum-frame padding. Neither caplen nor
            // original Ethernet length is substituted for the IP header's total length.
            let unpadded = offset + ip_len;
            assert!(
                original as usize == unpadded || original as usize == unpadded.max(60 + vlans * 4)
            );
            assert!(b.len() >= tcp + 20);
            let header = usize::from(b[tcp + 12] >> 4) * 4;
            assert!(header >= 20 && ip_len >= tcp - offset + header);
            let sport = be16(&b[tcp..tcp + 2]);
            let dport = be16(&b[tcp + 2..tcp + 4]);
            if src == local && dst == remote {
                assert_eq!(dport, 443);
                upload += ip_len as u64;
                up_packets += 1;
                if b[tcp + 13] & 0x12 == 0x02 {
                    syn_out += 1;
                }
            } else if src == remote && dst == local {
                assert_eq!(sport, 443);
                download += ip_len as u64;
                down_packets += 1;
            } else {
                panic!("packet outside pinned local/remote flow");
            }
        }
        assert_eq!(consumed, size);
        let campaign = &report["campaign"];
        for (key, want) in [
            ("packets", count),
            ("upload_packets", up_packets),
            ("download_packets", down_packets),
            ("upload_ip_bytes", upload),
            ("download_ip_bytes", download),
            ("total_ip_bytes", upload + download),
            ("unclassified_packets", 0),
            ("unclassified_ip_bytes", 0),
            ("unknown_ip_length_packets", 0),
        ] {
            assert_eq!(num(campaign, key), want, "{key}");
        }
        assert_eq!(
            report["capture_first_packet_unix_nanos"],
            first.unwrap().to_string()
        );
        assert_eq!(
            report["capture_last_packet_unix_nanos"],
            last.unwrap().to_string()
        );
        reports.push(json!({"shard":shard,"capture_report_sha256":report_sha256,"pcap_sha256":hex(&digest.finalize()),"pcap_file_bytes":size,"records":count,"upload_ip_bytes":upload,"download_ip_bytes":download,"total_ip_bytes":upload+download,"upload_packets":up_packets,"download_packets":down_packets,"outbound_syn_packets_including_retransmissions":syn_out,"capture_complete_as_reported":report["capture_complete"],"incomplete_reasons":report["incomplete_reasons"]}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema":"spiralcore.independent-broad-wan-pcap-review.v1","status":"campaign_totals_match_raw_header_replay","shards":reports,"limitations":["Only selected runner-to-relay TCP443 leg; not full path or physical Ethernet wire bytes","Independent replay verifies all packet lengths/directions and campaign totals, not individual TCP connection ownership","Per-row capture qualification additionally depends on production parser's strict observed-SYN ownership and accounting, checked separately","Zero kernel drops and matching counters do not prove interface/offload/filter provenance or uncaptured traffic"]})).unwrap());
}
