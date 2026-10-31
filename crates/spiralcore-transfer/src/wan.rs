//! Bounded HTTP relay adapter for public, synthetic two-host experiments.
//! HTTPS terminates at the configured relay; this is not end-to-end encryption.
use crate::{
    endpoint::{Authority, CodecCacheStats, Context, Endpoint, Limits},
    hex,
    protocol::Frame,
    Error, Id, Result, MAX_FRAME,
};
use cpu_time::ProcessTime;
use serde::Serialize;
use std::{
    collections::BTreeSet,
    io::{BufReader, Read, Write},
    net::{IpAddr, TcpListener, TcpStream},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;

pub const MAX_BATCH: usize = 20 << 20;
pub const MAX_BATCH_FRAMES: usize = 512;
const HEADER_LIMIT: usize = 16384;
const IO_TIMEOUT: Duration = Duration::from_secs(60);

pub fn parse_id(value: &str) -> Result<Id> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Malformed);
    }
    let mut id = [0; 32];
    for (i, b) in id.iter_mut().enumerate() {
        *b = u8::from_str_radix(&value[2 * i..2 * i + 2], 16).map_err(|_| Error::Malformed)?;
    }
    Ok(id)
}
pub fn context(scope: Id) -> Context {
    Context {
        tenant: "spiralcore-wan-experiment".into(),
        domain: "public-synthetic-only".into(),
        sender: "authorized-experiment-client".into(),
        recipient: "owned-experiment-cache".into(),
        mailbox: hex(&scope),
        capability: "temporary-capability-checked-by-http-adapter".into(),
        authorization_scope: "synthetic-object-transfer".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}
pub struct BenchAuthority;
impl Authority for BenchAuthority {
    fn authorize(&self, c: &Context) -> Result<()> {
        let scope = parse_id(&c.mailbox).map_err(|_| Error::Unauthorized)?;
        if *c == context(scope) {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
pub fn limits() -> Limits {
    Limits {
        resident_bytes: 448 << 20,
        max_states: 2048,
        ..Limits::default()
    }
}
fn add_frame(out: &mut Vec<u8>, frame: &Frame) -> Result<()> {
    let b = frame.encode()?;
    if out
        .len()
        .checked_add(4 + b.len())
        .is_none_or(|n| n > MAX_BATCH)
    {
        return Err(Error::Limit);
    }
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(&b);
    Ok(())
}
pub fn encode_batch(frames: &[Frame]) -> Result<Vec<u8>> {
    if frames.is_empty() || frames.len() > MAX_BATCH_FRAMES {
        return Err(Error::Limit);
    }
    let mut out = b"SCWBAT01".to_vec();
    out.extend_from_slice(&(frames.len() as u32).to_le_bytes());
    for f in frames {
        add_frame(&mut out, f)?;
    }
    Ok(out)
}
fn take<'a>(b: &mut &'a [u8], n: usize) -> Result<&'a [u8]> {
    if b.len() < n {
        return Err(Error::Malformed);
    }
    let (a, z) = b.split_at(n);
    *b = z;
    Ok(a)
}
fn number32(b: &mut &[u8]) -> Result<u32> {
    Ok(u32::from_le_bytes(
        take(b, 4)?.try_into().map_err(|_| Error::Malformed)?,
    ))
}
fn number64(b: &mut &[u8]) -> Result<u64> {
    Ok(u64::from_le_bytes(
        take(b, 8)?.try_into().map_err(|_| Error::Malformed)?,
    ))
}
fn frame_list(b: &mut &[u8]) -> Result<Vec<Frame>> {
    let count = number32(b)? as usize;
    if count == 0 || count > MAX_BATCH_FRAMES {
        return Err(Error::Limit);
    }
    let mut frames = Vec::with_capacity(count);
    for _ in 0..count {
        let len = number32(b)? as usize;
        if len > MAX_FRAME {
            return Err(Error::Limit);
        }
        frames.push(Frame::decode(take(b, len)?)?);
    }
    if !b.is_empty() {
        return Err(Error::Malformed);
    }
    Ok(frames)
}
pub fn decode_batch(mut bytes: &[u8]) -> Result<Vec<Frame>> {
    if bytes.len() > MAX_BATCH {
        return Err(Error::Limit);
    }
    if take(&mut bytes, 8)? != b"SCWBAT01" {
        return Err(Error::Malformed);
    }
    let frames = frame_list(&mut bytes)?;
    for pair in frames.windows(2) {
        if pair[0].transfer != pair[1].transfer || pair[0].seq.checked_add(1) != Some(pair[1].seq) {
            return Err(Error::Sequence);
        }
    }
    Ok(frames)
}
#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct WireMetrics {
    pub request_body_bytes: u64,
    pub response_body_bytes: u64,
    pub receiver_cpu_micros: u64,
    pub receiver_wall_micros: u64,
    pub http_requests: u64,
}
pub struct HttpClient {
    agent: ureq::Agent,
    url: String,
    token: String,
    scope: Id,
    metrics: WireMetrics,
}
impl HttpClient {
    pub fn new(url: &str, token: &str, scope: Id) -> Result<Self> {
        parse_id(token).map_err(|_| Error::Unauthorized)?;
        let u = url::Url::parse(url).map_err(|_| Error::Malformed)?;
        if !u.username().is_empty()
            || u.password().is_some()
            || u.query().is_some()
            || u.fragment().is_some()
            || u.path() != "/"
        {
            return Err(Error::Unauthorized);
        }
        let loopback = u
            .host_str()
            .and_then(|s| s.trim_matches(['[', ']']).parse::<IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback());
        if !(u.scheme() == "https" || u.scheme() == "http" && loopback) || u.host_str().is_none() {
            return Err(Error::Unauthorized);
        }
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(IO_TIMEOUT))
            .max_redirects(0)
            .max_redirects_will_error(true)
            .proxy(None)
            .build()
            .into();
        Ok(Self {
            agent,
            url: format!("{}/v1/batch", url.trim_end_matches('/')),
            token: token.into(),
            scope,
            metrics: WireMetrics::default(),
        })
    }
    pub fn metrics(&self) -> WireMetrics {
        self.metrics
    }
    pub fn exchange(&mut self, frames: &[Frame]) -> Result<Vec<Frame>> {
        let body = encode_batch(frames)?;
        self.metrics.request_body_bytes += body.len() as u64;
        self.metrics.http_requests += 1;
        let mut response = self
            .agent
            .post(&self.url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("X-Spiral-Scope", hex(&self.scope))
            .header("Content-Type", "application/octet-stream")
            .header("Accept-Encoding", "identity")
            .header("Cache-Control", "no-store")
            .send(body.as_slice())
            .map_err(|_| Error::Storage)?;
        if response.status().as_u16() != 200 || response.headers().get("content-encoding").is_some()
        {
            return Err(Error::Integrity);
        }
        let response = response
            .body_mut()
            .with_config()
            .limit(MAX_BATCH as u64)
            .read_to_vec()
            .map_err(|_| Error::Malformed)?;
        self.metrics.response_body_bytes += response.len() as u64;
        let mut b = response.as_slice();
        if take(&mut b, 8)? != b"SCWREP01" {
            return Err(Error::Malformed);
        }
        let cpu = number64(&mut b)?;
        let wall = number64(&mut b)?;
        if cpu > IO_TIMEOUT.as_micros() as u64 * 128 || wall > IO_TIMEOUT.as_micros() as u64 {
            return Err(Error::Integrity);
        }
        let replies = frame_list(&mut b)?;
        if replies.len() > frames.len() {
            return Err(Error::Integrity);
        }
        for (index, (reply, request)) in replies.iter().zip(frames).enumerate() {
            if reply.transfer != request.transfer || reply.seq != request.seq {
                return Err(Error::Integrity);
            }
            if (reply.kind == 103 || reply.kind == 127) && index + 1 != replies.len() {
                return Err(Error::Integrity);
            }
        }
        if replies.len() < frames.len()
            && replies
                .last()
                .is_none_or(|r| r.kind != 103 && r.kind != 127)
        {
            return Err(Error::Integrity);
        }
        self.metrics.receiver_cpu_micros = self
            .metrics
            .receiver_cpu_micros
            .checked_add(cpu)
            .ok_or(Error::Limit)?;
        self.metrics.receiver_wall_micros = self
            .metrics
            .receiver_wall_micros
            .checked_add(wall)
            .ok_or(Error::Limit)?;
        Ok(replies)
    }
}
fn remaining(deadline: Instant) -> std::io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::TimedOut, "request deadline"))
}
fn read_header(
    reader: &mut BufReader<TcpStream>,
    deadline: Instant,
    empty_timeout_is_eof: bool,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut header = Vec::new();
    while header.len() < HEADER_LIMIT {
        let timeout = match remaining(deadline) {
            Err(_) if empty_timeout_is_eof && header.is_empty() => return Ok(None),
            value => value?,
        };
        reader.get_mut().set_read_timeout(Some(timeout))?;
        let mut byte = [0];
        let count = match reader.read(&mut byte) {
            Err(e)
                if empty_timeout_is_eof
                    && header.is_empty()
                    && matches!(
                        e.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
            {
                return Ok(None)
            }
            value => value?,
        };
        if count == 0 {
            return if header.is_empty() {
                Ok(None)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "header",
                ))
            };
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            return Ok(Some(header));
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "header limit",
    ))
}
fn request(
    header: &[u8],
    token: &str,
    scopes: &BTreeSet<Id>,
) -> std::result::Result<(Id, usize, bool), u16> {
    let mut fields = [httparse::EMPTY_HEADER; 64];
    let mut r = httparse::Request::new(&mut fields);
    match r.parse(header) {
        Ok(httparse::Status::Complete(n)) if n == header.len() => {}
        _ => return Err(400),
    }
    if r.method != Some("POST") || r.path != Some("/v1/batch") || r.version != Some(1) {
        return Err(400);
    }
    let single = |name: &str| -> std::result::Result<Option<&[u8]>, u16> {
        let mut matched = r
            .headers
            .iter()
            .filter(|h| h.name.eq_ignore_ascii_case(name));
        let first = matched.next().map(|h| h.value);
        if matched.next().is_some() {
            Err(400)
        } else {
            Ok(first)
        }
    };
    let expected = format!("Bearer {token}");
    let auth = single("authorization")?.ok_or(403u16)?;
    if auth.len() != expected.len() || !bool::from(auth.ct_eq(expected.as_bytes())) {
        return Err(403);
    }
    let scope = single("x-spiral-scope")?
        .and_then(|b| std::str::from_utf8(b).ok())
        .ok_or(403u16)?;
    let scope = parse_id(scope).map_err(|_| 403u16)?;
    if !scopes.contains(&scope) {
        return Err(403);
    }
    if single("transfer-encoding")?.is_some()
        || single("content-encoding")?.is_some()
        || single("expect")?.is_some()
    {
        return Err(400);
    }
    if single("content-type")? != Some(b"application/octet-stream".as_slice()) {
        return Err(400);
    }
    let len = single("content-length")?
        .and_then(|b| std::str::from_utf8(b).ok())
        .ok_or(400u16)?;
    if len.is_empty() || !len.bytes().all(|b| b.is_ascii_digit()) {
        return Err(400);
    }
    let len: usize = len.parse().map_err(|_| 413u16)?;
    if !(12..=MAX_BATCH).contains(&len) {
        return Err(413);
    }
    let close = match single("connection")? {
        None => false,
        Some(value) => {
            let value = std::str::from_utf8(value).map_err(|_| 400u16)?;
            let mut close = false;
            for token in value.split(',') {
                let token = token.trim();
                if token.is_empty()
                    || !token
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
                {
                    return Err(400);
                }
                close |= token.eq_ignore_ascii_case("close");
            }
            close
        }
    };
    Ok((scope, len, close))
}
fn response(
    stream: &mut TcpStream,
    status: u16,
    body: &[u8],
    close: bool,
    deadline: Instant,
) -> std::io::Result<()> {
    let header=format!("HTTP/1.1 {status} {}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: {}\r\n\r\n",
        if status==200{"OK"}else{"Rejected"},body.len(),if close{"close"}else{"keep-alive"});
    for mut data in [header.as_bytes(), body] {
        while !data.is_empty() {
            stream.set_write_timeout(Some(remaining(deadline)?))?;
            let n = stream.write(data)?;
            if n == 0 {
                return Err(std::io::ErrorKind::WriteZero.into());
            }
            data = &data[n..];
        }
    }
    Ok(())
}
fn process(
    endpoint: &mut Endpoint<BenchAuthority>,
    scope: Id,
    body: &[u8],
    start: Instant,
    deadline: Instant,
) -> Result<Vec<u8>> {
    let wall = Instant::now();
    let cpu = ProcessTime::try_now()?;
    let frames = decode_batch(body)?;
    let mut replies = Vec::new();
    for f in &frames {
        if Instant::now() >= deadline {
            return Err(Error::Expired);
        }
        let reply = endpoint
            .handle(&context(scope), f, start.elapsed().as_secs())
            .unwrap_or_else(|e| f.reply(127, (e as u16).to_le_bytes().to_vec()));
        let terminal = reply.kind == 103 || reply.kind == 127;
        replies.push(reply);
        if terminal {
            break;
        }
    }
    let mut out = b"SCWREP01".to_vec();
    out.extend_from_slice(&[0; 16]);
    out.extend_from_slice(&(replies.len() as u32).to_le_bytes());
    for r in replies {
        add_frame(&mut out, &r)?;
    }
    out[8..16].copy_from_slice(&(cpu.try_elapsed()?.as_micros() as u64).to_le_bytes());
    out[16..24].copy_from_slice(&(wall.elapsed().as_micros() as u64).to_le_bytes());
    Ok(out)
}
/// Optional experiment telemetry. No request payload or credential is included.
#[derive(Debug, Clone, Copy)]
pub struct RequestObservation {
    pub request_ordinal: usize,
    pub connection_ordinal: usize,
    pub request_on_connection: usize,
    pub scope: Id,
    pub http_status: u16,
    pub codec_cache: CodecCacheStats,
}

