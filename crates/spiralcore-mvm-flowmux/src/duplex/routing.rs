//! Cooperative reply progress through one original decoder and pending operation.
use super::*;
use std::sync::{Condvar, TryLockError, mpsc::TryRecvError};

#[derive(Default)]
pub(crate) struct Wake {
    notified: Mutex<bool>,
    changed: Condvar,
}
impl Wake {
    pub(crate) fn notify(&self) {
        let mut notified = self.notified.lock().unwrap_or_else(|e| e.into_inner());
        *notified = true;
        self.changed.notify_all();
    }
    fn arm(&self) -> Result<()> {
        *lock(&self.notified)? = false;
        Ok(())
    }
    fn wait(&self, until: Instant, cancellation: &Cancellation) -> Result<()> {
        cancellation.check()?;
        let mut notified = lock(&self.notified)?;
        while !*notified {
            if cancellation.stopped().load(Ordering::Acquire) {
                drop(notified);
                cancellation.check()?;
                return Err(Error::Closed);
            }
            notified = self
                .changed
                .wait_timeout(notified, remaining(until)?)
                .map_err(|_| Error::Worker)?
                .0;
        }
        drop(notified);
        cancellation.check()?;
        Ok(())
    }
}

pub(super) struct Routing {
    reader: Mutex<Option<Rx>>,
    pub(super) pending: Pending,
    responses: SyncSender<Frame>,
    wake: Arc<Wake>,
    busy: AtomicBool,
}
pub(super) struct Inbound(Arc<Routing>);
impl Drop for Inbound {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::Release);
        self.0.wake.notify();
    }
}
impl Routing {
    pub(super) fn new(
        rx: Rx,
        pending: Pending,
        responses: SyncSender<Frame>,
        cancellation: &Cancellation,
    ) -> Result<Arc<Self>> {
        let wake = Arc::new(Wake::default());
        cancellation.bind_reply_wake(&wake)?;
        Ok(Arc::new(Self {
            reader: Mutex::new(Some(rx)),
            pending,
            responses,
            wake,
            busy: AtomicBool::new(false),
        }))
    }
    pub(super) fn ready(&self) {
        self.wake.notify();
    }
    pub(super) fn reader(&self) -> Result<MutexGuard<'_, Option<Rx>>> {
        lock(&self.reader)
    }
    pub(super) fn inbound(self: &Arc<Self>) -> Inbound {
        self.busy.store(true, Ordering::Release);
        self.wake.notify();
        Inbound(self.clone())
    }
    /// Only the original worker retires the transport, including unwind paths.
    pub(super) fn retire(&self) {
        let rx = self.reader.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(rx);
        self.wake.notify();
    }
    pub(super) fn read_deadline(&self, rx: &Rx, until: Instant) -> Result<Instant> {
        let retained = rx.object_deadline();
        let mut until = retained.map_or(until, |old| old.min(until));
        let pending = lock(&self.pending)?;
        if let Some(expected) = pending.as_ref() {
            remaining(expected.until)?;
            until = until.min(expected.until);
            if retained.is_none() {
                let seen = expected
                    .seen
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
                    .map_err(|_| Error::Limit)?;
                if seen >= 32 {
                    return Err(Error::Limit);
                }
            }
        }
        remaining(until)?;
        Ok(until)
    }
    /// Called with the original decoder held, so reply publication is serialized.
    pub(super) fn route(&self, purpose: Purpose, frame: Frame) -> Result<()> {
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
        self.wake.notify();
        Ok(())
    }
    fn assist(&self, until: Instant) -> Result<bool> {
        if !self.busy.load(Ordering::Acquire) {
            return Ok(false);
        }
        let mut held = match self.reader.try_lock() {
            Ok(held) => held,
            Err(TryLockError::WouldBlock) => return Ok(false),
            Err(TryLockError::Poisoned(_)) => return Err(Error::Worker),
        };
        if !self.busy.load(Ordering::Acquire) || lock(&self.pending)?.is_none() {
            return Ok(false);
        }
        let rx = held.as_mut().ok_or(Error::Closed)?;
        // A retained prefix belongs to the original worker. Do not spin or read past it.
        if rx.object_deadline().is_some() {
            return Ok(false);
        }
        if !rx.poll_input()? {
            return Ok(true);
        }
        let until = self.read_deadline(rx, until)?;
        if rx.prepare_object(until)? {
            match rx.receive_object(until)? {
                ObjectRecord::Frame(purpose, frame) => self.route(purpose, frame)?,
                ObjectRecord::ApplicationHalfClose | ObjectRecord::TransportFin => {
                    return Err(Error::Protocol);
                }
            }
        }
        Ok(rx.object_deadline().is_none())
    }
    pub(super) fn wait_reply(
        &self,
        responses: &Receiver<Frame>,
        until: Instant,
        cancellation: &Cancellation,
    ) -> Result<Frame> {
        loop {
            // Arm before checking predicates so publication/handler completion cannot be lost.
            self.wake.arm()?;
            cancellation.check()?;
            remaining(until)?;
            match responses.try_recv() {
                Ok(frame) => return Ok(frame),
                Err(TryRecvError::Disconnected) => return Err(Error::Closed),
                Err(TryRecvError::Empty) => {}
            }
            if self.assist(until)? {
                continue;
            }
            self.wake.wait(until, cancellation)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record;
    use spiralcore_transfer::v1::Capabilities;
    use std::{
        io::Write,
        net::{TcpListener, TcpStream},
    };
    struct Pair {
        routing: Arc<Routing>,
        cancellation: Cancellation,
        responses: Receiver<Frame>,
        _tx: Tx,
        peer: TcpStream,
        binding: Binding,
        seen: Arc<AtomicUsize>,
    }
    fn pair() -> Pair {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let client = TcpStream::connect(address).unwrap();
        let (peer, _) = listener.accept().unwrap();
        let binding = Binding {
            flow: [1; 32],
            policy_sha256: [2; 32],
            scope_commitment: [3; 32],
        };
        let caps = Capabilities::small_v1();
        let mut channel = ObjectChannel::from_connected(
            client,
            binding,
            address,
            Duration::from_secs(2),
            caps,
            caps,
        )
        .unwrap();
        channel.rx.role = 1;
        let cancellation = channel.cancellation();
        let seen = Arc::new(AtomicUsize::new(0));
        let pending = Arc::new(Mutex::new(Some(Expected {
            #[cfg(feature = "witness-stall")]
            kind: 1,
            purpose: Purpose::Application,
            transfer: [7; 32],
            seq: 0,
            until: Instant::now() + Duration::from_secs(2),
            seen: seen.clone(),
        })));
        let (sender, responses) = mpsc::sync_channel(1);
        let routing = Routing::new(channel.rx, pending, sender, &cancellation).unwrap();
        Pair {
            routing,
            cancellation,
            responses,
            _tx: channel.tx,
            peer,
            binding,
            seen,
        }
    }
    fn send(pair: &mut Pair, kind: u8) {
        let body = Frame {
            kind,
            transfer: [7; 32],
            seq: 0,
            body: vec![],
        }
        .encode()
        .unwrap();
        pair.peer
            .write_all(&record::header(pair.binding, 0, record::OBJECT, body.len(), 1).unwrap())
            .unwrap();
        pair.peer.write_all(&body).unwrap();
    }
    #[test]
    fn deferred_request_is_charged_once_and_cancellation_wakes_original_waiter() {
        let mut pair = pair();
        let _inbound = pair.routing.inbound();
        send(&mut pair, 1);
        let until = Instant::now() + Duration::from_secs(1);
        assert!(!pair.routing.assist(until).unwrap());
        assert_eq!(pair.seen.load(Ordering::Acquire), 1);
        {
            let mut held = pair.routing.reader().unwrap();
            let rx = held.as_mut().unwrap();
            pair.routing.read_deadline(rx, until).unwrap();
            assert_eq!(pair.seen.load(Ordering::Acquire), 1);
        }
        pair.cancellation.cancel();
        assert!(matches!(
            pair.routing
                .wait_reply(&pair.responses, until, &pair.cancellation),
            Err(Error::Cancelled)
        ));
    }
    #[test]
    fn prefix_resume_tightens_deadline_and_frame_thirty_three_is_refused() {
        let mut pair = pair();
        send(&mut pair, 1);
        let until = Instant::now() + Duration::from_secs(1);
        let mut held = pair.routing.reader().unwrap();
        let rx = held.as_mut().unwrap();
        let original = pair.routing.read_deadline(rx, until).unwrap();
        rx.prepare_object(original).unwrap();
        let tighter = Instant::now() + Duration::from_millis(100);
        lock(&pair.routing.pending).unwrap().as_mut().unwrap().until = tighter;
        assert_eq!(pair.routing.read_deadline(rx, until).unwrap(), tighter);
        rx.receive_object(tighter).unwrap();
        pair.seen.store(32, Ordering::Release);
        assert!(matches!(
            pair.routing.read_deadline(rx, until),
            Err(Error::Limit)
        ));
    }
    #[test]
    fn reply_correlation_and_single_response_slot_remain_mandatory() {
        for field in 0..3 {
            let pair = pair();
            let mut reply = Frame {
                kind: 101,
                transfer: [7; 32],
                seq: 0,
                body: vec![],
            };
            let mut purpose = Purpose::Application;
            match field {
                0 => purpose = Purpose::Artifact,
                1 => reply.transfer = [8; 32],
                2 => reply.seq = 1,
                _ => unreachable!(),
            }
            assert!(matches!(
                pair.routing.route(purpose, reply),
                Err(Error::Protocol)
            ));
            assert!(matches!(
                pair.responses.try_recv(),
                Err(TryRecvError::Empty)
            ));
        }
        let pair = pair();
        pair.routing
            .route(
                Purpose::Application,
                Frame {
                    kind: 101,
                    transfer: [7; 32],
                    seq: 0,
                    body: vec![],
                },
            )
            .unwrap();
        assert!(lock(&pair.routing.pending).unwrap().is_none());
        assert!(matches!(
            pair.routing.route(
                Purpose::Application,
                Frame {
                    kind: 101,
                    transfer: [7; 32],
                    seq: 0,
                    body: vec![]
                }
            ),
            Err(Error::Protocol)
        ));
    }
    #[test]
    fn retirement_removes_original_rx_even_with_caller_reference() {
        let pair = pair();
        let caller = pair.routing.clone();
        pair.routing.retire();
        assert!(caller.reader().unwrap().is_none());
        assert!(matches!(
            pair.routing.wait_reply(
                &pair.responses,
                Instant::now() + Duration::from_secs(1),
                &pair.cancellation
            ),
            Err(Error::Cancelled)
        ));
    }
    #[test]
    fn fixed_control_state_fits_explicit_metadata_reservation() {
        let fixed = std::mem::size_of::<Routing>()
            + std::mem::size_of::<Wake>()
            + 8 * std::mem::size_of::<usize>()
            + crate::HEADER_LEN
            + 128;
        assert!(fixed < 8 * 1024);
        assert!(
            spiralcore_transfer::resource_profile::ResourceProfile::SmallV1
                .operation_charge()
                .unwrap()
                < 8 * 1024 * 1024
        );
    }
}
