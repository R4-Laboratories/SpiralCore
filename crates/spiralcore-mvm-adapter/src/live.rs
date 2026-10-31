//! Bounded socket transport and authenticated call/reply proofs.
//!
//! Requires the retained MVM verified-open/transport-accessor patch. Framing
//! helpers carry ciphertext only; the relay must independently admit its route.
use super::*;
use mvm_core::mailbox::transport::{AuthorizedMailboxFrame, PeerChannel, TransportError};
use std::{
    io::{self, Read, Write},
    net::{Shutdown, TcpStream},
    sync::{
        Arc, Mutex, MutexGuard, TryLockError,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DISCRIMINATOR: &[u8; 8] = b"SCMVS001";
const MAX_OPERATION: Duration = Duration::from_secs(30);
const IO_SLICE: Duration = Duration::from_millis(50);
/// The live harness must establish this maximum pairwise UTC uncertainty.
pub const MAX_CLOCK_OFFSET_MS: i64 = 5_000;
pub type LiveResult<T> = std::result::Result<T, LiveError>;

#[derive(Debug)]
pub enum LiveError {
    Adapter(Error),
    Io(io::Error),
    Deadline,
    Cancelled,
    Framing,
    Closed,
    Route,
    Policy,
    CallPending,
    RateLimited,
    State,
}
impl std::fmt::Display for LiveError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{self:?}")
    }
}
impl std::error::Error for LiveError {}
impl From<Error> for LiveError {
    fn from(value: Error) -> Self {
        Self::Adapter(value)
    }
}
impl From<io::Error> for LiveError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// An owned blocking socket with checked timeout and shutdown support.
pub trait SocketIo: Read + Write + Send {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()>;
    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()>;
    fn shutdown(&self, how: Shutdown) -> io::Result<()>;
}
macro_rules! socket_io {
    ($socket:ty) => {
        impl SocketIo for $socket {
            fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
                checked_timeout(<$socket>::set_read_timeout(self, timeout), timeout, || {
                    <$socket>::read_timeout(self)
                })
            }
            fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
                checked_timeout(<$socket>::set_write_timeout(self, timeout), timeout, || {
                    <$socket>::write_timeout(self)
                })
            }
            fn shutdown(&self, how: Shutdown) -> io::Result<()> {
                <$socket>::shutdown(self, how)
            }
        }
    };
}
socket_io!(TcpStream);
#[cfg(unix)]
socket_io!(std::os::unix::net::UnixStream);

fn checked_timeout(
    result: io::Result<()>,
    requested: Option<Duration>,
    existing: impl FnOnce() -> io::Result<Option<Duration>>,
) -> io::Result<()> {
    // Darwin rejects SO_RCVTIMEO/SO_SNDTIMEO changes after peer close even
    // while a complete response remains buffered. Keep only a read-back,
    // nonzero timeout that is at least as strict as this operation requested.
    if cfg!(target_os = "macos")
        && result
            .as_ref()
            .is_err_and(|error| error.kind() == io::ErrorKind::InvalidInput)
        && let (Some(requested), Ok(Some(actual))) = (requested, existing())
        && !actual.is_zero()
        && actual <= requested
    {
        return Ok(());
    }
    result
}