/// Opt-in bounded reuse for the serial experiment origin. Defaults remain close.
#[derive(Debug, Clone, Copy)]
pub enum ConnectionPolicy {
    Close,
    KeepAlive {
        max_requests_per_connection: usize,
        idle_timeout: Duration,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct ServeConfig {
    pub duration: Duration,
    pub max_requests: usize,
    pub connection_policy: ConnectionPolicy,
}

impl ServeConfig {
    fn validate(self) -> Result<()> {
        if self.max_requests == 0
            || self.max_requests > 16384
            || self.duration > Duration::from_secs(3600)
        {
            return Err(Error::Limit);
        }
        if let ConnectionPolicy::KeepAlive {
            max_requests_per_connection,
            idle_timeout,
        } = self.connection_policy
        {
            if !(1..=64).contains(&max_requests_per_connection)
                || idle_timeout < Duration::from_millis(1)
                || idle_timeout > Duration::from_secs(1)
            {
                return Err(Error::Limit);
            }
        }
        Ok(())
    }
}

pub fn serve_configured(
    listener: TcpListener,
    endpoint: Endpoint<BenchAuthority>,
    token: &str,
    scopes: &BTreeSet<Id>,
    config: ServeConfig,
) -> Result<()> {
    serve_inner(listener, endpoint, token, scopes, config, None)
}

pub fn serve_observed_configured(
    listener: TcpListener,
    endpoint: Endpoint<BenchAuthority>,
    token: &str,
    scopes: &BTreeSet<Id>,
    config: ServeConfig,
    mut observer: impl FnMut(RequestObservation) -> Result<()>,
) -> Result<()> {
    serve_inner(
        listener,
        endpoint,
        token,
        scopes,
        config,
        Some(&mut observer),
    )
}

pub fn serve(
    listener: TcpListener,
    endpoint: Endpoint<BenchAuthority>,
    token: &str,
    scopes: &BTreeSet<Id>,
    duration: Duration,
    max_requests: usize,
) -> Result<()> {
    serve_inner(
        listener,
        endpoint,
        token,
        scopes,
        ServeConfig {
            duration,
            max_requests,
            connection_policy: ConnectionPolicy::Close,
        },
        None,
    )
}

/// Observe admitted, fully received requests after handler timing and before
/// response I/O. Observer errors terminate the run rather than lose evidence.
pub fn serve_observed(
    listener: TcpListener,
    endpoint: Endpoint<BenchAuthority>,
    token: &str,
    scopes: &BTreeSet<Id>,
    duration: Duration,
    max_requests: usize,
    mut observer: impl FnMut(RequestObservation) -> Result<()>,
) -> Result<()> {
    serve_inner(
        listener,
        endpoint,
        token,
        scopes,
        ServeConfig {
            duration,
            max_requests,
            connection_policy: ConnectionPolicy::Close,
        },
        Some(&mut observer),
    )
}

fn serve_inner(
    listener: TcpListener,
    mut endpoint: Endpoint<BenchAuthority>,
    token: &str,
    scopes: &BTreeSet<Id>,
    config: ServeConfig,
    mut observer: Option<&mut dyn FnMut(RequestObservation) -> Result<()>>,
) -> Result<()> {
    parse_id(token).map_err(|_| Error::Unauthorized)?;
    config.validate()?;
    if !listener.local_addr()?.ip().is_loopback() || scopes.is_empty() || scopes.len() > 4096 {
        return Err(Error::Limit);
    }
    listener.set_nonblocking(true)?;
    let start = Instant::now();
    let end = start + config.duration;
    let mut requests = 0;
    let mut connections = 0;
    let (connection_limit, idle_timeout) = match config.connection_policy {
        ConnectionPolicy::Close => (1, Duration::ZERO),
        ConnectionPolicy::KeepAlive {
            max_requests_per_connection,
            idle_timeout,
        } => (max_requests_per_connection, idle_timeout),
    };
    while Instant::now() < end && requests < config.max_requests {
        let (stream, peer) = match listener.accept() {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        if !peer.ip().is_loopback() {
            continue;
        }
        stream.set_nonblocking(false)?;
        stream.set_nodelay(true)?;
        connections += 1;
        let mut reader = BufReader::new(stream);
        for request_index in 0..connection_limit {
            if Instant::now() >= end || requests >= config.max_requests {
                break;
            }
            let deadline = (Instant::now() + IO_TIMEOUT).min(end);
            let header_timeout = if request_index == 0 {
                Duration::from_secs(5)
            } else {
                idle_timeout
            };
            let header = match read_header(
                &mut reader,
                (Instant::now() + header_timeout).min(deadline),
                request_index != 0,
            ) {
                Ok(Some(h)) => h,
                Ok(None) => break,
                Err(_) => {
                    requests += 1;
                    let _ = response(reader.get_mut(), 400, &[], true, deadline);
                    break;
                }
            };
            requests += 1;
            let (scope, len, client_close) = match request(&header, token, scopes) {
                Ok(r) => r,
                Err(status) => {
                    let _ = response(reader.get_mut(), status, &[], true, deadline);
                    break;
                }
            };
            let mut body = vec![0; len];
            let mut filled = 0;
            while filled < len {
                let Ok(timeout) = remaining(deadline) else {
                    break;
                };
                if reader.get_mut().set_read_timeout(Some(timeout)).is_err() {
                    break;
                }
                match reader.read(&mut body[filled..]) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => filled += n,
                }
            }
            if filled != len {
                let _ = response(reader.get_mut(), 400, &[], true, deadline);
                break;
            }
            let (status, body) = match process(&mut endpoint, scope, &body, start, deadline) {
                Ok(b) => (200, b),
                Err(_) => (400, Vec::new()),
            };
            if let Some(observe) = observer.as_mut() {
                observe(RequestObservation {
                    request_ordinal: requests,
                    connection_ordinal: connections,
                    request_on_connection: request_index + 1,
                    scope,
                    http_status: status,
                    codec_cache: endpoint.codec_cache_stats(),
                })?;
            }
            let close = client_close
                || status != 200
                || request_index + 1 == connection_limit
                || requests >= config.max_requests
                || Instant::now() >= end;
            if response(reader.get_mut(), status, &body, close, deadline).is_err() || close {
                break;
            }
        }
    }
    Ok(())
}
