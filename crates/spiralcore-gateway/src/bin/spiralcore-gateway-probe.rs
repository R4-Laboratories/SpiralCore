//! Test-only application peer and capture summarizer. No secret or production input.
use sha2::{Digest, Sha256};
use spiralcore_gateway::measurement::{self as m, invalid, Result};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    path::Path,
    time::{Duration, Instant},
};
fn fixture_file(path: &str) -> Result<Vec<u8>> {
    let len = std::fs::metadata(path)?.len();
    if !(1..=64 * 1024 * 1024).contains(&len) {
        return Err(invalid("fixture size").into());
    }
    Ok(std::fs::read(path)?)
}
fn report(
    path: &str,
    protocol: &str,
    role: &str,
    data: &[u8],
    wall: Instant,
    cpu: u64,
    extra: serde_json::Value,
) -> Result<()> {
    let wall_us = wall.elapsed().as_micros() as u64;
    let process_cpu_us = m::cpu_us()? - cpu;
    write_report(path, protocol, role, data, (wall_us, process_cpu_us), extra)
}
fn write_report(
    path: &str,
    protocol: &str,
    role: &str,
    data: &[u8],
    timing: (u64, u64),
    extra: serde_json::Value,
) -> Result<()> {
    let (wall_us, process_cpu_us) = timing;
    m::write_json(
        Path::new(path),
        &serde_json::json!({"version":1,"protocol":protocol,"role":role,"exact":true,"application_bytes":data.len(),"sha256":m::digest(data),"wall_us":wall_us,"process_cpu_us":process_cpu_us,"timing":"fixture loaded before timer; transfer and exact verification included; TCP connect inclusion follows detail.connect_included, server bind/accept excluded; UDP server includes wait for first packet; final report hashing and serialization excluded","detail":extra}),
    )
}
fn tcp(role: &str, addr: &str, path: &str, out: &str) -> Result<()> {
    let data = fixture_file(path)?;
    let server = role == "tcp-server";
    let (mut stream, wall, cpu) = if server {
        let stream = TcpListener::bind(addr)?.accept()?.0;
        (stream, Instant::now(), m::cpu_us()?)
    } else {
        let wall = Instant::now();
        let cpu = m::cpu_us()?;
        let stream = TcpStream::connect_timeout(&addr.parse()?, Duration::from_secs(15))?;
        (stream, wall, cpu)
    };
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    stream.set_nodelay(true)?;
    tcp_exchange(&mut stream, &data, server)?;
    report(
        out,
        "tcp",
        role,
        &data,
        wall,
        cpu,
        serde_json::json!({"receipt_bytes":32,"connect_included":!server}),
    )
}
fn tcp_exchange(stream: &mut TcpStream, data: &[u8], server: bool) -> Result<()> {
    if server {
        let mut length = [0; 8];
        stream.read_exact(&mut length)?;
        if u64::from_be_bytes(length) != data.len() as u64 {
            return Err(invalid("length mismatch").into());
        }
        let mut offset = 0;
        let mut buf = [0u8; 65536];
        let mut hash = Sha256::new();
        while offset < data.len() {
            let size = buf.len().min(data.len() - offset);
            stream.read_exact(&mut buf[..size])?;
            if buf[..size] != data[offset..offset + size] {
                return Err(invalid("TCP exact byte mismatch").into());
            }
            hash.update(&buf[..size]);
            offset += size;
        }
        stream.write_all(&hash.finalize())?;
    } else {
        stream.write_all(&(data.len() as u64).to_be_bytes())?;
        stream.write_all(data)?;
        let mut receipt = [0; 32];
        stream.read_exact(&mut receipt)?;
        if receipt[..] != Sha256::digest(data)[..] {
            return Err(invalid("TCP receipt mismatch").into());
        }
    }
    Ok(())
}
fn tcp_sequence(
    role: &str,
    addr: &str,
    pattern_path: &str,
    opaque_path: &str,
    prefix: &str,
) -> Result<()> {
    let pattern = fixture_file(pattern_path)?;
    let opaque = fixture_file(opaque_path)?;
    let mut reports = Vec::with_capacity(3);
    let server = role == "tcp-sequence-server";
    let (mut stream, first_wall, first_cpu) = if server {
        let stream = TcpListener::bind(addr)?.accept()?.0;
        (stream, Instant::now(), m::cpu_us()?)
    } else {
        let wall = Instant::now();
        let cpu = m::cpu_us()?;
        (
            TcpStream::connect_timeout(&addr.parse()?, Duration::from_secs(15))?,
            wall,
            cpu,
        )
    };
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    stream.set_nodelay(true)?;
    let local = stream.local_addr()?.to_string();
    let peer = stream.peer_addr()?.to_string();
    for (index, (name, data)) in [
        ("pattern-before", &pattern),
        ("opaque", &opaque),
        ("pattern-after", &pattern),
    ]
    .into_iter()
    .enumerate()
    {
        let (wall, cpu) = if index == 0 {
            (first_wall, first_cpu)
        } else {
            (Instant::now(), m::cpu_us()?)
        };
        tcp_exchange(&mut stream, data, server)?;
        // Freeze the completed interval now. Hashing, JSON construction and
        // output I/O must not spill into the other peer's next transfer.
        let timing = (wall.elapsed().as_micros() as u64, m::cpu_us()? - cpu);
        reports.push((index, name, data, timing));
    }
    for (index, name, data, timing) in reports {
        write_report(
            &format!("{prefix}-{name}.json"),
            "tcp",
            role,
            data,
            timing,
            serde_json::json!({"receipt_bytes":32,"connect_included":!server && index==0,"sequence_index":index,"same_connection":true,"local_address":local,"peer_address":peer,"reports_emitted_after_sequence":true}),
        )?;
    }
    Ok(())
}
fn udp(role: &str, addr: &str, path: &str, out: &str) -> Result<()> {
    let data = fixture_file(path)?;
    let server = role == "udp-server";
    let remote: SocketAddr = addr.parse()?;
    let socket = if server {
        UdpSocket::bind(remote)?
    } else {
        UdpSocket::bind(if remote.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        })?
    };
    socket.set_read_timeout(Some(Duration::from_secs(15)))?;
    let chunks = data.len().div_ceil(1000);
    let mut buf = [0u8; 1100];
    let wall = Instant::now();
    let cpu = m::cpu_us()?;
    let mut retries = 0u64;
    if server {
        let mut expected = 0usize;
        let mut peer = None;
        loop {
            let (n, from) = socket.recv_from(&mut buf)?;
            if n < 8 {
                return Err(invalid("UDP short frame").into());
            }
            if peer.is_some_and(|p| p != from) {
                return Err(invalid("UDP changed peer").into());
            }
            peer = Some(from);
            let sequence = u64::from_be_bytes(buf[..8].try_into()?) as usize;
            if sequence == chunks && n == 8 && expected == chunks {
                let mut receipt = Vec::from(&buf[..8]);
                receipt.extend(Sha256::digest(&data));
                socket.send_to(&receipt, from)?;
                report(
                    out,
                    "udp",
                    role,
                    &data,
                    wall,
                    cpu,
                    serde_json::json!({"chunks":chunks,"application_reliability":"stop-and-wait exact echo"}),
                )?;
                socket.set_read_timeout(Some(Duration::from_millis(500)))?;
                while let Ok((n, from_again)) = socket.recv_from(&mut buf) {
                    if from_again == from && n == 8 && buf[..8] == receipt[..8] {
                        socket.send_to(&receipt, from)?;
                    } else {
                        break;
                    }
                }
                return Ok(());
            }
            if sequence >= chunks || sequence > expected {
                return Err(invalid("UDP sequence").into());
            }
            let start = sequence * 1000;
            let end = (start + 1000).min(data.len());
            if buf[8..n] != data[start..end] {
                return Err(invalid("UDP exact byte mismatch").into());
            }
            socket.send_to(&buf[..n], from)?;
            if sequence == expected {
                expected += 1;
            }
        }
    } else {
        socket.connect(remote)?;
        socket.set_read_timeout(Some(Duration::from_millis(300)))?;
        for sequence in 0..=chunks {
            let mut frame = Vec::from((sequence as u64).to_be_bytes());
            if sequence < chunks {
                frame.extend_from_slice(
                    &data[sequence * 1000..((sequence + 1) * 1000).min(data.len())],
                );
            }
            let mut success = false;
            for attempt in 0..10 {
                socket.send(&frame)?;
                let deadline = Instant::now() + Duration::from_millis(300);
                loop {
                    match socket.recv(&mut buf) {
                        Ok(n) => {
                            if n < 8 || buf[..8] != frame[..8] {
                                if Instant::now() >= deadline {
                                    break;
                                }
                                continue;
                            }
                            let exact = if sequence == chunks {
                                n == 40 && buf[8..n] == Sha256::digest(&data)[..]
                            } else {
                                buf[..n] == frame
                            };
                            if !exact {
                                return Err(invalid("UDP echo mismatch").into());
                            }
                            success = true;
                            break;
                        }
                        Err(e)
                            if [std::io::ErrorKind::WouldBlock, std::io::ErrorKind::TimedOut]
                                .contains(&e.kind()) =>
                        {
                            break
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                if success {
                    break;
                }
                if attempt < 9 {
                    retries += 1;
                }
            }
            if !success {
                return Err(invalid("UDP bounded retries exhausted").into());
            }
        }
        report(
            out,
            "udp",
            role,
            &data,
            wall,
            cpu,
            serde_json::json!({"chunks":chunks,"retries":retries,"application_reliability":"stop-and-wait exact echo; all echo traffic counted"}),
        )
    }
}
fn main() -> Result<()> {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str){
 Some("fixture") if a.len()==6=>{let bytes=m::fixture(a[3].parse()?,&a[4],a[5].parse()?)?;std::fs::write(&a[2],&bytes)?;println!("{}",serde_json::json!({"bytes":bytes.len(),"sha256":m::digest(&bytes)}));Ok(())},
 Some("pcap-summary") if a.len()==4=>{let len=std::fs::metadata(&a[2])?.len();if len>128*1024*1024{return Err(invalid("pcap size cap").into());}m::write_json(Path::new(&a[3]),&m::summarize_pcap(&std::fs::read(&a[2])?)?)},
 Some("tcp-server"|"tcp-client") if a.len()==5=>tcp(&a[1],&a[2],&a[3],&a[4]),
 Some("tcp-sequence-server"|"tcp-sequence-client") if a.len()==6=>tcp_sequence(&a[1],&a[2],&a[3],&a[4],&a[5]),
 Some("udp-server"|"udp-client") if a.len()==5=>udp(&a[1],&a[2],&a[3],&a[4]),
 _=>Err(invalid("usage: fixture PATH SIZE pattern|opaque SEED | tcp-server/tcp-client/udp-server/udp-client ADDR FIXTURE OUT.json | tcp-sequence-server/tcp-sequence-client ADDR PATTERN OPAQUE OUT_PREFIX | pcap-summary IN.pcap OUT.json").into())}
}
