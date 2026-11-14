use crate::FailureSite;
pub(crate) mod routing;
#[cfg(feature = "witness-timing")]
use crate::timing::{FrameKind, Metric, Observation, Phase, TimingObserver};
use crate::{
    Binding, Cancellation, Counters, Error, Event, MAX_BLOCK, ObjectChannel, ObjectRecord, Purpose,
    Result, Rx, Tx,
};
use routing::Routing;
use spiralcore_transfer::{
    Id,
    client::Stats,
    endpoint::{Authority, Context},
    protocol::{Frame, Offer},
    v1::{
        ObjectEndpoint, ObjectReceipt, ObjectSender, OrderedBinding, OrderedLane,
        OrderedObjectEndpoint, OrderedSnapshot, ordered_transfer_id,
    },
};
use std::{
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
/// One owned reader's explicit stack, charged separately from core workspace.
pub const READER_STACK_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Initiator,
    Acceptor,
}
impl Role {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::Initiator => 0,
            Self::Acceptor => 1,
        }
    }
    pub(crate) fn remote(self) -> Self {
        match self {
            Self::Initiator => Self::Acceptor,
            Self::Acceptor => Self::Initiator,
        }
    }
}
fn transfer(binding: Binding, role: Role, index: u64, artifact: bool) -> Id {
    ordered_transfer_id(
        if artifact {
            b"spiralcore.flow.artifact.v1\0"
        } else {
            b"spiralcore.flow.application.v1\0"
        },
        &binding.flow,
        role.tag(),
        index,
    )
}
fn lock<T>(value: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    value.lock().map_err(|_| Error::Worker)
}
pub(crate) fn lock_until<T>(value: &Mutex<T>, until: Instant) -> Result<MutexGuard<'_, T>> {
    loop {
        let left = remaining(until)?;
        match value.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(std::sync::TryLockError::Poisoned(_)) => return Err(Error::Worker),
            Err(std::sync::TryLockError::WouldBlock) => {
                thread::park_timeout(left.min(Duration::from_millis(1)))
            }
        }
    }
}
pub(crate) fn deadline(timeout: Duration) -> Result<Instant> {
    Instant::now().checked_add(timeout).ok_or(Error::Limit)
}
fn remaining(until: Instant) -> Result<Duration> {
    let left = until.saturating_duration_since(Instant::now());
    if left.is_zero() {
        Err(Error::Timeout)
    } else {
        Ok(left)
    }
}
fn now() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Protocol)?
        .as_secs())
}
#[cfg(feature = "witness-stall")]
pub mod stall;
#[cfg(feature = "witness-stall")]
use stall::{SenderPhase, StallObserver, WorkerPhase};

struct Expected {
    #[cfg(feature = "witness-stall")]
    kind: u8,
    purpose: Purpose,
    transfer: Id,
    seq: u64,
    until: Instant,
    seen: Arc<AtomicUsize>,
}
type Pending = Arc<Mutex<Option<Expected>>>;
type SharedTx = Arc<crate::lock_wait::ReleaseMutex<Tx>>;
fn acquire_tx<'a>(
    tx: &'a SharedTx,
    until: Instant,
    cancellation: &Cancellation,
) -> Result<crate::lock_wait::Guard<'a, Tx>> {
    cancellation.check()?;
    let guard = tx
        .acquire(until, cancellation.stopped())
        .map_err(|error| match error {
            crate::lock_wait::AcquireError::Timeout => Error::Timeout,
            crate::lock_wait::AcquireError::Poisoned => Error::Worker,
            crate::lock_wait::AcquireError::Stopped => {
                cancellation.check().err().unwrap_or(Error::Closed)
            }
        })?;
    cancellation.check()?;
    remaining(until)?;
    Ok(guard)
}

macro_rules! tx_lock {
    ($tx:expr, $until:expr, $cancellation:expr, $timing:expr, $phase:ident, $kind:expr) => {{
        #[cfg(feature = "witness-timing")]
        let _span = $timing.span(Metric::Frame(Phase::$phase, $kind));
        acquire_tx($tx, $until, $cancellation)
    }};
}