#[derive(Clone, Copy, Debug)]
pub enum HandshakeRole {
    Initiator,
    Acceptor,
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct StreamCounters {
    pub envelope_bytes_sent: u64,
    pub envelope_bytes_received: u64,
    pub framing_bytes_sent: u64,
    pub framing_bytes_received: u64,
    pub frames_sent: u64,
    pub frames_received: u64,
}

fn increment(counter: &mut u64, bytes: usize) -> LiveResult<()> {
    *counter = counter.checked_add(bytes as u64).ok_or(LiveError::State)?;
    Ok(())
}
fn remaining(cancel: &CancellationToken, deadline: Instant) -> LiveResult<Duration> {
    if cancel.is_cancelled() {
        return Err(LiveError::Cancelled);
    }
    let duration = deadline
        .checked_duration_since(Instant::now())
        .ok_or(LiveError::Deadline)?;
    if duration.is_zero() {
        return Err(LiveError::Deadline);
    }
    Ok(duration.min(IO_SLICE))
}
fn bounded_deadline(deadline: Instant) -> Instant {
    deadline.min(Instant::now() + MAX_OPERATION)
}
fn close_on_error<S: SocketIo, T>(socket: &S, result: LiveResult<T>) -> LiveResult<T> {
    if result.is_err() {
        let _ = socket.shutdown(Shutdown::Both);
    }
    result
}
fn read_exact<S: SocketIo>(
    socket: &mut S,
    bytes: &mut [u8],
    cancel: &CancellationToken,
    deadline: Instant,
    counter: &mut u64,
) -> LiveResult<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        socket
            .set_read_timeout(Some(remaining(cancel, deadline)?))
            .map_err(|error| io::Error::new(error.kind(), format!("set_read_timeout: {error}")))?;
        match socket.read(&mut bytes[offset..]) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
            Ok(count) => {
                increment(counter, count)?;
                offset += count;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => {
                return Err(io::Error::new(error.kind(), format!("socket read: {error}")).into());
            }
        }
    }
    remaining(cancel, deadline)?;
    Ok(())
}
fn write_all<S: SocketIo>(
    socket: &mut S,
    bytes: &[u8],
    cancel: &CancellationToken,
    deadline: Instant,
    counter: &mut u64,
) -> LiveResult<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        socket.set_write_timeout(Some(remaining(cancel, deadline)?))?;
        match socket.write(&bytes[offset..]) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero).into()),
            Ok(count) => {
                increment(counter, count)?;
                offset += count;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
    remaining(cancel, deadline)?;
    Ok(())
}

/// Version/readiness exchange. This is not authentication.
pub fn handshake_stream<S: SocketIo>(
    socket: &mut S,
    role: HandshakeRole,
    cancel: &CancellationToken,
    deadline: Instant,
    counters: &mut StreamCounters,
) -> LiveResult<()> {
    let deadline = bounded_deadline(deadline);
    let result = (|| {
        if matches!(role, HandshakeRole::Initiator) {
            write_all(
                socket,
                DISCRIMINATOR,
                cancel,
                deadline,
                &mut counters.framing_bytes_sent,
            )?;
        }
        let mut peer = [0; 8];
        read_exact(
            socket,
            &mut peer,
            cancel,
            deadline,
            &mut counters.framing_bytes_received,
        )?;
        if peer != *DISCRIMINATOR {
            return Err(LiveError::Framing);
        }
        if matches!(role, HandshakeRole::Acceptor) {
            write_all(
                socket,
                DISCRIMINATOR,
                cancel,
                deadline,
                &mut counters.framing_bytes_sent,
            )?;
        }
        Ok(())
    })();
    close_on_error(socket, result)
}

/// Read one bounded ciphertext frame; the recipient still must authenticate it.
pub fn read_ciphertext_frame<S: SocketIo>(
    socket: &mut S,
    cancel: &CancellationToken,
    deadline: Instant,
    counters: &mut StreamCounters,
) -> LiveResult<Vec<u8>> {
    let deadline = bounded_deadline(deadline);
    let result = (|| {
        let mut prefix = [0; 4];
        read_exact(
            socket,
            &mut prefix,
            cancel,
            deadline,
            &mut counters.framing_bytes_received,
        )?;
        let len = u32::from_be_bytes(prefix) as usize;
        if len == 0 || len > MAX_ENVELOPE_BYTES {
            return Err(LiveError::Framing);
        }
        let mut bytes = vec![0; len];
        read_exact(
            socket,
            &mut bytes,
            cancel,
            deadline,
            &mut counters.envelope_bytes_received,
        )?;
        increment(&mut counters.frames_received, 1)?;
        Ok(bytes)
    })();
    close_on_error(socket, result)
}

/// Relay-only byte forwarding. This does not construct an authorized frame.
/// Admit/validate the canonical outer route before calling this helper.
pub fn write_ciphertext_frame<S: SocketIo>(
    socket: &mut S,
    bytes: &[u8],
    cancel: &CancellationToken,
    deadline: Instant,
    counters: &mut StreamCounters,
) -> LiveResult<()> {
    let deadline = bounded_deadline(deadline);
    let result = (|| {
        if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES {
            return Err(LiveError::Framing);
        }
        write_all(
            socket,
            &(bytes.len() as u32).to_be_bytes(),
            cancel,
            deadline,
            &mut counters.framing_bytes_sent,
        )?;
        write_all(
            socket,
            bytes,
            cancel,
            deadline,
            &mut counters.envelope_bytes_sent,
        )?;
        increment(&mut counters.frames_sent, 1)
    })();
    close_on_error(socket, result)
}

