//! One application-level CONNECT refusal followed by the registered peer's binary echo.
use anyhow::{Context, Result, ensure};
use cpu_time::ProcessTime;
use serde_json::json;
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpStream},
    time::{Duration, Instant},
};

const GOOD: &[u8] = b"HTTP/1.1 200 Connection established\r\n\r\n";
const DENIED: &[u8] = b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
const TIME_LIMIT: Duration = Duration::from_secs(15);

#[derive(Default, Debug)]
struct Count {
    head_tx: usize,
    head_rx: usize,
    body_tx: usize,
    body_rx: usize,
}

fn proxy(url: &str) -> Result<SocketAddr> {
    let port = url
        .strip_prefix("http://127.0.0.1:")
        .context("expected workload HTTP loopback proxy")?;
    Ok(SocketAddr::from((Ipv4Addr::LOCALHOST, port_number(port)?)))
}

fn port_number(text: &str) -> Result<u16> {
    ensure!(
        !text.starts_with('0') && text.len() <= 5 && text.bytes().all(|b| b.is_ascii_digit()),
        "invalid port"
    );
    let port: u16 = text.parse().context("invalid port")?;
    ensure!(port > 0, "zero port");
    Ok(port)
}

fn target(authority: &str) -> Result<()> {
    let (host, port) = authority
        .rsplit_once(':')
        .context("expected DNS name and port")?;
    port_number(port)?;
    ensure!(
        host.len() <= 253
            && host.contains('.')
            && host
                .rsplit('.')
                .next()
                .is_some_and(|label| label.bytes().any(|c| c.is_ascii_lowercase())),
        "expected a dotted DNS name"
    );
    for label in host.split('.') {
        ensure!(
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'),
            "invalid DNS label"
        );
    }
    Ok(())
}

fn remaining(until: Instant) -> io::Result<Duration> {
    until
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| io::ErrorKind::TimedOut.into())
}

// The workload owns this one nonblocking stream. A readiness race cannot extend
// the operation deadline; yield briefly only when local I/O would block.
fn io_before<T>(until: Instant, mut operation: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    loop {
        let left = remaining(until)?;
        match operation() {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(left.min(Duration::from_millis(1)))
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

fn write_counted(
    s: &mut TcpStream,
    mut bytes: &[u8],
    until: Instant,
    count: &mut usize,
) -> io::Result<()> {
    while !bytes.is_empty() {
        let n = io_before(until, || s.write(bytes))?;
        if n == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        *count += n;
        bytes = &bytes[n..];
    }
    Ok(())
}

fn read_counted(
    s: &mut TcpStream,
    bytes: &mut [u8],
    until: Instant,
    count: &mut usize,
) -> io::Result<usize> {
    let n = io_before(until, || s.read(bytes))?;
    *count += n;
    Ok(n)
}

fn exchange(
    addr: SocketAddr,
    authority: &str,
    allow: bool,
    until: Instant,
    c: &mut Count,
) -> Result<()> {
    target(authority)?;
    ensure!(
        addr.ip() == Ipv4Addr::LOCALHOST && addr.port() != 0,
        "expected workload loopback proxy"
    );
    let mut stream = TcpStream::connect_timeout(&addr, remaining(until)?)?;
    stream.set_nonblocking(true)?;
    let head = format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n\r\n");
    write_counted(&mut stream, head.as_bytes(), until, &mut c.head_tx)?;
    // Read exactly the response head, without consuming or losing any subsequent data.
    let expected = if allow { GOOD } else { DENIED };
    let mut head = [0; DENIED.len()];
    let mut pos = 0;
    while pos < expected.len() {
        let n = read_counted(
            &mut stream,
            &mut head[pos..expected.len()],
            until,
            &mut c.head_rx,
        )?;
        ensure!(
            n > 0 && head[pos..pos + n] == expected[pos..pos + n],
            "unexpected proxy response"
        );
        pos += n;
    }

    if allow {
        // Matches the existing registered_direct_peer fixture, including its final NULs.
        let mut body = b"{\"kind\":\"flowmux\",\"value\":1}\n".repeat(200);
        body.extend_from_slice(&[0, 255, 0, 0]);
        write_counted(&mut stream, &body, until, &mut c.body_tx)?;
        stream.shutdown(Shutdown::Write)?;
        let mut offset = 0;
        let mut buf = [0; 1024];
        while offset < body.len() {
            let cap = buf.len().min(body.len() - offset);
            let n = read_counted(&mut stream, &mut buf[..cap], until, &mut c.body_rx)?;
            ensure!(n > 0, "truncated peer echo");
            ensure!(buf[..n] == body[offset..offset + n], "peer echo differs");
            offset += n;
        }
    }
    ensure!(
        read_counted(&mut stream, &mut [0], until, &mut c.body_rx)? == 0,
        "expected EOF"
    );
    remaining(until)?;
    Ok(())
}

fn row(c: &Count, cpu: ProcessTime, start: Instant, completed: bool) -> Result<serde_json::Value> {
    Ok(
        json!({"completed":completed,"connect_request_bytes":c.head_tx,"connect_response_bytes":c.head_rx,
        "application_sent_bytes":c.body_tx,"application_received_bytes":c.body_rx,
        "process_cpu_ns":u64::try_from(cpu.try_elapsed()?.as_nanos())?,"elapsed_ns":u64::try_from(start.elapsed().as_nanos())?}),
    )
}

fn targets(denied: &str, allowed: &str) -> Result<()> {
    target(denied)?;
    target(allowed)?;
    let denied_host = denied.split_once(':').context("missing denied port")?.0;
    let allowed_host = allowed.split_once(':').context("missing echo port")?.0;
    ensure!(
        allowed_host == "cooperator.mvm.peer"
            && denied_host.ends_with(".mvm.peer")
            && denied_host != allowed_host,
        "expected distinct fixture peers in the registered test domain"
    );
    Ok(())
}

fn run() -> Result<()> {
    #[cfg(target_os = "linux")]
    ensure!(
        mvm_agentd::vsock::current_uid().is_some_and(|uid| uid > 0),
        "non-root guest workload required"
    );
    #[cfg(not(target_os = "linux"))]
    ensure!(false, "requires a Linux guest workload");

    let mut args = std::env::args_os().skip(1);
    let denied = args.next().context("expected denied authority")?;
    let allowed = args.next().context("expected allowed authority")?;
    ensure!(args.next().is_none(), "expected exactly two authorities");
    let denied = denied.to_str().context("expected ASCII denied authority")?;
    let allowed = allowed
        .to_str()
        .context("expected ASCII allowed authority")?;
    targets(denied, allowed)?;
    let url = std::env::var_os("HTTP_PROXY")
        .and_then(|v| v.into_string().ok())
        .context("missing or invalid admitted HTTP proxy")?;
    let addr = proxy(&url)?;
    let until = Instant::now() + TIME_LIMIT;
    for (phase, authority, allow) in [("denied", denied, false), ("echo", allowed, true)] {
        let mut count = Count::default();
        let started = Instant::now();
        let cpu = ProcessTime::try_now()?;
        let result = exchange(addr, authority, allow, until, &mut count);
        let mut out = io::stdout().lock();
        writeln!(
            out,
            "{}",
            json!({"phase":phase,"guest_proxy":row(&count,cpu,started,result.is_ok())?})
        )?;
        out.flush()?;
        result?;
    }
    Ok(())
}

fn main() -> Result<()> {
    run()
}

#[cfg(test)]
#[path = "flowmux_proxy/tests.rs"]
mod tests;