#[derive(Debug)]
pub struct BlockReceipt {
    pub index: u64,
    pub transfer: Id,
    pub object: ObjectReceipt,
}
pub struct Duplex {
    writer: DuplexWriter,
    reader: DuplexReader,
    guard: JoinGuard,
    retention: RetentionObserver,
}
/// Copied diagnostics only; it owns no endpoint, file, socket or resource lease.
#[derive(Clone, Default)]
pub struct RetentionObserver {
    inner: Arc<Mutex<Option<OrderedSnapshot>>>,
}
impl RetentionObserver {
    /// A concurrent diagnostic observation; never waits for the publisher.
    #[cfg(feature = "witness-timing")]
    pub fn try_snapshot(&self) -> Result<Option<OrderedSnapshot>> {
        match self.inner.try_lock() {
            Ok(snapshot) => Ok(*snapshot),
            Err(std::sync::TryLockError::WouldBlock) => Err(Error::Timeout),
            Err(std::sync::TryLockError::Poisoned(_)) => Err(Error::Worker),
        }
    }
    pub fn snapshot(&self) -> Result<Option<OrderedSnapshot>> {
        Ok(*lock(&self.inner)?)
    }
    fn publish(&self, snapshot: Option<OrderedSnapshot>) -> Result<()> {
        *lock(&self.inner)? = snapshot;
        Ok(())
    }
}
impl Duplex {
    /// Bounded diagnostics only in the opt-in stall witness.
    #[cfg(feature = "witness-stall")]
    pub fn stall_observer(&self) -> StallObserver {
        self.writer.stall.clone()
    }

    /// Fixed additional TX wrapper/control state; excludes original TX and timing-owner heap.
    #[cfg(feature = "witness-timing")]
    pub const fn ownership_wait_bytes() -> usize {
        crate::lock_wait::ReleaseMutex::<Tx>::additional_owned_bytes()
            + Cancellation::wake_binding_bytes()
    }
    #[cfg(feature = "witness-timing")]
    pub fn timing_observer(&self) -> TimingObserver {
        self.writer.timing.clone()
    }
    pub fn retention_observer(&self) -> RetentionObserver {
        self.retention.clone()
    }
    pub(crate) fn stream_config(&self) -> (Cancellation, Duration) {
        (self.writer.cancellation.clone(), self.writer.timeout)
    }
    pub fn spawn<A: Authority + Send + 'static>(
        mut channel: ObjectChannel,
        sender: ObjectSender,
        endpoint: ObjectEndpoint<A>,
        context: Context,
        role: Role,
    ) -> Result<Self> {
        channel.check_context(&context)?;
        if channel
            .authenticated_role
            .is_some_and(|authenticated| authenticated != role)
        {
            return Err(Error::InvalidBinding);
        }
        channel.tx.role = role.tag();
        channel.rx.role = role.remote().tag();
        if sender.profile() != channel.profile() || endpoint.profile() != channel.profile() {
            return Err(Error::Protocol);
        }
        let cancellation = channel.cancellation();
        let timeout = channel.timeout;
        let binding = channel.binding;
        let endpoint = if endpoint.profile().capabilities().profile == 2 {
            let ordered = OrderedBinding::new(
                &context,
                binding.flow,
                role.remote().tag(),
                b"spiralcore.flow.application.v1\0",
                b"spiralcore.flow.artifact.v1\0",
            )?;
            ReceiveEndpoint::Ordered(Box::new(OrderedObjectEndpoint::new(
                endpoint, ordered, &context,
            )?))
        } else {
            ReceiveEndpoint::General(Box::new(endpoint))
        };
        #[cfg(feature = "witness-timing")]
        let timing = TimingObserver::default();
        let shared_tx = crate::lock_wait::ReleaseMutex::new(channel.tx);
        #[cfg(feature = "witness-timing")]
        let shared_tx = shared_tx.observed(timing.clone(), crate::timing::LockKind::Tx);
        let shared_tx = Arc::new(shared_tx);
        cancellation.bind_tx_wake(shared_tx.gate())?;
        let pending = Arc::new(Mutex::new(None));
        let closed = Arc::new(AtomicBool::new(false));
        let (responses, response_rx) = mpsc::sync_channel(1);
        let routing = Routing::new(channel.rx, pending.clone(), responses, &cancellation)?;
        let (delivery, delivery_rx) = mpsc::sync_channel(4);
        let (done, done_rx) = mpsc::sync_channel(1);
        let retention = RetentionObserver::default();
        #[cfg(feature = "witness-stall")]
        let stall = StallObserver::new(pending.clone());
        let worker = Worker {
            #[cfg(feature = "witness-stall")]
            stall: stall.clone(),
            #[cfg(feature = "witness-timing")]
            timing: timing.clone(),
            completed: false,
            routing: routing.clone(),
            tx: shared_tx.clone(),
            pending: pending.clone(),
            delivery,
            endpoint,
            context: context.clone(),
            binding,
            role: role.remote(),
            index: 0,
            artifact_index: 0,
            active: None,
            remote_closed: false,
            local_closed: closed.clone(),
            cancellation: cancellation.clone(),
            timeout,
            retention: retention.clone(),
        };
        let child = thread::Builder::new()
            .stack_size(READER_STACK_BYTES)
            .name("spiral-flow-reader".into())
            .spawn(move || {
                let result = worker.run();
                let _ = done.send(());
                result
            })?;
        Ok(Self {
            writer: DuplexWriter {
                #[cfg(feature = "witness-stall")]
                stall,
                #[cfg(feature = "witness-timing")]
                timing,
                tx: shared_tx,
                routing,
                pending,
                responses: response_rx,
                sender,
                context,
                binding,
                role,
                index: 0,
                artifact_index: 0,
                closed,
                cancellation: cancellation.clone(),
                timeout,
            },
            reader: DuplexReader {
                delivery: delivery_rx,
                ended: false,
                cancellation: cancellation.clone(),
                timeout,
            },
            guard: JoinGuard {
                child: Some(child),
                done: done_rx,
                done_seen: false,
                joined: None,
                panicked: false,
                cancellation,
                timeout,
            },
            retention,
        })
    }
    pub fn split(self) -> (DuplexWriter, DuplexReader, JoinGuard) {
        (self.writer, self.reader, self.guard)
    }
}

