use super::*;
use mvm_core::mailbox::transport::{
    AuthorizedMailboxFrame, AuthorizedRoute, ChannelLimits, OutboundAuthorization, PeerTransport,
    PreparedMailboxEnvelope, TransportAssurance, TransportBindingContext, TransportError,
    authorize_outbound, authorize_route,
};
use spiralcore_mvm_adapter::{
    live::SocketIo,
    service::{CallHandle, MailboxBinding, ServiceBindingConfig, ServiceError},
};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::Shutdown,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct State {
    incoming: VecDeque<u8>,
    written: Vec<u8>,
    timeout: Option<Duration>,
    stall_after: Option<usize>,
    shutdowns: usize,
    write_timeouts: Vec<Duration>,
}
struct Socket(Arc<Mutex<State>>);
impl Read for Socket {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let mut state = self.0.lock().unwrap();
        let count = bytes.len().min(state.incoming.len());
        for byte in &mut bytes[..count] {
            *byte = state.incoming.pop_front().unwrap();
        }
        Ok(count)
    }
}
impl Write for Socket {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let timeout = {
            let mut state = self.0.lock().unwrap();
            let count = state.stall_after.map_or(bytes.len(), |cap| {
                cap.saturating_sub(state.written.len()).min(bytes.len())
            });
            if count > 0 {
                state.written.extend_from_slice(&bytes[..count]);
                return Ok(count);
            }
            state
                .timeout
                .ok_or_else(|| io::Error::other("missing bounded write timeout"))?
        };
        // Model bounded backpressure while honoring the real helper's installed
        // timeout. No success is reported for bytes that were not accepted.
        thread::sleep(timeout);
        Err(io::ErrorKind::WouldBlock.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl SocketIo for Socket {
    fn set_read_timeout(&self, _timeout: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        let mut state = self.0.lock().unwrap();
        state.timeout = timeout;
        if let Some(timeout) = timeout {
            state.write_timeouts.push(timeout);
        }
        Ok(())
    }
    fn shutdown(&self, _how: Shutdown) -> io::Result<()> {
        self.0.lock().unwrap().shutdowns += 1;
        Ok(())
    }
}
fn ready(timeout: Duration) -> (StreamTransport<Socket>, Arc<Mutex<State>>) {
    let state = Arc::new(Mutex::new(State {
        incoming: b"SCMVS001".iter().copied().collect(),
        ..State::default()
    }));
    let stream = StreamTransport::new(
        Socket(state.clone()),
        common::config(MailboxMessageKind::Call)
            .binding
            .claims
            .route_id,
        timeout,
    )
    .unwrap();
    stream
        .handshake(HandshakeRole::Initiator, &CancellationToken::default())
        .unwrap();
    (stream, state)
}
fn route(kind: MailboxMessageKind) -> AuthorizedRoute {
    let config = common::config(kind);
    let context = config.policy.snapshot().unwrap();
    authorize_route(
        &config.binding,
        &TransportBindingContext {
            expected_issuer: context.expected_capability_issuer,
            expected_address: config.capability.grant.address,
            current_execution_epoch: context.current_execution_epoch,
            current_policy_epoch: context.current_policy_epoch,
            minimum_assurance: TransportAssurance::Development,
            now_unix_ms: fixture::time().unix_ms,
        },
        &config.authority_key,
    )
    .unwrap()
}
struct Call {
    service: LocalMailboxService,
    binding: MailboxBinding,
    handle: CallHandle,
    prepared: PreparedMailboxEnvelope,
    frame: AuthorizedMailboxFrame,
    route: AuthorizedRoute,
}
impl Call {
    fn new() -> Self {
        let config = common::config(MailboxMessageKind::Call);
        let context = config.policy.snapshot().unwrap();
        let capability = config.capability.clone();
        let authority = config.authority_key;
        let reply_to = config.reply_to;
        let binding =
            MailboxBinding::new(ServiceBindingConfig::from_direction(config).unwrap(), None)
                .unwrap();
        let mut options = MessageOptions::fresh(
            context.accepted_content_types[0].clone(),
            fixture::time(),
            2000,
        )
        .unwrap();
        options.reply_to = reply_to;
        let prepared = service_sender::prepare(
            &binding,
            b"canonical\0deadline\xff",
            &options,
            fixture::time(),
        )
        .unwrap();
        let route = route(MailboxMessageKind::Call);
        let frame = authorize_outbound(OutboundAuthorization {
            prepared: &prepared,
            capability: &capability,
            context: &context,
            capability_authority_key: &authority,
            route: &route,
        })
        .unwrap();
        let mut service = LocalMailboxService::new(budget()).unwrap();
        let handle = service
            .register_call(&binding, frame.encoded_bytes(), fixture::time())
            .unwrap();
        Self {
            service,
            binding,
            handle,
            prepared,
            frame,
            route,
        }
    }
    fn send(&self, transport: &impl PeerTransport) -> Result<usize, ServiceError> {
        self.service.send_call_prepared(
            &self.binding,
            &self.handle,
            &self.prepared,
            transport,
            &CancellationToken::default(),
            fixture::time(),
        )
    }
    fn limits(&self) -> ChannelLimits {
        ChannelLimits::new(self.frame.encoded_len(), self.frame.encoded_len()).unwrap()
    }
}
fn wait_until(at: Instant) {
    assert!(at.saturating_duration_since(Instant::now()) < Duration::from_secs(1));
    while let Some(left) = at.checked_duration_since(Instant::now()) {
        thread::park_timeout(left);
    }
}

#[test]
fn fixed_view_caps_authorized_blocked_prefix_and_partial_payload_under_original_deadline() {
    for (prefix, payload) in [(0, 0), (2, 0), (4, 7)] {
        let call = Call::new();
        let (stream, state) = ready(Duration::from_secs(2));
        state.lock().unwrap().stall_after = Some(8 + prefix + payload);
        let deadline = Instant::now() + Duration::from_millis(100);
        let before = stream.before(deadline);
        let started = Instant::now();
        assert_eq!(call.send(&before), Err(ServiceError::Unavailable));
        assert!(Instant::now() >= deadline);
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "write used the longer parent timeout"
        );
        let counters = stream.counters().unwrap();
        assert_eq!(counters.frames_sent, 0);
        assert_eq!(counters.framing_bytes_sent, (8 + prefix) as u64);
        assert_eq!(counters.envelope_bytes_sent, payload as u64);
        let snapshot = serde_json::to_value(counters).unwrap();
        assert_eq!(state.lock().unwrap().written.len(), 8 + prefix + payload);
        assert!(state.lock().unwrap().shutdowns > 0);
        assert_eq!(call.send(&stream), Err(ServiceError::ShuttingDown));
        assert_eq!(
            serde_json::to_value(stream.counters().unwrap()).unwrap(),
            snapshot
        );
    }
}

#[test]
fn precreated_channels_repeated_views_and_parent_timeout_never_refresh_creation_deadline() {
    for phase in [
        "expired-connect",
        "held-channel",
        "repeated-views",
        "parent-cap",
    ] {
        let call = Call::new();
        let timeout = if phase == "parent-cap" {
            Duration::from_millis(60)
        } else {
            Duration::from_secs(2)
        };
        let (stream, state) = ready(timeout);
        let original = Instant::now() + Duration::from_millis(100);
        let view = stream.before(if phase == "parent-cap" {
            original + Duration::from_secs(1)
        } else {
            original
        });
        let cancel = CancellationToken::default();
        let mut channel = if phase == "held-channel" || phase == "repeated-views" {
            Some(view.connect(&call.route, call.limits(), &cancel).unwrap())
        } else {
            None
        };
        let second = stream.before(original);
        if phase == "repeated-views" {
            second.connect(&call.route, call.limits(), &cancel).unwrap();
        }
        let before = serde_json::to_value(stream.counters().unwrap()).unwrap();
        wait_until(original + Duration::from_millis(5));
        if let Some(channel) = &mut channel {
            assert_eq!(
                channel.try_send(&call.frame, &cancel),
                Err(TransportError::Partitioned)
            );
        } else {
            assert_eq!(
                view.connect(&call.route, call.limits(), &cancel).err(),
                Some(TransportError::Partitioned)
            );
        }
        assert_eq!(
            view.connect(&call.route, call.limits(), &cancel).err(),
            Some(TransportError::Partitioned)
        );
        assert_eq!(
            second.connect(&call.route, call.limits(), &cancel).err(),
            Some(TransportError::Partitioned)
        );
        assert_eq!(
            stream
                .before(original)
                .connect(&call.route, call.limits(), &cancel)
                .err(),
            Some(TransportError::Partitioned)
        );
        assert_eq!(call.send(&stream), Err(ServiceError::ShuttingDown));
        assert_eq!(
            serde_json::to_value(stream.counters().unwrap()).unwrap(),
            before
        );
        assert_eq!(state.lock().unwrap().written, b"SCMVS001");
        assert!(state.lock().unwrap().shutdowns > 0);
    }
}

#[test]
fn deadline_view_keeps_route_size_cancel_fence_and_owned_view_healthy_send_contracts() {
    let call = Call::new();
    let (stream, state) = ready(Duration::from_secs(2));
    let view = stream.before(Instant::now() + Duration::from_secs(1));
    let cancel = CancellationToken::default();
    let before = serde_json::to_value(stream.counters().unwrap()).unwrap();
    let wrong = route(MailboxMessageKind::Reply);
    assert_eq!(
        view.connect(&wrong, call.limits(), &cancel).err(),
        Some(TransportError::UnknownRoute)
    );
    assert_eq!(state.lock().unwrap().shutdowns, 0);
    let small =
        ChannelLimits::new(call.frame.encoded_len() - 1, call.frame.encoded_len() - 1).unwrap();
    assert_eq!(
        view.connect(&call.route, small, &cancel)
            .unwrap()
            .try_send(&call.frame, &cancel),
        Err(TransportError::FrameTooLarge)
    );
    assert_eq!(
        serde_json::to_value(stream.counters().unwrap()).unwrap(),
        before
    );
    assert_eq!(state.lock().unwrap().shutdowns, 0);
    assert_eq!(call.send(&view), Ok(call.frame.encoded_len()));
    assert_eq!(stream.counters().unwrap().frames_sent, 1);
    let written = state.lock().unwrap().written.clone();
    assert_eq!(
        &written[8..12],
        &(call.frame.encoded_len() as u32).to_be_bytes()
    );
    assert_eq!(&written[12..], call.frame.encoded_bytes());
    cancel.cancel();
    assert_eq!(
        view.connect(&wrong, call.limits(), &cancel).err(),
        Some(TransportError::UnknownRoute)
    );
    assert_eq!(state.lock().unwrap().shutdowns, 0);
    assert_eq!(
        view.connect(&call.route, call.limits(), &cancel).err(),
        Some(TransportError::Cancelled)
    );
    assert!(state.lock().unwrap().shutdowns > 0);
    assert_eq!(call.send(&stream), Err(ServiceError::ShuttingDown));

    // The opaque return must own the Arc, not borrow the temporary handle.
    let (owned, state) = ready(Duration::from_secs(2));
    let surviving = owned.before(Instant::now() + Duration::from_secs(1));
    drop(owned);
    assert_eq!(call.send(&surviving), Ok(call.frame.encoded_len()));
    assert_eq!(
        &state.lock().unwrap().written[12..],
        call.frame.encoded_bytes()
    );
}
