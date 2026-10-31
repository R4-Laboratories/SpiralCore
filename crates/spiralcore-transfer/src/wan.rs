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

/// Sanitized diagnostics only: every label comes from a fixed local allowlist.
/// No error strings, URLs, credentials, headers, or payload bytes are retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TransportFailure {
    pub stage: &'static str,
    pub category: &'static str,
    pub http_status: Option<u16>,
    pub io_kind: Option<&'static str>,
    pub timeout_phase: Option<&'static str>,
}
fn io_label(kind: std::io::ErrorKind) -> &'static str {
    use std::io::ErrorKind;
    match kind {
        ErrorKind::TimedOut => "timed_out",
        ErrorKind::WouldBlock => "would_block",
        ErrorKind::UnexpectedEof => "unexpected_eof",
        ErrorKind::ConnectionReset => "connection_reset",
        ErrorKind::ConnectionAborted => "connection_aborted",
        ErrorKind::ConnectionRefused => "connection_refused",
        ErrorKind::BrokenPipe => "broken_pipe",
        ErrorKind::NotConnected => "not_connected",
        ErrorKind::Interrupted => "interrupted",
        ErrorKind::InvalidData => "invalid_data",
        ErrorKind::InvalidInput => "invalid_input",
        ErrorKind::WriteZero => "write_zero",
        ErrorKind::PermissionDenied => "permission_denied",
        _ => "other",
    }
}
impl TransportFailure {
    fn from_error(stage: &'static str, error: &ureq::Error) -> Self {
        let mut failure = Self {
            stage,
            category: "other",
            http_status: None,
            io_kind: None,
            timeout_phase: None,
        };
        failure.category = match error {
            ureq::Error::StatusCode(status) => {
                failure.http_status = Some(*status);
                "http_status"
            }
            ureq::Error::Io(error) => {
                failure.io_kind = Some(io_label(error.kind()));
                "io"
            }
            ureq::Error::Timeout(phase) => {
                failure.timeout_phase = Some(match phase {
                    ureq::Timeout::PerRead => "per_read",
                    ureq::Timeout::PerWrite => "per_write",
                    ureq::Timeout::Global => "global",
                    ureq::Timeout::PerCall => "per_call",
                    ureq::Timeout::Resolve => "resolve",
                    ureq::Timeout::Connect => "connect",
                    ureq::Timeout::SendRequest => "send_request",
                    ureq::Timeout::Await100 => "await_100",
                    ureq::Timeout::SendBody => "send_body",
                    ureq::Timeout::RecvResponse => "receive_response",
                    ureq::Timeout::RecvBody => "receive_body",
                    _ => "other",
                });
                "timeout"
            }
            ureq::Error::Protocol(_) => "protocol",
            ureq::Error::Http(_) => "http",
            ureq::Error::BadUri(_) => "bad_uri",
            ureq::Error::HostNotFound => "host_not_found",
            ureq::Error::ConnectionFailed => "connection_failed",
            ureq::Error::RedirectFailed | ureq::Error::TooManyRedirects => "redirect",
            ureq::Error::Tls(_) => "tls",
            ureq::Error::BodyExceedsLimit(_) => "body_limit",
            ureq::Error::LargeResponseHeader(_, _) => "response_header_limit",
            _ => "other",
        };
        failure
    }
}
pub struct HttpClient {
    agent: ureq::Agent,
    url: String,
    token: String,
    scope: Id,
    metrics: WireMetrics,
    last_transport_failure: Option<TransportFailure>,
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
            last_transport_failure: None,
        })
    }
    pub fn metrics(&self) -> WireMetrics {
        self.metrics
    }
    /// Latest exchange's transport failure, cleared before every new exchange.
    /// Local codec/validation errors do not manufacture a transport failure.
    pub fn last_transport_failure(&self) -> Option<TransportFailure> {
        self.last_transport_failure
    }
    pub fn exchange(&mut self, frames: &[Frame]) -> Result<Vec<Frame>> {
        self.last_transport_failure = None;
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
            .map_err(|error| {
                self.last_transport_failure = Some(TransportFailure::from_error("send", &error));
                Error::Storage
            })?;
        if response.status().as_u16() != 200 || response.headers().get("content-encoding").is_some()
        {
            self.last_transport_failure = Some(TransportFailure {
                stage: "response_headers",
                category: if response.status().as_u16() != 200 {
                    "unexpected_status"
                } else {
                    "content_encoding"
                },
                http_status: Some(response.status().as_u16()),
                io_kind: None,
                timeout_phase: None,
            });
            return Err(Error::Integrity);
        }
        let response = response
            .body_mut()
            .with_config()
            .limit(MAX_BATCH as u64)
            .read_to_vec()
            .map_err(|error| {
                self.last_transport_failure =
                    Some(TransportFailure::from_error("response_body", &error));
                Error::Malformed
            })?;
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
struct HeaderDiagnostic {
    bytes: usize,
    reason: &'static str,
    io_kind: Option<&'static str>,
}
impl HeaderDiagnostic {
    fn error(&mut self, error: &std::io::Error) {
        self.reason = match error.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => "header_timeout",
            std::io::ErrorKind::UnexpectedEof => "header_truncated",
            _ => "header_io",
        };
        self.io_kind = Some(io_label(error.kind()));
    }
}
fn read_header(
    reader: &mut BufReader<TcpStream>,
    deadline: Instant,
    empty_timeout_is_eof: bool,
    diagnostic: &mut HeaderDiagnostic,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut header = Vec::new();
    while header.len() < HEADER_LIMIT {
        let timeout = match remaining(deadline) {
            Err(error) if empty_timeout_is_eof && header.is_empty() => {
                diagnostic.reason = "idle_timeout";
                diagnostic.io_kind = Some(io_label(error.kind()));
                return Ok(None);
            }
            value => value.inspect_err(|e| diagnostic.error(e))?,
        };
        reader
            .get_mut()
            .set_read_timeout(Some(timeout))
            .inspect_err(|e| diagnostic.error(e))?;
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
                diagnostic.reason = "idle_timeout";
                diagnostic.io_kind = Some(io_label(e.kind()));
                return Ok(None);
            }
            value => value.inspect_err(|e| diagnostic.error(e))?,
        };
        if count == 0 {
            return if header.is_empty() {
                diagnostic.reason = "peer_eof";
                Ok(None)
            } else {
                diagnostic.reason = "header_truncated";
                diagnostic.io_kind = Some("unexpected_eof");
                Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "header",
                ))
            };
        }
        header.push(byte[0]);
        diagnostic.bytes = header.len();
        if header.ends_with(b"\r\n\r\n") {
            return Ok(Some(header));
        }
    }
    diagnostic.reason = "header_limit";
    diagnostic.io_kind = Some("invalid_data");
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

