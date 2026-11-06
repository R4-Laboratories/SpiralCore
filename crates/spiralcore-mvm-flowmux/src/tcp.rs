use crate::{Binding, Error, HEADER_LEN, Result, record};
use spiralcore_transfer::zstd_whole::{PreparedDecoder, PreparedEncoder};
use std::{
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub wire_bytes: u64,
    pub payload_bytes: u64,
    pub records: u64,
    pub compressed_records: u64,
}
impl Counters {
    fn wire(&mut self, n: usize) -> Result<()> {
        self.wire_bytes = self.wire_bytes.checked_add(n as u64).ok_or(Error::Limit)?;
        Ok(())
    }
    fn complete(&mut self, payload: usize, compressed: bool) -> Result<()> {
        self.payload_bytes = self
            .payload_bytes
            .checked_add(payload as u64)
            .ok_or(Error::Limit)?;
        self.records = self.records.checked_add(1).ok_or(Error::Limit)?;
        self.compressed_records = self
            .compressed_records
            .checked_add(u64::from(compressed))
            .ok_or(Error::Limit)?;
        Ok(())
    }
}
enum Carrier {
    Tcp(TcpStream),
    Tls(crate::tls::TlsCarrier),
}
impl Carrier {
    fn peek(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(socket) => socket.peek(bytes),
            Self::Tls(socket) => socket.peek(bytes),
        }
    }
    fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            Self::Tcp(socket) => Ok(Self::Tcp(socket.try_clone()?)),
            Self::Tls(socket) => Ok(Self::Tls(socket.try_clone()?)),
        }
    }
    fn shutdown(&self, how: Shutdown) -> std::io::Result<()> {
        match self {
            Self::Tcp(socket) => socket.shutdown(how),
            Self::Tls(socket) => socket.shutdown(how),
        }
    }
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            Self::Tcp(socket) => checked_timeout(socket.set_read_timeout(timeout), timeout, || {
                socket.read_timeout()
            }),
            Self::Tls(socket) => socket.set_read_timeout(timeout),
        }
    }
    fn set_write_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            Self::Tcp(socket) => {
                checked_timeout(socket.set_write_timeout(timeout), timeout, || {
                    socket.write_timeout()
                })
            }
            Self::Tls(socket) => socket.set_write_timeout(timeout),
        }
    }
}
fn checked_timeout(
    result: std::io::Result<()>,
    requested: Option<Duration>,
    existing: impl FnOnce() -> std::io::Result<Option<Duration>>,
) -> std::io::Result<()> {
    // Darwin can reject setsockopt after peer close while exact response bytes
    // remain readable. Only an observed timeout at least as strict as the fresh
    // original-deadline slice can preserve that operation's bound.
    if cfg!(target_os = "macos")
        && result
            .as_ref()
            .is_err_and(|error| error.raw_os_error() == Some(22))
        && let (Some(requested), Ok(Some(actual))) = (requested, existing())
        && !actual.is_zero()
        && actual <= requested
    {
        return Ok(());
    }
    result
}
impl Read for Carrier {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(socket) => socket.read(bytes),
            Self::Tls(socket) => socket.read(bytes),
        }
    }
}
impl Write for Carrier {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(socket) => socket.write(bytes),
            Self::Tls(socket) => socket.write(bytes),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tcp(socket) => socket.flush(),
            Self::Tls(socket) => socket.flush(),
        }
    }
}
struct Control {
    delivery_fenced: AtomicBool,
    scope: Option<crate::scope::ScopeAuthority>,
    socket: Carrier,
    stopped: AtomicBool,
}
impl Control {
    fn stop(&self) {
        self.delivery_fenced.store(true, Ordering::Release);
        self.close();
    }
    fn close(&self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self.socket.shutdown(Shutdown::Both);
    }
    fn stop_on_drop(&self) {
        if !self.stopped.load(Ordering::Acquire) {
            self.stop();
        }
    }
    fn check_delivery(&self) -> Result<()> {
        self.check_scope()?;
        if self.delivery_fenced.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    fn check(&self) -> Result<()> {
        self.check_delivery()?;
        if self.stopped.load(Ordering::Acquire) {
            Err(Error::Closed)
        } else {
            Ok(())
        }
    }
    fn check_scope(&self) -> Result<()> {
        if let Some(scope) = &self.scope {
            scope.check()?;
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct Cancellation {
    control: Arc<Control>,
}
impl Cancellation {
    pub(crate) fn check_delivery(&self) -> Result<()> {
        self.control.check_delivery()
    }
    pub(crate) fn complete(&self) {
        self.control.close();
    }
    pub fn cancel(&self) {
        self.control.stop();
    }
    pub(crate) fn check(&self) -> Result<()> {
        self.control.check()
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Data(Vec<u8>),
    HalfClose,
    Reset,
    Cancel,
}

pub struct TcpMapping {
    tx: Tx,
    rx: Rx,
}
impl TcpMapping {
    /// A connected, explicitly selected peer is required. This is not admission
    /// or peer authentication and never opens a destination socket.
    pub fn from_connected(
        socket: TcpStream,
        binding: Binding,
        expected_peer: SocketAddr,
        timeout: Duration,
    ) -> Result<Self> {
        Self::with_profile(socket, binding, expected_peer, timeout, false)
    }
    pub(crate) fn with_profile(
        socket: TcpStream,
        binding: Binding,
        expected_peer: SocketAddr,
        timeout: Duration,
        object: bool,
    ) -> Result<Self> {
        binding.validate()?;
        if timeout.is_zero() || timeout > Duration::from_secs(30) {
            return Err(Error::Limit);
        }
        if socket.peer_addr()? != expected_peer {
            return Err(Error::PeerMismatch);
        }
        socket.set_nodelay(true)?;
        Self::with_carrier(Carrier::Tcp(socket), binding, timeout, object, 0, 0, None)
    }
    pub(crate) fn with_authenticated(
        socket: crate::tls::TlsCarrier,
        scope: &crate::scope::ScopeAuthority,
        timeout: Duration,
        role: crate::Role,
    ) -> Result<Self> {
        scope.check_carrier(&socket, role)?;
        Self::with_carrier(
            Carrier::Tls(socket),
            scope.binding()?,
            timeout,
            true,
            role.tag(),
            role.remote().tag(),
            Some(scope.clone()),
        )
    }
    fn with_carrier(
        mut socket: Carrier,
        binding: Binding,
        timeout: Duration,
        object: bool,
        local_role: u8,
        remote_role: u8,
        scope: Option<crate::scope::ScopeAuthority>,
    ) -> Result<Self> {
        binding.validate()?;
        if timeout.is_zero() || timeout > Duration::from_secs(30) {
            return Err(Error::Limit);
        }
        // Establish a real bounded timeout while connected; a first receive
        // after the peer's last descriptor closes must never accept None.
        let initial = Some(timeout.min(Duration::from_millis(20)));
        socket.set_read_timeout(initial)?;
        socket.set_write_timeout(initial)?;
        let read = socket.try_clone()?;
        let control = Arc::new(Control {
            delivery_fenced: AtomicBool::new(false),
            scope,
            socket: socket.try_clone()?,
            stopped: AtomicBool::new(false),
        });
        let cap = if object {
            spiralcore_transfer::MAX_FRAME
        } else {
            record::body_limit()?
        };
        Ok(Self {
            tx: Tx {
                role: local_role,
                socket,
                binding,
                sequence: 0,
                ended: false,
                timeout,
                encoder: if object {
                    None
                } else {
                    Some(PreparedEncoder::new()?)
                },
                counters: Counters::default(),
                control: control.clone(),
            },
            rx: Rx {
                role: remote_role,
                socket: read,
                binding,
                sequence: 0,
                ended: false,
                timeout,
                cap,
                data_kind: if object { record::OBJECT } else { record::DATA },
                decoder: if object {
                    None
                } else {
                    Some(PreparedDecoder::new()?)
                },
                counters: Counters::default(),
                control,
            },
        })
    }
    pub fn cancellation(&self) -> Cancellation {
        Cancellation {
            control: self.tx.control.clone(),
        }
    }
    pub fn split(self) -> (Tx, Rx) {
        (self.tx, self.rx)
    }
}
fn deadline(timeout: Duration) -> Result<Instant> {
    Instant::now().checked_add(timeout).ok_or(Error::Limit)
}
fn poll_budget(until: Instant) -> Result<Duration> {
    let left = until.saturating_duration_since(Instant::now());
    if left.is_zero() {
        Err(Error::Timeout)
    } else {
        Ok(left.min(Duration::from_millis(20)))
    }
}
fn transient(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
    )
}

pub struct Tx {
    socket: Carrier,
    pub(crate) role: u8,
    binding: Binding,
    sequence: u64,
    ended: bool,
    timeout: Duration,
    encoder: Option<PreparedEncoder>,
    counters: Counters,
    control: Arc<Control>,
}
impl Tx {
    pub fn counters(&self) -> Counters {
        self.counters
    }
    fn write_record(
        &mut self,
        kind: u8,
        body: &[u8],
        payload: usize,
        compressed: bool,
        until: Instant,
    ) -> Result<()> {
        let next = self.sequence.checked_add(1).ok_or(Error::Sequence)?;
        let mut wire = record::header(self.binding, self.sequence, kind, body.len(), self.role)?;
        wire.extend_from_slice(body);
        let mut offset = 0;
        while offset < wire.len() {
            self.control.check()?;
            self.socket.set_write_timeout(Some(poll_budget(until)?))?;
            match self.socket.write(&wire[offset..]) {
                Ok(0) => return Err(Error::Truncated),
                Ok(n) => {
                    self.counters.wire(n)?;
                    offset += n;
                }
                Err(error) if transient(&error) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        loop {
            self.control.check()?;
            self.socket.set_write_timeout(Some(poll_budget(until)?))?;
            match self.socket.flush() {
                Ok(()) => break,
                Err(error) if transient(&error) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        poll_budget(until)?;
        self.counters.complete(payload, compressed)?;
        self.sequence = next;
        Ok(())
    }
    pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
        if self.ended {
            return Err(Error::Closed);
        }
        self.control.check()?;
        let until = deadline(self.timeout)?;
        let (body, compressed) =
            record::encode(self.encoder.as_mut().ok_or(Error::Protocol)?, bytes)?;
        let result = self.write_record(record::DATA, &body, bytes.len(), compressed, until);
        if result.is_err() {
            self.control.stop();
        }
        result
    }
    fn terminal(&mut self, kind: u8) -> Result<()> {
        if self.ended {
            return Err(Error::Closed);
        }
        self.control.check()?;
        let result = self.write_record(kind, &[], 0, false, deadline(self.timeout)?);
        if result.is_err() {
            self.control.stop();
            return result;
        }
        self.ended = true;
        if kind == record::FIN {
            if let Err(error) = self.socket.shutdown(Shutdown::Write) {
                self.control.stop();
                return Err(error.into());
            }
        } else {
            self.control.stop();
        }
        Ok(())
    }
    pub fn half_close(&mut self) -> Result<()> {
        self.terminal(record::FIN)
    }
    pub fn reset(&mut self) -> Result<()> {
        self.terminal(record::RESET)
    }
    pub fn cancel(&mut self) -> Result<()> {
        self.terminal(record::CANCEL)
    }
    pub(crate) fn send_object(
        &mut self,
        frame: &spiralcore_transfer::protocol::Frame,
        purpose: crate::Purpose,
        until: Instant,
    ) -> Result<()> {
        if self.ended {
            return Err(Error::Closed);
        }
        let bytes = frame.encode()?;
        let kind = match purpose {
            crate::Purpose::Application => record::OBJECT,
            crate::Purpose::Artifact => record::ARTIFACT,
        };
        let result = self.write_record(kind, &bytes, bytes.len(), false, until);
        if result.is_err() {
            self.control.stop();
        }
        result
    }
    pub(crate) fn application_half_close(&mut self, until: Instant) -> Result<()> {
        let result = self.write_record(record::APP_FIN, &[], 0, false, until);
        if result.is_err() {
            self.control.stop();
        }
        result
    }
}
impl Drop for Tx {
    fn drop(&mut self) {
        if !self.ended {
            self.control.stop_on_drop();
        }
    }
}

pub struct Rx {
    socket: Carrier,
    pub(crate) role: u8,
    binding: Binding,
    sequence: u64,
    ended: bool,
    timeout: Duration,
    cap: usize,
    data_kind: u8,
    decoder: Option<PreparedDecoder>,
    counters: Counters,
    control: Arc<Control>,
}
impl Rx {
    pub fn counters(&self) -> Counters {
        self.counters
    }
    fn fill(&mut self, buffer: &mut [u8], until: Instant) -> Result<()> {
        let mut offset = 0;
        while offset < buffer.len() {
            self.control.check()?;
            self.socket.set_read_timeout(Some(poll_budget(until)?))?;
            match self.socket.read(&mut buffer[offset..]) {
                Ok(0) => return Err(Error::Truncated),
                Ok(n) => {
                    self.counters.wire(n)?;
                    offset += n;
                }
                Err(error) if transient(&error) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        poll_budget(until)?;
        Ok(())
    }
    fn receive_inner(&mut self) -> Result<Event> {
        let next = self.sequence.checked_add(1).ok_or(Error::Sequence)?;
        let until = deadline(self.timeout)?;
        let mut header = [0; HEADER_LEN];
        self.fill(&mut header, until)?;
        let (kind, len) = record::parse_header(
            &header,
            self.binding,
            self.sequence,
            self.cap,
            self.data_kind,
            self.role,
        )?;
        let event = if kind == record::DATA {
            let mut body = vec![0; len];
            self.fill(&mut body, until)?;
            let (bytes, compressed) =
                record::decode(self.decoder.as_mut().ok_or(Error::Protocol)?, &body)?;
            poll_budget(until)?;
            self.counters.complete(bytes.len(), compressed)?;
            Event::Data(bytes)
        } else {
            self.counters.complete(0, false)?;
            self.ended = true;
            match kind {
                record::FIN => Event::HalfClose,
                record::RESET => Event::Reset,
                record::CANCEL => Event::Cancel,
                _ => return Err(Error::Protocol),
            }
        };
        self.sequence = next;
        Ok(event)
    }
    pub fn receive(&mut self) -> Result<Event> {
        if self.ended {
            return Err(Error::Closed);
        }
        let result = self.receive_inner();
        if result.is_err() || matches!(result, Ok(Event::Reset | Event::Cancel)) {
            self.control.stop();
        }
        result
    }
    pub(crate) fn receive_frame(
        &mut self,
        until: Instant,
    ) -> Result<Option<spiralcore_transfer::protocol::Frame>> {
        match self.receive_object(until)? {
            crate::ObjectRecord::Frame(_, frame) => Ok(Some(frame)),
            crate::ObjectRecord::TransportFin => Ok(None),
            crate::ObjectRecord::ApplicationHalfClose => {
                self.control.stop();
                Err(Error::Protocol)
            }
        }
    }
    pub(crate) fn poll_input(&mut self) -> Result<bool> {
        self.control.check()?;
        self.socket
            .set_read_timeout(Some(Duration::from_millis(20)))?;
        match self.socket.peek(&mut [0; 1]) {
            Ok(0) => Err(Error::Truncated),
            Ok(_) => Ok(true),
            Err(error) if transient(&error) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
    pub(crate) fn receive_object(&mut self, until: Instant) -> Result<crate::ObjectRecord> {
        if self.ended {
            return Err(Error::Closed);
        }
        let result = (|| {
            let next = self.sequence.checked_add(1).ok_or(Error::Sequence)?;
            let mut header = [0; HEADER_LEN];
            self.fill(&mut header, until)?;
            let (kind, len) = record::parse_header(
                &header,
                self.binding,
                self.sequence,
                self.cap,
                record::OBJECT,
                self.role,
            )?;
            let frame = if matches!(kind, record::OBJECT | record::ARTIFACT) {
                let mut body = vec![0; len];
                self.fill(&mut body, until)?;
                let frame = spiralcore_transfer::protocol::Frame::decode(&body)?;
                poll_budget(until)?;
                self.counters.complete(body.len(), false)?;
                crate::ObjectRecord::Frame(
                    if kind == record::OBJECT {
                        crate::Purpose::Application
                    } else {
                        crate::Purpose::Artifact
                    },
                    frame,
                )
            } else if kind == record::APP_FIN {
                self.counters.complete(0, false)?;
                crate::ObjectRecord::ApplicationHalfClose
            } else if kind == record::FIN {
                self.counters.complete(0, false)?;
                self.ended = true;
                crate::ObjectRecord::TransportFin
            } else {
                return Err(Error::Cancelled);
            };
            self.sequence = next;
            Ok(frame)
        })();
        if result.is_err() {
            self.control.stop();
        }
        result
    }
}
impl Drop for Rx {
    fn drop(&mut self) {
        if !self.ended {
            self.control.stop_on_drop();
        }
    }
}

#[cfg(test)]
mod timeout_tests {
    use super::*;
    fn rejected() -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(22))
    }
    #[test]
    fn closed_peer_timeout_exception_requires_observed_nonzero_tighter_deadline() {
        let requested = Some(Duration::from_millis(20));
        assert_eq!(
            checked_timeout(rejected(), requested, || Ok(Some(Duration::from_millis(
                20
            ))))
            .is_ok(),
            cfg!(target_os = "macos")
        );
        assert_eq!(
            checked_timeout(rejected(), requested, || Ok(Some(Duration::from_millis(1)))).is_ok(),
            cfg!(target_os = "macos")
        );
        for existing in [None, Some(Duration::ZERO), Some(Duration::from_millis(21))] {
            assert!(checked_timeout(rejected(), requested, || Ok(existing)).is_err());
        }
        assert!(checked_timeout(rejected(), None, || Ok(Some(Duration::from_millis(1)))).is_err());
        assert!(
            checked_timeout(rejected(), requested, || Err(std::io::Error::other(
                "readback failed"
            )))
            .is_err()
        );
        let other = Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "not EINVAL",
        ));
        assert!(checked_timeout(other, requested, || Ok(Some(Duration::from_millis(1)))).is_err());
    }
}