struct StreamState<S> {
    socket: S,
    counters: StreamCounters,
    ready: bool,
    closed: bool,
}
/// One preadmitted route and one ordered writer; no pending message queue.
pub struct StreamTransport<S: SocketIo> {
    state: Arc<Mutex<StreamState<S>>>,
    route: RouteId,
    timeout: Duration,
}
impl<S: SocketIo + 'static> StreamTransport<S> {
    pub fn new(socket: S, allowed_route: RouteId, timeout: Duration) -> LiveResult<Self> {
        if timeout.is_zero() || timeout > MAX_OPERATION {
            return Err(LiveError::Deadline);
        }
        // Fail construction if this socket cannot support bounded operations.
        socket.set_read_timeout(Some(timeout.min(IO_SLICE)))?;
        socket.set_write_timeout(Some(timeout.min(IO_SLICE)))?;
        Ok(Self {
            state: Arc::new(Mutex::new(StreamState {
                socket,
                counters: StreamCounters::default(),
                ready: false,
                closed: false,
            })),
            route: allowed_route,
            timeout,
        })
    }
    pub fn handshake(&self, role: HandshakeRole, cancel: &CancellationToken) -> LiveResult<()> {
        let deadline = Instant::now() + self.timeout;
        let mut state = lock(&self.state, cancel, deadline)?;
        if state.closed || state.ready {
            return Err(LiveError::Closed);
        }
        let StreamState {
            socket, counters, ..
        } = &mut *state;
        let result = handshake_stream(socket, role, cancel, deadline, counters);
        state.closed = result.is_err();
        state.ready = result.is_ok();
        result
    }
    pub fn receive(&self, cancel: &CancellationToken, deadline: Instant) -> LiveResult<Vec<u8>> {
        let deadline = deadline.min(Instant::now() + self.timeout);
        let mut state = lock(&self.state, cancel, deadline)?;
        if state.closed || !state.ready {
            return Err(LiveError::Closed);
        }
        let StreamState {
            socket, counters, ..
        } = &mut *state;
        let result = read_ciphertext_frame(socket, cancel, deadline, counters);
        state.closed = result.is_err();
        result
    }
    pub fn counters(&self) -> LiveResult<StreamCounters> {
        let state = lock(
            &self.state,
            &CancellationToken::default(),
            Instant::now() + self.timeout,
        )?;
        Ok(state.counters)
    }
    pub fn close(&self) -> LiveResult<()> {
        let mut state = lock(
            &self.state,
            &CancellationToken::default(),
            Instant::now() + self.timeout,
        )?;
        state.closed = true;
        state.socket.shutdown(Shutdown::Both)?;
        Ok(())
    }
    fn before(&self, deadline: Instant) -> DeadlineTransport<S> {
        DeadlineTransport {
            state: Arc::clone(&self.state),
            route: self.route,
            deadline: deadline.min(Instant::now() + self.timeout),
        }
    }
}
fn lock<'a, S: SocketIo>(
    state: &'a Mutex<StreamState<S>>,
    cancel: &CancellationToken,
    deadline: Instant,
) -> LiveResult<MutexGuard<'a, StreamState<S>>> {
    loop {
        match state.try_lock() {
            Ok(mut guard) => {
                if let Err(error) = remaining(cancel, deadline) {
                    guard.closed = true;
                    let _ = guard.socket.shutdown(Shutdown::Both);
                    return Err(error);
                }
                return Ok(guard);
            }
            Err(TryLockError::Poisoned(error)) => {
                let _ = error.into_inner().socket.shutdown(Shutdown::Both);
                return Err(LiveError::State);
            }
            Err(TryLockError::WouldBlock) => {
                remaining(cancel, deadline)?;
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
struct DeadlineTransport<S: SocketIo> {
    state: Arc<Mutex<StreamState<S>>>,
    route: RouteId,
    deadline: Instant,
}
struct SocketChannel<S: SocketIo> {
    state: Arc<Mutex<StreamState<S>>>,
    deadline: Instant,
    max_frame: usize,
}
impl<S: SocketIo + 'static> PeerTransport for StreamTransport<S> {
    fn connect(
        &self,
        route: &AuthorizedRoute,
        limits: ChannelLimits,
        cancel: &CancellationToken,
    ) -> std::result::Result<Box<dyn PeerChannel>, TransportError> {
        self.before(Instant::now() + self.timeout)
            .connect(route, limits, cancel)
    }
}
impl<S: SocketIo + 'static> PeerTransport for DeadlineTransport<S> {
    fn connect(
        &self,
        route: &AuthorizedRoute,
        limits: ChannelLimits,
        cancel: &CancellationToken,
    ) -> std::result::Result<Box<dyn PeerChannel>, TransportError> {
        if route.route_id() != self.route {
            return Err(TransportError::UnknownRoute);
        }
        let state = lock(&self.state, cancel, self.deadline).map_err(transport_error)?;
        if state.closed || !state.ready {
            return Err(TransportError::ShuttingDown);
        }
        if limits.max_frame_bytes() > MAX_ENVELOPE_BYTES
            || limits.max_pending_bytes() < limits.max_frame_bytes()
        {
            return Err(TransportError::InvalidLimits);
        }
        Ok(Box::new(SocketChannel {
            state: Arc::clone(&self.state),
            deadline: self.deadline,
            max_frame: limits.max_frame_bytes(),
        }))
    }
}
impl<S: SocketIo> PeerChannel for SocketChannel<S> {
    fn try_send(
        &mut self,
        frame: &AuthorizedMailboxFrame,
        cancel: &CancellationToken,
    ) -> std::result::Result<(), TransportError> {
        if frame.encoded_len() > self.max_frame {
            return Err(TransportError::FrameTooLarge);
        }
        let mut state = lock(&self.state, cancel, self.deadline).map_err(transport_error)?;
        if state.closed || !state.ready {
            return Err(TransportError::ShuttingDown);
        }
        let StreamState {
            socket, counters, ..
        } = &mut *state;
        let result = write_ciphertext_frame(
            socket,
            frame.encoded_bytes(),
            cancel,
            self.deadline,
            counters,
        );
        state.closed = result.is_err();
        result.map_err(transport_error)
    }
}
fn transport_error(error: LiveError) -> TransportError {
    match error {
        LiveError::Cancelled => TransportError::Cancelled,
        LiveError::Closed => TransportError::ShuttingDown,
        LiveError::Framing => TransportError::FrameTooLarge,
        LiveError::Route => TransportError::UnknownRoute,
        LiveError::State => TransportError::StateUnavailable,
        _ => TransportError::Partitioned,
    }
}

pub(super) fn unix_ms() -> LiveResult<i64> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| LiveError::State)?
            .as_millis(),
    )
    .map_err(|_| LiveError::State)
}
pub(super) fn expiry_deadline(expiry: i64) -> LiveResult<Instant> {
    let now = unix_ms()?;
    let remaining = expiry
        .checked_sub(MAX_CLOCK_OFFSET_MS)
        .and_then(|guarded| guarded.checked_sub(now))
        .ok_or(LiveError::Deadline)?;
    if remaining <= 0 {
        return Err(LiveError::Deadline);
    }
    Ok(Instant::now() + Duration::from_millis(remaining as u64).min(MAX_OPERATION))
}
pub(super) struct DirectionState {
    outstanding: Arc<AtomicBool>,
    window: Instant,
    sent: u32,
    received: u32,
}
impl Default for DirectionState {
    fn default() -> Self {
        Self {
            outstanding: Arc::new(AtomicBool::new(false)),
            window: Instant::now(),
            sent: 0,
            received: 0,
        }
    }
}
impl DirectionState {
    pub(super) fn admit(&mut self, sent: bool, limit: u32) -> LiveResult<()> {
        if self.window.elapsed() >= Duration::from_secs(60) {
            self.window = Instant::now();
            self.sent = 0;
            self.received = 0;
        }
        let count = if sent {
            &mut self.sent
        } else {
            &mut self.received
        };
        if *count >= limit {
            return Err(LiveError::RateLimited);
        }
        *count += 1;
        Ok(())
    }
}
struct CallPermit(Arc<AtomicBool>);
impl Drop for CallPermit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
/// A single outstanding, locally authorized call. Not Clone and not constructible.
pub struct SentCall {
    metadata: SentMetadata,
    context: Context,
    request_kind: u8,
    transfer: [u8; 32],
    seq: u64,
    sender_policy: Policy,
    sender_generation: u64,
    receiver_policy: Policy,
    receiver_generation: u64,
    receiver_context: Context,
    sender_key: [u8; 32],
    authority_key: [u8; 32],
    key_id: [u8; 32],
    binding_id: TransportBindingId,
    boot: BootId,
    deadline: Instant,
    expiry: i64,
    _permit: CallPermit,
}
impl SentCall {
    pub fn deadline(&self) -> Instant {
        self.deadline
    }
    /// Namespace from canonical outbound authorization, not receiver authority.
    pub fn context(&self) -> &Context {
        &self.context
    }
}