/// Optional connection diagnostics, separate from admitted request observations.
/// `request_ordinal` is the next/assigned attempt ordinal. Idle and clean EOF
/// do not increment the global counter (`request_counted == false`). Elapsed
/// time starts immediately before this attempt's header/idle wait. Fixed labels
/// and bounded byte counts never include request content or credentials.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ConnectionObservation {
    pub connection_ordinal: usize,
    pub request_ordinal: usize,
    pub request_on_connection: usize,
    pub request_counted: bool,
    pub reason: &'static str,
    pub http_status: Option<u16>,
    pub io_kind: Option<&'static str>,
    pub header_bytes: usize,
    pub elapsed_micros: u64,
}
impl ConnectionObservation {
    fn emit(
        mut self,
        observer: &mut Option<&mut dyn FnMut(ConnectionObservation) -> Result<()>>,
        reason: &'static str,
        status: Option<u16>,
        io_kind: Option<&'static str>,
        since: Instant,
    ) -> Result<()> {
        if let Some(observer) = observer.as_mut() {
            self.reason = reason;
            self.http_status = status;
            self.io_kind = io_kind;
            self.elapsed_micros = since.elapsed().as_micros() as u64;
            observer(self)?;
        }
        Ok(())
    }
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
    serve_inner(listener, endpoint, token, scopes, config, None, None)
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
        None,
    )
}

/// Stream sanitized connection events to a caller-owned bounded sink. Errors
/// terminate serving rather than silently lose evidence. Existing callbacks and
/// HTTP policy remain unchanged; diagnostics do not retain an in-memory log.
pub fn serve_diagnosed_configured(
    listener: TcpListener,
    endpoint: Endpoint<BenchAuthority>,
    token: &str,
    scopes: &BTreeSet<Id>,
    config: ServeConfig,
    mut observer: impl FnMut(RequestObservation) -> Result<()>,
    mut diagnostic: impl FnMut(ConnectionObservation) -> Result<()>,
) -> Result<()> {
    serve_inner(
        listener,
        endpoint,
        token,
        scopes,
        config,
        Some(&mut observer),
        Some(&mut diagnostic),
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
        None,
    )
}