pub struct DuplexWriter {
    routing: Arc<Routing>,
    #[cfg(feature = "witness-stall")]
    stall: StallObserver,
    #[cfg(feature = "witness-timing")]
    timing: TimingObserver,
    tx: SharedTx,
    pending: Pending,
    responses: Receiver<Frame>,
    sender: ObjectSender,
    context: Context,
    binding: Binding,
    role: Role,
    index: u64,
    artifact_index: u64,
    closed: Arc<AtomicBool>,
    cancellation: Cancellation,
    timeout: Duration,
}
impl DuplexWriter {
    // Diagnostic observers are optional; transport inputs stay unchanged.
    #[allow(clippy::too_many_arguments)]
    fn exchange(
        tx: &SharedTx,
        pending: &Pending,
        responses: &Receiver<Frame>,
        routing: &Routing,
        frames: &[Frame],
        purpose: Purpose,
        until: Instant,
        budget: &Arc<AtomicUsize>,
        cancellation: &Cancellation,
        #[cfg(feature = "witness-timing")] timing: &TimingObserver,
        #[cfg(feature = "witness-stall")] stall: &StallObserver,
    ) -> Result<Vec<Frame>> {
        if frames.is_empty() || frames.len() > 8 {
            return Err(Error::Limit);
        }
        for frame in frames {
            frame.encode()?;
            if !matches!(frame.kind, 1..=6) {
                return Err(Error::Protocol);
            }
        }
        let mut replies = Vec::with_capacity(frames.len());
        for frame in frames {
            remaining(until)?;
            {
                let mut pending = lock(pending)?;
                if pending.is_some() {
                    return Err(Error::Protocol);
                }
                *pending = Some(Expected {
                    #[cfg(feature = "witness-stall")]
                    kind: frame.kind,
                    purpose,
                    transfer: frame.transfer,
                    seq: frame.seq,
                    until,
                    seen: budget.clone(),
                });
            }
            {
                #[cfg(feature = "witness-stall")]
                stall.sender(SenderPhase::TxLock);
                let mut tx = tx_lock!(
                    tx,
                    until,
                    cancellation,
                    timing,
                    SenderLock,
                    FrameKind::from_wire(frame.kind)
                )
                .inspect_err(|e| {
                    cancellation.observe(FailureSite::SenderTxLock, Some(frame.kind), e)
                })?;
                #[cfg(feature = "witness-timing")]
                let _span = timing.span(Metric::Frame(
                    Phase::SenderSend,
                    FrameKind::from_wire(frame.kind),
                ));
                #[cfg(feature = "witness-stall")]
                stall.sender(SenderPhase::Send);
                tx.send_object(frame, purpose, until)?;
            }
            #[cfg(feature = "witness-stall")]
            stall.sender(SenderPhase::Wait);
            let response = {
                #[cfg(feature = "witness-timing")]
                let _span = timing.span(Metric::Frame(
                    Phase::SenderWait,
                    FrameKind::from_wire(frame.kind),
                ));
                routing.wait_reply(responses, until, cancellation)
            };
            #[cfg(feature = "witness-timing")]
            timing.count(match &response {
                Ok(reply) if reply.kind == 127 => Observation::WaitRefused,
                Ok(_) => Observation::WaitOk,
                Err(Error::Timeout) => Observation::WaitTimeout,
                Err(_) => Observation::WaitDisconnected,
            });
            let reply = response.inspect_err(|e| {
                cancellation.observe(FailureSite::SenderReplyWait, Some(frame.kind), e)
            })?;
            remaining(until).inspect_err(|e| {
                cancellation.observe(FailureSite::SenderReplyWait, Some(frame.kind), e)
            })?;
            let terminal = matches!(reply.kind, 103 | 127);
            replies.push(reply);
            if terminal {
                break;
            }
        }
        #[cfg(feature = "witness-stall")]
        stall.sender(SenderPhase::Caller);
        Ok(replies)
    }
    fn run<T>(
        &mut self,
        purpose: Purpose,
        until: Instant,
        operation: impl FnOnce(
            &mut ObjectSender,
            &Context,
            &mut dyn FnMut(&[Frame]) -> spiralcore_transfer::Result<Vec<Frame>>,
        ) -> spiralcore_transfer::Result<T>,
    ) -> Result<T> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::Closed);
        }
        self.cancellation.check()?;
        let mut transport_error = None;
        let budget = Arc::new(AtomicUsize::new(0));
        let tx = &self.tx;
        let pending = &self.pending;
        let responses = &self.responses;
        let result = operation(&mut self.sender, &self.context, &mut |frames| {
            Self::exchange(
                tx,
                pending,
                responses,
                &self.routing,
                frames,
                purpose,
                until,
                &budget,
                &self.cancellation,
                #[cfg(feature = "witness-timing")]
                &self.timing,
                #[cfg(feature = "witness-stall")]
                &self.stall,
            )
            .map_err(|error| {
                transport_error = Some(error);
                spiralcore_transfer::Error::Unknown
            })
        });
        match result {
            Ok(value) if Instant::now() < until => Ok(value),
            result => {
                let failure = transport_error.unwrap_or(match result {
                    Ok(_) => Error::Timeout,
                    Err(error) => Error::Core(error),
                });
                self.cancellation
                    .observe(FailureSite::SenderOperation, None, &failure);
                self.cancellation.cancel();
                self.sender.invalidate_receiver_state();
                Err(failure)
            }
        }
    }
    pub fn send_block(&mut self, bytes: Vec<u8>) -> Result<BlockReceipt> {
        self.send_until(bytes, deadline(self.timeout)?)
    }
    pub(crate) fn send_until(&mut self, bytes: Vec<u8>, until: Instant) -> Result<BlockReceipt> {
        if bytes.is_empty() || bytes.len() > MAX_BLOCK {
            return Err(Error::Limit);
        }
        let next = self.index.checked_add(1).ok_or(Error::Sequence)?;
        let index = self.index;
        let transfer = transfer(self.binding, self.role, index, false);
        let object = self.run(Purpose::Application, until, |sender, c, exchange| {
            sender.send_with_receipt(c, bytes, transfer, false, exchange)
        })?;
        self.index = next;
        #[cfg(feature = "witness-timing")]
        self.timing.count(Observation::Ack);
        Ok(BlockReceipt {
            index,
            transfer,
            object,
        })
    }
    fn artifact_id(&mut self) -> Result<Id> {
        let next = self.artifact_index.checked_add(1).ok_or(Error::Sequence)?;
        let id = transfer(self.binding, self.role, self.artifact_index, true);
        self.artifact_index = next;
        Ok(id)
    }
    pub fn seed_base(&mut self, bytes: Vec<u8>) -> Result<Stats> {
        self.seed_base_until(bytes, deadline(self.timeout)?)
    }
    pub(crate) fn seed_base_until(&mut self, bytes: Vec<u8>, until: Instant) -> Result<Stats> {
        if bytes.is_empty() || bytes.len() > 65536 {
            return Err(Error::Limit);
        }
        let id = self.artifact_id()?;
        self.run(Purpose::Artifact, until, |sender, c, exchange| {
            sender.seed(c, bytes, id, exchange)
        })
    }
    pub fn seed_schema(&mut self) -> Result<Stats> {
        self.seed_schema_until(deadline(self.timeout)?)
    }
    pub(crate) fn seed_schema_until(&mut self, until: Instant) -> Result<Stats> {
        let id = self.artifact_id()?;
        self.run(Purpose::Artifact, until, |sender, c, exchange| {
            sender.seed_schema(c, id, exchange)
        })
    }
    pub fn seed_checkpoint(&mut self) -> Result<Stats> {
        self.seed_checkpoint_until(deadline(self.timeout)?)
    }
    pub(crate) fn seed_checkpoint_until(&mut self, until: Instant) -> Result<Stats> {
        let id = self.artifact_id()?;
        self.run(Purpose::Artifact, until, |sender, c, exchange| {
            sender.seed_checkpoint(c, id, exchange)
        })
    }
    pub fn half_close(&mut self) -> Result<()> {
        self.half_close_until(deadline(self.timeout)?)
    }
    pub(crate) fn half_close_until(&mut self, until: Instant) -> Result<()> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::Closed);
        }
        let result = (|| {
            let mut tx = acquire_tx(&self.tx, until, &self.cancellation)?;
            tx.application_half_close(until)?;
            self.closed.store(true, Ordering::Release);
            Ok(())
        })();
        if result.is_err() {
            self.cancellation.cancel();
        }
        result
    }
}
impl Drop for DuplexWriter {
    fn drop(&mut self) {
        if !self.closed.load(Ordering::Acquire) {
            self.cancellation.cancel();
        }
    }
}