fn reply_kind(request: u8, response: u8) -> bool {
    response == 127
        || match request {
            1 | 4 => matches!(response, 101 | 103),
            2 => response == 102,
            3 => matches!(response, 103 | 104),
            5 | 6 => response == 105,
            _ => false,
        }
}
fn address(context: &OpenContext) -> MailboxAddress {
    MailboxAddress {
        tenant: context.expected_tenant,
        authorization_domain: context.expected_authorization_domain,
        mailbox: context.expected_mailbox,
        recipient: context.expected_recipient,
    }
}
pub(super) fn check_verified(frame: &VerifiedFrame) -> LiveResult<()> {
    frame.policy.check_generation(frame.generation)?;
    if Instant::now() >= frame.live_deadline.ok_or(LiveError::Policy)?
        || unix_ms()? >= frame.live_expiry_unix_ms.ok_or(LiveError::Policy)?
    {
        return Err(LiveError::Deadline);
    }
    Ok(())
}

impl VerifiedFrame {
    /// The live service owner serializes this operation with policy updates.
    /// Borrowing retains the authenticated header needed for the safe reply.
    pub fn dispatch_live(
        &self,
        endpoint: &mut Endpoint<MailboxAuthority>,
        monotonic_seconds: u64,
    ) -> LiveResult<Frame> {
        check_verified(self)?;
        endpoint
            .handle(&self.context, &self.frame, monotonic_seconds)
            .map_err(|error| LiveError::Adapter(Error::Frame(error)))
    }
}

