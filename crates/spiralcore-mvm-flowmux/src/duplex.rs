use crate::{
    Binding, Cancellation, Counters, Error, Event, MAX_BLOCK, ObjectChannel, ObjectRecord, Purpose,
    Result, Rx, Tx,
};
use spiralcore_transfer::{
    Id,
    client::Stats,
    endpoint::{Authority, Context},
    hash,
    protocol::{Frame, Offer},
    v1::{ObjectEndpoint, ObjectReceipt, ObjectSender},
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
    hash(
        if artifact {
            b"spiralcore.flow.artifact.v1\0"
        } else {
            b"spiralcore.flow.application.v1\0"
        },
        &[&binding.flow, &[role.tag()], &index.to_be_bytes()],
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
struct Expected {
    purpose: Purpose,
    transfer: Id,
    seq: u64,
    until: Instant,
    seen: Arc<AtomicUsize>,
}
type Pending = Arc<Mutex<Option<Expected>>>;
type SharedTx = Arc<Mutex<Tx>>;

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
}
impl Duplex {
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
        let shared_tx = Arc::new(Mutex::new(channel.tx));
        let pending = Arc::new(Mutex::new(None));
        let closed = Arc::new(AtomicBool::new(false));
        let (responses, response_rx) = mpsc::sync_channel(1);
        let (delivery, delivery_rx) = mpsc::sync_channel(4);
        let (done, done_rx) = mpsc::sync_channel(1);
        let worker = Worker {
            completed: false,
            rx: channel.rx,
            tx: shared_tx.clone(),
            pending: pending.clone(),
            responses,
            delivery,
            endpoint,
            context: context.clone(),
            binding,
            role: role.remote(),
            index: 0,
            active: None,
            remote_closed: false,
            local_closed: closed.clone(),
            cancellation: cancellation.clone(),
            timeout,
        };
        let child = thread::Builder::new()
            .name("spiral-flow-reader".into())
            .spawn(move || {
                let result = worker.run();
                let _ = done.send(());
                result
            })?;
        Ok(Self {
            writer: DuplexWriter {
                tx: shared_tx,
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
        })
    }
    pub fn split(self) -> (DuplexWriter, DuplexReader, JoinGuard) {
        (self.writer, self.reader, self.guard)
    }
}

pub struct DuplexWriter {
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
    fn exchange(
        tx: &SharedTx,
        pending: &Pending,
        responses: &Receiver<Frame>,
        frames: &[Frame],
        purpose: Purpose,
        until: Instant,
        budget: &Arc<AtomicUsize>,
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
                    purpose,
                    transfer: frame.transfer,
                    seq: frame.seq,
                    until,
                    seen: budget.clone(),
                });
            }
            lock_until(tx, until)?.send_object(frame, purpose, until)?;
            let reply = responses
                .recv_timeout(remaining(until)?)
                .map_err(|error| match error {
                    RecvTimeoutError::Timeout => Error::Timeout,
                    RecvTimeoutError::Disconnected => Error::Closed,
                })?;
            remaining(until)?;
            let terminal = matches!(reply.kind, 103 | 127);
            replies.push(reply);
            if terminal {
                break;
            }
        }
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
            Self::exchange(tx, pending, responses, frames, purpose, until, &budget).map_err(
                |error| {
                    transport_error = Some(error);
                    spiralcore_transfer::Error::Unknown
                },
            )
        });
        match result {
            Ok(value) if Instant::now() < until => Ok(value),
            result => {
                self.cancellation.cancel();
                self.sender.invalidate_receiver_state();
                Err(transport_error.unwrap_or(match result {
                    Ok(_) => Error::Timeout,
                    Err(error) => Error::Core(error),
                }))
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
            let mut tx = lock_until(&self.tx, until)?;
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
                self.cancellation.cancel();
                Err(match error {
                    RecvTimeoutError::Timeout => Error::Timeout,
                    RecvTimeoutError::Disconnected => Error::Closed,
                })
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

struct Worker<A: Authority> {
    completed: bool,
    rx: Rx,
    tx: SharedTx,
    pending: Pending,
    responses: SyncSender<Frame>,
    delivery: SyncSender<Event>,
    endpoint: ObjectEndpoint<A>,
    context: Context,
    binding: Binding,
    role: Role,
    index: u64,
    active: Option<(Id, Id, u64)>,
    remote_closed: bool,
    local_closed: Arc<AtomicBool>,
    cancellation: Cancellation,
    timeout: Duration,
}
impl<A: Authority> Worker<A> {
    fn complete(&mut self) -> Result<(Counters, Counters)> {
        let counters = (lock(&self.tx)?.counters(), self.rx.counters());
        self.completed = true;
        self.cancellation.complete();
        Ok(counters)
    }
    fn deliver(&self, mut event: Event, until: Instant) -> Result<()> {
        loop {
            self.cancellation.check()?;
            let left = remaining(until)?;
            match self.delivery.try_send(event) {
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
        let reply = self.endpoint.handle(&self.context, &request, now()?)?;
        if purpose == Purpose::Application {
            if matches!(request.kind, 1 | 4) {
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
            if reply.kind == 103 {
                let (_, id, len) = self.active.ok_or(Error::Protocol)?;
                if reply.body != id {
                    return Err(Error::Integrity);
                }
                let bytes = self.endpoint.read_object(&self.context, id)?;
                if bytes.len() as u64 != len {
                    return Err(Error::Integrity);
                }
                let next = self.index.checked_add(1).ok_or(Error::Sequence)?;
                self.deliver(Event::Data(bytes), until)?;
                self.index = next;
                self.active = None;
            }
        }
        lock_until(&self.tx, until)?.send_object(&reply, purpose, until)
    }
    fn run(mut self) -> Result<(Counters, Counters)> {
        loop {
            if self.remote_closed
                && self.local_closed.load(Ordering::Acquire)
                && lock(&self.pending)?.is_none()
            {
                return self.complete();
            }
            match self.rx.poll_input() {
                Ok(false) => continue,
                Ok(true) => {}
                Err(Error::Truncated) if self.remote_closed => {
                    // A peer can observe our FIN before the writing thread
                    // publishes local_closed. Preserve the original TX barrier
                    // before deciding whether this EOF is a clean completion.
                    let tx = lock(&self.tx)?;
                    if self.local_closed.load(Ordering::Acquire) && lock(&self.pending)?.is_none() {
                        drop(tx);
                        return self.complete();
                    }
                    return Err(Error::Truncated);
                }
                Err(error) => return Err(error),
            }
            let mut until = deadline(self.timeout)?;
            {
                let mut pending = lock(&self.pending)?;
                if let Some(expected) = pending.as_mut() {
                    remaining(expected.until)?;
                    let seen = expected
                        .seen
                        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
                        .map_err(|_| Error::Limit)?;
                    if seen >= 32 {
                        return Err(Error::Limit);
                    }
                    until = until.min(expected.until);
                }
            }
            match self.rx.receive_object(until)? {
                ObjectRecord::Frame(purpose, frame) if matches!(frame.kind, 1..=6) => {
                    self.request(purpose, frame, until)?
                }
                ObjectRecord::Frame(purpose, frame) => {
                    let expected = lock(&self.pending)?.take().ok_or(Error::Protocol)?;
                    if purpose != expected.purpose
                        || frame.transfer != expected.transfer
                        || frame.seq != expected.seq
                    {
                        return Err(Error::Protocol);
                    }
                    remaining(expected.until)?;
                    self.responses
                        .try_send(frame)
                        .map_err(|_| Error::Protocol)?;
                }
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
impl<A: Authority> Drop for Worker<A> {
    fn drop(&mut self) {
        if !self.completed {
            self.cancellation.cancel();
        }
    }
}