pub struct DuplexReader {
    delivery: Receiver<Event>,
    ended: bool,
    cancellation: Cancellation,
    timeout: Duration,
}
impl DuplexReader {
    pub fn receive(&mut self) -> Result<Event> {
        self.receive_until(deadline(self.timeout)?)
    }
    pub(crate) fn receive_until(&mut self, until: Instant) -> Result<Event> {
        self.cancellation.check_delivery()?;
        if self.ended {
            return Err(Error::Closed);
        }
        match self.delivery.recv_timeout(remaining(until)?) {
            Ok(event) => {
                self.cancellation.check_delivery()?;
                if event == Event::HalfClose {
                    self.ended = true;
                }
                Ok(event)
            }
            Err(error) => {
                let failure = match error {
                    RecvTimeoutError::Timeout => Error::Timeout,
                    RecvTimeoutError::Disconnected => Error::Closed,
                };
                self.cancellation
                    .observe(FailureSite::CallerRead, None, &failure);
                self.cancellation.cancel();
                Err(failure)
            }
        }
    }
}
impl Drop for DuplexReader {
    fn drop(&mut self) {
        if !self.ended {
            self.cancellation.cancel();
        }
    }
}

pub struct JoinGuard {
    child: Option<JoinHandle<Result<(Counters, Counters)>>>,
    done: Receiver<()>,
    done_seen: bool,
    joined: Option<Result<(Counters, Counters)>>,
    panicked: bool,
    cancellation: Cancellation,
    timeout: Duration,
}
impl JoinGuard {
    /// Complete the original reader without relinquishing custody on timeout.
    /// Physical completion does not replace its retained semantic result.
    pub fn try_join_until(&mut self, until: Instant) -> Result<()> {
        let result = (|| {
            if self.joined.is_some() {
                return if self.panicked {
                    Err(Error::Worker)
                } else {
                    Ok(())
                };
            }
            if !self.done_seen {
                match self.done.recv_timeout(remaining(until)?) {
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => self.done_seen = true,
                    Err(RecvTimeoutError::Timeout) => return Err(Error::Timeout),
                }
            }
            // The completion signal can precede the native thread epilogue.
            // Keep the original handle while waiting on the same deadline.
            while !self.child.as_ref().ok_or(Error::Worker)?.is_finished() {
                thread::park_timeout(remaining(until)?.min(Duration::from_millis(1)));
            }
            remaining(until)?;
            let child = self.child.take().ok_or(Error::Worker)?;
            self.joined = Some(match child.join() {
                Ok(result) => result,
                Err(_) => {
                    self.panicked = true;
                    Err(Error::Worker)
                }
            });
            remaining(until)?;
            if self.panicked {
                Err(Error::Worker)
            } else {
                Ok(())
            }
        })();
        if result.is_err() {
            self.cancellation.cancel();
        }
        result
    }
    pub fn join(mut self) -> Result<(Counters, Counters)> {
        if let Some(result) = self.joined.take() {
            return result;
        }
        let expired = matches!(
            self.done.recv_timeout(self.timeout),
            Err(RecvTimeoutError::Timeout)
        );
        if expired {
            self.cancellation.cancel();
        }
        let result = match self.child.take() {
            Some(child) => child.join().unwrap_or(Err(Error::Worker)),
            None => Err(Error::Worker),
        };
        if expired || result.is_err() {
            self.cancellation.cancel();
        }
        if expired { Err(Error::Timeout) } else { result }
    }
}
impl Drop for JoinGuard {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            self.cancellation.cancel();
            let _ = child.join();
        }
    }
}