impl Direction {
    /// Refresh wall time independently of policy generation, then authenticate.
    pub fn open_live(&mut self, wire: &[u8]) -> LiveResult<VerifiedFrame> {
        self.config.policy.refresh_time(unix_ms()?)?;
        self.open_inner(wire, true).map_err(LiveError::Adapter)
    }

    pub fn send_call<S: SocketIo + 'static>(
        &mut self,
        frame: &Frame,
        replies: &Direction,
        transport: &StreamTransport<S>,
        cancel: &CancellationToken,
        deadline: Instant,
    ) -> LiveResult<SentCall> {
        if self.config.kind != MailboxMessageKind::Call
            || replies.config.kind != MailboxMessageKind::Reply
        {
            return Err(LiveError::Policy);
        }
        let now = unix_ms()?;
        self.config.policy.refresh_time(now)?;
        replies.config.policy.refresh_time(now)?;
        let (context, generation) = self.config.policy.versioned()?;
        let (reverse, reverse_generation) = replies.config.policy.versioned()?;
        let expected_route = ReplyRoute {
            address: address(&reverse),
            capability_id: replies.config.capability.grant.capability_id,
        };
        if self.config.reply_to != Some(expected_route)
            || context.expected_tenant != reverse.expected_tenant
            || context.expected_authorization_domain != reverse.expected_authorization_domain
            || context.expected_sender != reverse.expected_recipient
            || context.expected_recipient != reverse.expected_sender
        {
            return Err(LiveError::Policy);
        }
        self.route(&context)?;
        replies.route(&reverse)?;
        let budget = deadline
            .checked_duration_since(Instant::now())
            .ok_or(LiveError::Deadline)?;
        let requested_expiry = now
            .checked_add(i64::try_from(budget.as_millis()).map_err(|_| LiveError::Deadline)?)
            .ok_or(LiveError::Deadline)?;
        let expiry = self
            .config
            .capability
            .grant
            .expires_at_unix_ms
            .min(self.config.binding.claims.expires_at_unix_ms)
            .min(replies.config.capability.grant.expires_at_unix_ms)
            .min(replies.config.binding.claims.expires_at_unix_ms)
            .min(requested_expiry)
            .min(now.checked_add(30_000).ok_or(LiveError::Deadline)?);
        let deadline = deadline.min(expiry_deadline(expiry)?);
        remaining(cancel, deadline)?;
        self.live_state
            .outstanding
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| LiveError::CallPending)?;
        let permit = CallPermit(Arc::clone(&self.live_state.outstanding));
        self.live_state
            .admit(true, self.config.capability.grant.max_messages_per_minute)?;
        self.send_correlated_lifetime(
            frame,
            &transport.before(deadline),
            cancel,
            None,
            Some((MAX_CLOCK_OFFSET_MS, expiry)),
        )?;
        let metadata = self.last_sent.ok_or(LiveError::State)?;
        Ok(SentCall {
            metadata,
            context: self.verified_context(&context)?,
            request_kind: frame.kind,
            transfer: frame.transfer,
            seq: frame.seq,
            sender_policy: self.config.policy.clone(),
            sender_generation: generation,
            receiver_policy: replies.config.policy.clone(),
            receiver_generation: reverse_generation,
            receiver_context: replies.verified_context(&reverse)?,
            sender_key: replies.config.sender_verifying_key.to_bytes(),
            authority_key: replies.config.authority_key.to_bytes(),
            key_id: replies.config.key_id,
            binding_id: replies.config.binding.claims.binding_id,
            boot: replies.config.boot,
            deadline,
            expiry: expiry
                .checked_sub(MAX_CLOCK_OFFSET_MS)
                .ok_or(LiveError::Deadline)?,
            _permit: permit,
        })
    }

    /// Derive reply routing/correlation only from a verified live inbound call.
    pub fn reply_to_verified<S: SocketIo + 'static>(
        &mut self,
        request: &VerifiedFrame,
        reply: &Frame,
        transport: &StreamTransport<S>,
        cancel: &CancellationToken,
    ) -> LiveResult<usize> {
        check_verified(request)?;
        self.config.policy.refresh_time(unix_ms()?)?;
        let context = self.config.policy.snapshot()?;
        let header = &request.header;
        let route = ReplyRoute {
            address: address(&context),
            capability_id: self.config.capability.grant.capability_id,
        };
        if self.config.kind != MailboxMessageKind::Reply
            || header.kind != MailboxMessageKind::Call
            || header.reply_to != Some(route)
            || header.tenant != context.expected_tenant
            || header.authorization_domain != context.expected_authorization_domain
            || header.sender != context.expected_recipient
            || header.recipient != context.expected_sender
            || header.ordering_scope
                != OrderingScope::parse("spiralcore-object-v1").map_err(|_| LiveError::Policy)?
            || reply.transfer != request.frame.transfer
            || reply.seq != request.frame.seq
            || !reply_kind(request.frame.kind, reply.kind)
        {
            return Err(LiveError::Policy);
        }
        let expiry = header
            .expires_at_unix_ms
            .min(self.config.capability.grant.expires_at_unix_ms)
            .min(self.config.binding.claims.expires_at_unix_ms);
        let deadline = request
            .live_deadline
            .ok_or(LiveError::Policy)?
            .min(expiry_deadline(expiry)?);
        self.live_state
            .admit(true, self.config.capability.grant.max_messages_per_minute)?;
        self.send_correlated_lifetime(
            reply,
            &transport.before(deadline),
            cancel,
            Some(SentMetadata {
                message_id: header.message_id,
                correlation_id: header.correlation_id,
            }),
            Some((MAX_CLOCK_OFFSET_MS, expiry)),
        )
        .map_err(LiveError::Adapter)
    }

    /// Consume the pending call even when the supplied reply fails validation.
    pub fn accept_reply(&self, call: SentCall, reply: VerifiedFrame) -> LiveResult<Frame> {
        check_verified(&reply)?;
        call.sender_policy
            .check_generation(call.sender_generation)?;
        call.receiver_policy
            .check_generation(call.receiver_generation)?;
        if Instant::now() >= call.deadline || unix_ms()? >= call.expiry {
            return Err(LiveError::Deadline);
        }
        let header = &reply.header;
        if self.config.kind != MailboxMessageKind::Reply
            || !Arc::ptr_eq(&self.config.policy.state, &call.receiver_policy.state)
            || !Arc::ptr_eq(&reply.policy.state, &call.receiver_policy.state)
            || reply.context != call.receiver_context
            || reply.sender_key != call.sender_key
            || reply.authority_key != call.authority_key
            || header.encryption.key_id != call.key_id
            || reply.binding_id != call.binding_id
            || header.sender_boot != call.boot
            || header.kind != MailboxMessageKind::Reply
            || header.reply_to.is_some()
            || header.correlation_id != call.metadata.correlation_id
            || header.causation_id.as_bytes() != call.metadata.message_id.as_bytes()
            || header.ordering_scope
                != OrderingScope::parse("spiralcore-object-v1").map_err(|_| LiveError::Policy)?
            || reply.frame.transfer != call.transfer
            || reply.frame.seq != call.seq
            || !reply_kind(call.request_kind, reply.frame.kind)
        {
            return Err(LiveError::Policy);
        }
        Ok(reply.frame)
    }
}