fn serve_inner(
    listener: TcpListener,
    mut endpoint: Endpoint<BenchAuthority>,
    token: &str,
    scopes: &BTreeSet<Id>,
    config: ServeConfig,
    mut observer: Option<&mut dyn FnMut(RequestObservation) -> Result<()>>,
    mut diagnostic: Option<&mut dyn FnMut(ConnectionObservation) -> Result<()>>,
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
            let attempt_start = Instant::now();
            let mut event = ConnectionObservation {
                connection_ordinal: connections,
                request_ordinal: requests + 1,
                request_on_connection: request_index + 1,
                request_counted: false,
                reason: "other",
                http_status: None,
                io_kind: None,
                header_bytes: 0,
                elapsed_micros: 0,
            };
            let deadline = (Instant::now() + IO_TIMEOUT).min(end);
            let header_timeout = if request_index == 0 {
                Duration::from_secs(5)
            } else {
                idle_timeout
            };
            let mut header_diagnostic = HeaderDiagnostic {
                bytes: 0,
                reason: "header_io",
                io_kind: None,
            };
            let header = match read_header(
                &mut reader,
                (Instant::now() + header_timeout).min(deadline),
                request_index != 0,
                &mut header_diagnostic,
            ) {
                Ok(Some(h)) => h,
                Ok(None) => {
                    event.emit(
                        &mut diagnostic,
                        header_diagnostic.reason,
                        None,
                        header_diagnostic.io_kind,
                        attempt_start,
                    )?;
                    break;
                }
                Err(_) => {
                    requests += 1;
                    event.request_counted = true;
                    event.header_bytes = header_diagnostic.bytes;
                    let _ = response(reader.get_mut(), 400, &[], true, deadline);
                    event.emit(
                        &mut diagnostic,
                        header_diagnostic.reason,
                        Some(400),
                        header_diagnostic.io_kind,
                        attempt_start,
                    )?;
                    break;
                }
            };
            requests += 1;
            event.request_counted = true;
            event.header_bytes = header.len();
            let (scope, len, client_close) = match request(&header, token, scopes) {
                Ok(r) => r,
                Err(status) => {
                    let _ = response(reader.get_mut(), status, &[], true, deadline);
                    event.emit(
                        &mut diagnostic,
                        "request_rejected",
                        Some(status),
                        None,
                        attempt_start,
                    )?;
                    break;
                }
            };
            let mut body = vec![0; len];
            let mut filled = 0;
            let mut body_reason = "body_truncated";
            let mut body_io_kind = None;
            while filled < len {
                let Ok(timeout) = remaining(deadline) else {
                    body_reason = "body_timeout";
                    body_io_kind = Some("timed_out");
                    break;
                };
                if let Err(error) = reader.get_mut().set_read_timeout(Some(timeout)) {
                    body_reason = "body_io";
                    body_io_kind = Some(io_label(error.kind()));
                    break;
                }
                match reader.read(&mut body[filled..]) {
                    Ok(0) => break,
                    Err(error) => {
                        body_reason = if matches!(
                            error.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) {
                            "body_timeout"
                        } else {
                            "body_io"
                        };
                        body_io_kind = Some(io_label(error.kind()));
                        break;
                    }
                    Ok(n) => filled += n,
                }
            }
            if filled != len {
                let _ = response(reader.get_mut(), 400, &[], true, deadline);
                event.emit(
                    &mut diagnostic,
                    body_reason,
                    Some(400),
                    body_io_kind,
                    attempt_start,
                )?;
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
            if let Err(error) = response(reader.get_mut(), status, &body, close, deadline) {
                event.emit(
                    &mut diagnostic,
                    "response_io",
                    Some(status),
                    Some(io_label(error.kind())),
                    attempt_start,
                )?;
                break;
            }
            if close {
                let reason = if status != 200 {
                    "process_rejected"
                } else if client_close {
                    "client_close"
                } else if requests >= config.max_requests {
                    "global_limit"
                } else if Instant::now() >= end {
                    "server_deadline"
                } else if matches!(config.connection_policy, ConnectionPolicy::Close) {
                    "close_policy"
                } else {
                    "connection_limit"
                };
                event.emit(&mut diagnostic, reason, Some(status), None, attempt_start)?;
                break;
            }
        }
    }
    Ok(())
}