enum ReceiveEndpoint<A: Authority> {
    General(Box<ObjectEndpoint<A>>),
    Ordered(Box<OrderedObjectEndpoint<A>>),
}
impl<A: Authority> ReceiveEndpoint<A> {
    fn snapshot(&self, context: &Context) -> spiralcore_transfer::Result<Option<OrderedSnapshot>> {
        match self {
            Self::General(_) => Ok(None),
            Self::Ordered(endpoint) => endpoint.snapshot(context).map(Some),
        }
    }
    fn handle(
        &mut self,
        purpose: Purpose,
        ordinal: u64,
        context: &Context,
        frame: &Frame,
        now: u64,
    ) -> spiralcore_transfer::Result<(Frame, Option<Vec<u8>>)> {
        match self {
            Self::General(endpoint) => endpoint
                .handle(context, frame, now)
                .map(|reply| (reply, None)),
            Self::Ordered(endpoint) if purpose == Purpose::Application => {
                endpoint.handle_application(ordinal, context, frame, now)
            }
            Self::Ordered(endpoint) => endpoint
                .handle(OrderedLane::Artifact, ordinal, context, frame, now)
                .map(|reply| (reply, None)),
        }
    }
    fn enqueue(
        &self,
        context: &Context,
        sender: &SyncSender<Event>,
        event: Event,
    ) -> Result<std::result::Result<(), TrySendError<Event>>> {
        if let (Self::Ordered(endpoint), Event::Data(_)) = (self, &event) {
            endpoint.check_delivery_authority(context)?;
        }
        Ok(sender.try_send(event))
    }
    fn application_bytes(
        &mut self,
        context: &Context,
        id: Id,
        verified: Option<Vec<u8>>,
    ) -> spiralcore_transfer::Result<Vec<u8>> {
        match (self, verified) {
            (Self::General(endpoint), None) => endpoint.read_object(context, id),
            (Self::Ordered(_), Some(bytes)) => Ok(bytes),
            _ => Err(spiralcore_transfer::Error::Integrity),
        }
    }
}
struct Worker<A: Authority> {
    #[cfg(feature = "witness-stall")]
    stall: StallObserver,
    #[cfg(feature = "witness-timing")]
    timing: TimingObserver,
    completed: bool,
    routing: Arc<Routing>,
    tx: SharedTx,
    pending: Pending,
    delivery: SyncSender<Event>,
    endpoint: ReceiveEndpoint<A>,
    context: Context,
    binding: Binding,
    role: Role,
    index: u64,
    artifact_index: u64,
    active: Option<(Id, Id, u64)>,
    remote_closed: bool,
    local_closed: Arc<AtomicBool>,
    cancellation: Cancellation,
    timeout: Duration,
    retention: RetentionObserver,
}
impl<A: Authority> Worker<A> {
    fn complete(&mut self) -> Result<(Counters, Counters)> {
        self.cancellation.check_delivery()?;
        let counters = (
            acquire_tx(&self.tx, deadline(self.timeout)?, &self.cancellation)?.counters(),
            self.routing
                .reader()?
                .as_ref()
                .ok_or(Error::Closed)?
                .counters(),
        );
        self.completed = true;
        self.cancellation.complete();
        Ok(counters)
    }
    fn deliver(&self, mut event: Event, until: Instant) -> Result<()> {
        loop {
            self.cancellation.check()?;
            let left = remaining(until)?;
            match self
                .endpoint
                .enqueue(&self.context, &self.delivery, event)?
            {
                Ok(()) => return Ok(()),
                Err(TrySendError::Disconnected(_)) => return Err(Error::Closed),
                Err(TrySendError::Full(returned)) => {
                    event = returned;
                    thread::park_timeout(left.min(Duration::from_millis(5)));
                }
            }
        }
    }
    fn request(&mut self, purpose: Purpose, request: Frame, until: Instant) -> Result<()> {
        if self.remote_closed {
            return Err(Error::Protocol);
        }
        if purpose == Purpose::Application {
            if request.transfer != transfer(self.binding, self.role, self.index, false) {
                return Err(Error::Sequence);
            }
            if let Some((id, _, _)) = self.active {
                if request.transfer != id {
                    return Err(Error::Sequence);
                }
            } else if !matches!(request.kind, 1 | 4) {
                return Err(Error::Protocol);
            }
        }
        if purpose == Purpose::Application && matches!(request.kind, 1 | 4) {
            let body = if request.kind == 4 {
                request.body.get(32..).ok_or(Error::Protocol)?
            } else {
                &request.body
            };
            let offer = Offer::decode(body)?;
            if offer.len == 0 || offer.len > MAX_BLOCK as u64 {
                return Err(Error::Limit);
            }
            let candidate = (request.transfer, offer.object, offer.len);
            if self.active.is_some_and(|active| active != candidate) {
                return Err(Error::Protocol);
            }
            self.active = Some(candidate);
        }
        let ordinal = if purpose == Purpose::Application {
            self.index
        } else {
            self.artifact_index
        };
        #[cfg(feature = "witness-stall")]
        self.stall
            .mark(WorkerPhase::EndpointHandle, request.kind, ordinal);
        let (reply, readback) = {
            #[cfg(feature = "witness-timing")]
            let _span = self.timing.span(Metric::Frame(
                Phase::ReceiverHandle,
                FrameKind::from_wire(request.kind),
            ));
            self.endpoint
                .handle(purpose, ordinal, &self.context, &request, now()?)
                .map_err(Error::from)
                .inspect_err(|e| {
                    self.cancellation
                        .observe(FailureSite::ReceiverHandle, Some(request.kind), e)
                })?
        };
        #[cfg(feature = "witness-stall")]
        self.stall
            .mark(WorkerPhase::Retention, request.kind, ordinal);
        self.retention
            .publish(self.endpoint.snapshot(&self.context)?)?;
        if purpose == Purpose::Application && reply.kind == 103 {
            let (_, id, len) = self.active.ok_or(Error::Protocol)?;
            if reply.body != id {
                return Err(Error::Integrity);
            }
            #[cfg(feature = "witness-stall")]
            self.stall
                .mark(WorkerPhase::Readback, request.kind, ordinal);
            let bytes = self
                .endpoint
                .application_bytes(&self.context, id, readback)
                .map_err(Error::from)
                .inspect_err(|e| {
                    self.cancellation
                        .observe(FailureSite::ReceiverReadback, Some(request.kind), e)
                })?;
            if bytes.len() as u64 != len {
                return Err(Error::Integrity);
            }
            let next = self.index.checked_add(1).ok_or(Error::Sequence)?;
            #[cfg(feature = "witness-stall")]
            self.stall
                .mark(WorkerPhase::Delivery, request.kind, ordinal);
            self.deliver(Event::Data(bytes), until).inspect_err(|e| {
                self.cancellation
                    .observe(FailureSite::ReceiverDelivery, Some(request.kind), e)
            })?;
            self.index = next;
            self.active = None;
        }
        if purpose == Purpose::Artifact && reply.kind == 103 {
            self.artifact_index = self.artifact_index.checked_add(1).ok_or(Error::Sequence)?;
        }
        #[cfg(feature = "witness-stall")]
        self.stall
            .mark(WorkerPhase::ReplyLock, request.kind, ordinal);
        let mut tx = tx_lock!(
            &self.tx,
            until,
            &self.cancellation,
            &self.timing,
            ReceiverLock,
            FrameKind::from_wire(request.kind)
        )
        .inspect_err(|e| {
            self.cancellation
                .observe(FailureSite::ReceiverReplyLock, Some(request.kind), e)
        })?;
        #[cfg(feature = "witness-timing")]
        let _span = self.timing.span(Metric::Frame(
            Phase::ReceiverSend,
            FrameKind::from_wire(request.kind),
        ));
        #[cfg(feature = "witness-stall")]
        self.stall
            .mark(WorkerPhase::ReplyWrite, request.kind, ordinal);
        tx.send_object(&reply, purpose, until)
    }
    fn run(mut self) -> Result<(Counters, Counters)> {
        let result = self.run_inner();
        self.routing
            .finish(result)
            .inspect_err(|e| self.cancellation.observe(FailureSite::WorkerExit, None, e))
    }
    fn run_inner(&mut self) -> Result<(Counters, Counters)> {
        loop {
            if self.remote_closed
                && self.local_closed.load(Ordering::Acquire)
                && lock(&self.pending)?.is_none()
            {
                return self.complete();
            }
            #[cfg(feature = "witness-stall")]
            self.stall.mark(WorkerPhase::Poll, 0, self.index);
            let routing = self.routing.clone();
            let mut held = routing.worker_reader()?;
            let rx = held.as_mut().ok_or(Error::Closed)?;
            match rx.poll_input() {
                Ok(false) => continue,
                Ok(true) => {}
                Err(error)
                    if self.remote_closed
                        && (matches!(&error, Error::Truncated)
                            || matches!(&error, Error::Io(io) if io.kind() == std::io::ErrorKind::UnexpectedEof)) =>
                {
                    // A peer can observe our FIN before the writing thread
                    // publishes local_closed. Preserve the original TX barrier
                    // before deciding whether this EOF is a clean completion.
                    let tx = acquire_tx(&self.tx, deadline(self.timeout)?, &self.cancellation)?;
                    if self.local_closed.load(Ordering::Acquire) && lock(&self.pending)?.is_none() {
                        drop(tx);
                        drop(held);
                        return self.complete();
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            }
            let until = routing.read_deadline(rx, deadline(self.timeout)?)?;
            #[cfg(feature = "witness-stall")]
            self.stall.mark(WorkerPhase::Receive, 0, self.index);
            let record = rx.receive_object(until)?;
            if let ObjectRecord::Frame(purpose, frame) = record {
                if !matches!(frame.kind, 1..=6) {
                    routing.route(purpose, frame)?;
                    continue;
                }
                let _inbound = routing.inbound();
                drop(held);
                routing.ready();
                self.request(purpose, frame, until)?;
                continue;
            }
            let _inbound = routing.inbound();
            drop(held);
            routing.ready();
            match record {
                ObjectRecord::Frame(_, _) => return Err(Error::Protocol),
                ObjectRecord::ApplicationHalfClose => {
                    if self.remote_closed || self.active.is_some() {
                        return Err(Error::Protocol);
                    }
                    self.deliver(Event::HalfClose, until)?;
                    self.remote_closed = true;
                }
                ObjectRecord::TransportFin => return Err(Error::Protocol),
            }
        }
    }
}
#[cfg(test)]
mod reply_progress_tests;
#[cfg(test)]
mod tests;

impl<A: Authority> Drop for Worker<A> {
    fn drop(&mut self) {
        if !self.completed {
            self.cancellation.cancel();
        }
        self.routing.retire();
    }
}
