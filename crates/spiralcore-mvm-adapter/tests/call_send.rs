#![cfg(feature = "live")]
mod common;
#[path = "common/service.rs"]
mod fixture;

use mvm_contract::protocol::mailbox::MailboxMessageKind;
use mvm_core::mailbox::{
    encode_envelope,
    transport::{
        AuthorizedMailboxFrame, AuthorizedRoute, CancellationToken, ChannelLimits,
        InMemoryPeerTransport, PeerChannel, PeerTransport, PreparedMailboxEnvelope, TransportError,
    },
};
use spiralcore_mvm_adapter::{
    service::{
        CallHandle, CallStatus, HostBudget, LocalMailboxService, MailboxBinding, ServiceError,
        ServiceLimits, ServiceTime,
    },
    service_sender::{self, MessageOptions},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn at(delta: u64) -> ServiceTime {
    ServiceTime {
        unix_ms: fixture::time().unix_ms + delta as i64,
        monotonic_ms: fixture::time().monotonic_ms + delta,
    }
}

const RESPONSE: &[u8] = b"retained\0reply\xff";

struct Registered {
    budget: HostBudget,
    service: LocalMailboxService,
    binding: MailboxBinding,
    prepared: PreparedMailboxEnvelope,
    call: CallHandle,
    options: MessageOptions,
    wire: Vec<u8>,
}

impl Registered {
    fn new(with_reply: bool) -> Self {
        let budget = HostBudget::new(ServiceLimits::default()).unwrap();
        let mut service = LocalMailboxService::new(budget.clone()).unwrap();
        let binding = fixture::binding(MailboxMessageKind::Call, true, false, true);
        let mut options = MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
        options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
        let prepared =
            service_sender::prepare(&binding, b"original\0call", &options, at(0)).unwrap();
        let wire = encode_envelope(prepared.envelope()).unwrap();
        let call = service.register_call(&binding, &wire, at(0)).unwrap();
        if with_reply {
            let mut receiver =
                LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap())
                    .unwrap();
            let incoming = fixture::binding(MailboxMessageKind::Call, true, false, false);
            receiver.admit(&incoming, &wire, at(0)).unwrap();
            let proof = receiver.receive(&incoming, at(0)).unwrap().unwrap();
            let replies = fixture::binding(MailboxMessageKind::Reply, false, false, true);
            let reply_options = MessageOptions::for_reply(&proof, at(7)).unwrap();
            let reply = receiver
                .reply_prepared(
                    &replies,
                    proof.handle(),
                    service_sender::request(&replies, RESPONSE, &reply_options, at(7)).unwrap(),
                    at(7),
                )
                .unwrap();
            let incoming_reply = fixture::binding(MailboxMessageKind::Reply, false, false, false);
            service
                .admit(
                    &incoming_reply,
                    &encode_envelope(reply.envelope()).unwrap(),
                    at(7),
                )
                .unwrap();
        }
        Self {
            budget,
            service,
            binding,
            prepared,
            call,
            options,
            wire,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RetainedState {
    bytes: u64,
    messages: usize,
    active: usize,
    identities: usize,
    owners: usize,
    accepted: usize,
    calls: usize,
    scopes: usize,
    entries: usize,
}
fn retained(f: &Registered) -> RetainedState {
    let b = f.budget.snapshot().unwrap();
    let s = f.service.snapshot();
    RetainedState {
        bytes: b.queued_bytes,
        messages: b.queued_messages,
        active: b.active_calls,
        identities: b.identities,
        owners: b.owner_count,
        accepted: s.accepted,
        calls: s.calls,
        scopes: s.fifo_scopes,
        entries: s.fifo_entries,
    }
}

// Observe the actual canonical transport seam; an empty queue alone cannot prove
// that validation refused a connection. Faults never introduce a raw-byte path.
struct CountedTransport {
    memory: InMemoryPeerTransport,
    connects: AtomicUsize,
    sends: Arc<AtomicUsize>,
    connect_error: Option<TransportError>,
    send_error: Option<TransportError>,
    write_before_error: bool,
}
impl CountedTransport {
    fn new() -> Self {
        let memory = InMemoryPeerTransport::default();
        memory
            .register_route(
                common::config(MailboxMessageKind::Call)
                    .binding
                    .claims
                    .route_id,
            )
            .unwrap();
        Self {
            memory,
            connects: AtomicUsize::new(0),
            sends: Arc::new(AtomicUsize::new(0)),
            connect_error: None,
            send_error: None,
            write_before_error: false,
        }
    }
    fn drain(&self) -> Vec<Vec<u8>> {
        self.memory
            .drain(
                common::config(MailboxMessageKind::Call)
                    .binding
                    .claims
                    .route_id,
            )
            .unwrap()
    }
}
impl PeerTransport for CountedTransport {
    fn connect(
        &self,
        route: &AuthorizedRoute,
        limits: ChannelLimits,
        cancel: &CancellationToken,
    ) -> Result<Box<dyn PeerChannel>, TransportError> {
        self.connects.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = &self.connect_error {
            return Err(error.clone());
        }
        Ok(Box::new(CountedChannel {
            inner: self.memory.connect(route, limits, cancel)?,
            sends: self.sends.clone(),
            error: self.send_error.clone(),
            write_before_error: self.write_before_error,
        }))
    }
}
struct CountedChannel {
    inner: Box<dyn PeerChannel>,
    sends: Arc<AtomicUsize>,
    error: Option<TransportError>,
    write_before_error: bool,
}
impl PeerChannel for CountedChannel {
    fn try_send(
        &mut self,
        frame: &AuthorizedMailboxFrame,
        cancel: &CancellationToken,
    ) -> Result<(), TransportError> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        if self.error.is_none() || self.write_before_error {
            self.inner.try_send(frame, cancel)?;
        }
        self.error.clone().map_or(Ok(()), Err)
    }
}

#[test]
fn pending_call_retry_is_exact_without_advancing_local_or_shared_accounting_clock() {
    let mut f = Registered::new(false);
    let mut other = LocalMailboxService::new(f.budget.clone()).unwrap();
    other.expire(at(2)).unwrap();
    let before = retained(&f);
    let transport = CountedTransport::new();
    let cancel = CancellationToken::default();
    for time in [at(1), at(3)] {
        assert_eq!(
            f.service.retry_call_prepared(
                &f.binding,
                &f.call,
                &f.prepared,
                &transport,
                &cancel,
                time
            ),
            Ok(f.wire.len())
        );
        assert_eq!(transport.drain(), [f.wire.clone()]);
        assert_eq!(retained(&f), before);
    }
    assert_eq!(transport.connects.load(Ordering::SeqCst), 2);
    assert_eq!(transport.sends.load(Ordering::SeqCst), 2);
    // These mutable operations would reject time 2 if retry at time 3 had
    // advanced either the shared accounting clock or this service's clock.
    other.expire(at(2)).unwrap();
    assert_eq!(
        f.service.poll_call(&f.binding, &f.call, at(2)).unwrap(),
        CallStatus::Pending
    );
    assert_eq!(retained(&f), before);
}

#[test]
fn registered_call_retry_refusals_precede_connect_and_preserve_retained_state() {
    for with_reply in [false, true] {
        for mode in [
            "generation",
            "new-request",
            "same-identity-payload",
            "foreign-binding",
            "foreign-service",
            "cancel-token",
            "cancel-call",
            "shutdown",
            "unix-expiry",
            "unix-past",
            "monotonic-expiry",
            "monotonic-past",
            "clock",
            "negative-unix",
        ] {
            // Cancelling an already replied Call is a consuming poll in the
            // existing lifecycle API, not a Cancelled status transition.
            if with_reply && mode == "cancel-call" {
                continue;
            }
            let mut f = Registered::new(with_reply);
            let transport = CountedTransport::new();
            let cancel = CancellationToken::default();
            let mut time = at(8);
            let foreign_service = LocalMailboxService::new(f.budget.clone()).unwrap();
            let foreign_binding = fixture::binding(MailboxMessageKind::Call, true, false, true);
            let expected = match mode {
                "generation" => {
                    let mut policy = common::config(MailboxMessageKind::Call)
                        .policy
                        .snapshot()
                        .unwrap();
                    policy.accepted_content_types = vec![fixture::content_type()];
                    f.binding.replace_policy(policy).unwrap();
                    ServiceError::Unauthorized
                }
                "new-request" => {
                    let mut options =
                        MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
                    options.reply_to = f.options.reply_to;
                    f.prepared =
                        service_sender::prepare(&f.binding, b"different", &options, at(0)).unwrap();
                    ServiceError::Conflict
                }
                "same-identity-payload" => {
                    f.prepared =
                        service_sender::prepare(&f.binding, b"different", &f.options, at(0))
                            .unwrap();
                    ServiceError::Conflict
                }
                "foreign-binding" | "foreign-service" => ServiceError::Unauthorized,
                "cancel-token" => {
                    cancel.cancel();
                    ServiceError::Cancelled
                }
                "cancel-call" => {
                    f.service.cancel(&f.binding, &f.call, at(0)).unwrap();
                    ServiceError::Cancelled
                }
                "shutdown" => {
                    f.service.shutdown(time).unwrap();
                    ServiceError::ShuttingDown
                }
                "unix-expiry" => {
                    time.unix_ms = 2000;
                    ServiceError::Expired
                }
                "unix-past" => {
                    time.unix_ms = 2001;
                    ServiceError::Expired
                }
                "monotonic-expiry" => {
                    time.monotonic_ms = 600;
                    ServiceError::Expired
                }
                "monotonic-past" => {
                    time.monotonic_ms = 601;
                    ServiceError::Expired
                }
                "clock" => {
                    time.monotonic_ms = if with_reply { 106 } else { 99 };
                    ServiceError::Clock
                }
                "negative-unix" => {
                    time.unix_ms = -1;
                    ServiceError::Clock
                }
                _ => unreachable!(),
            };
            let before = retained(&f);
            let service = if mode == "foreign-service" {
                &foreign_service
            } else {
                &f.service
            };
            let binding = if mode == "foreign-binding" {
                &foreign_binding
            } else {
                &f.binding
            };
            assert_eq!(
                service.retry_call_prepared(
                    binding,
                    &f.call,
                    &f.prepared,
                    &transport,
                    &cancel,
                    time
                ),
                Err(expected),
                "{mode}, reply={with_reply}"
            );
            assert_eq!(transport.connects.load(Ordering::SeqCst), 0, "{mode}");
            assert_eq!(transport.sends.load(Ordering::SeqCst), 0, "{mode}");
            assert!(transport.drain().is_empty(), "{mode}");
            assert_eq!(retained(&f), before, "{mode}, reply={with_reply}");
        }
    }
}

#[test]
fn failed_call_retry_keeps_original_request_and_charged_reply_even_after_uncertain_write() {
    for mode in [
        "refuse",
        "partition",
        "backpressure",
        "cancelled-send",
        "uncertain-write",
    ] {
        let mut f = Registered::new(true);
        let before = retained(&f);
        assert!(before.bytes > 0);
        let mut transport = CountedTransport::new();
        let expected = match mode {
            "refuse" => {
                transport.connect_error = Some(TransportError::Refused);
                ServiceError::Unavailable
            }
            "partition" => {
                transport.connect_error = Some(TransportError::Partitioned);
                ServiceError::Unavailable
            }
            "backpressure" => {
                transport.send_error = Some(TransportError::Backpressured);
                ServiceError::Backpressured
            }
            "cancelled-send" => {
                transport.send_error = Some(TransportError::Cancelled);
                ServiceError::Cancelled
            }
            "uncertain-write" => {
                transport.send_error = Some(TransportError::Partitioned);
                transport.write_before_error = true;
                ServiceError::Unavailable
            }
            _ => unreachable!(),
        };
        let cancel = CancellationToken::default();
        assert_eq!(
            f.service.retry_call_prepared(
                &f.binding,
                &f.call,
                &f.prepared,
                &transport,
                &cancel,
                at(8)
            ),
            Err(expected),
            "{mode}"
        );
        assert_eq!(transport.connects.load(Ordering::SeqCst), 1);
        assert_eq!(
            transport.sends.load(Ordering::SeqCst),
            usize::from(transport.connect_error.is_none())
        );
        let transmitted = transport.drain();
        if mode == "uncertain-write" {
            assert_eq!(transmitted, [f.wire.clone()]);
        } else {
            assert!(transmitted.is_empty());
        }
        assert_eq!(retained(&f), before, "{mode}");
        transport.connect_error = None;
        transport.send_error = None;
        assert_eq!(
            f.service.retry_call_prepared(
                &f.binding,
                &f.call,
                &f.prepared,
                &transport,
                &cancel,
                at(8)
            ),
            Ok(f.wire.len())
        );
        assert_eq!(transport.drain(), [f.wire.clone()]);
        assert_eq!(retained(&f), before);
        assert_eq!(
            f.service.poll_call(&f.binding, &f.call, at(8)).unwrap(),
            CallStatus::Replied(Some(RESPONSE.to_vec()))
        );
        assert_eq!(f.budget.snapshot().unwrap().queued_bytes, 0);
    }
}

#[test]
fn registered_call_retry_preserves_unconsumed_canonical_reply_and_exact_request() {
    let budget = HostBudget::new(ServiceLimits::default()).unwrap();
    let mut caller = LocalMailboxService::new(budget.clone()).unwrap();
    let calls = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let mut options = MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared = service_sender::prepare(&calls, b"original\0call", &options, at(0)).unwrap();
    let wire = encode_envelope(prepared.envelope()).unwrap();
    let call = caller.register_call(&calls, &wire, at(0)).unwrap();

    // A separate real receiver admits the Call and prepares its authorized Reply.
    let mut receiver =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let incoming = fixture::binding(MailboxMessageKind::Call, true, false, false);
    receiver.admit(&incoming, &wire, at(0)).unwrap();
    let proof = receiver.receive(&incoming, at(0)).unwrap().unwrap();
    let replies = fixture::binding(MailboxMessageKind::Reply, false, false, true);
    let response = b"retained\0reply\xff";
    let reply_options = MessageOptions::for_reply(&proof, at(0)).unwrap();
    let reply = receiver
        .reply_prepared(
            &replies,
            proof.handle(),
            service_sender::request(&replies, response, &reply_options, at(0)).unwrap(),
            at(0),
        )
        .unwrap();
    let reply_wire = encode_envelope(reply.envelope()).unwrap();
    let incoming_reply = fixture::binding(MailboxMessageKind::Reply, false, false, false);
    caller.admit(&incoming_reply, &reply_wire, at(0)).unwrap();
    let charged = budget.snapshot().unwrap();
    assert_eq!(charged.queued_bytes, reply_wire.len() as u64);
    assert_eq!(charged.queued_messages, 1);
    assert_eq!(charged.active_calls, 0);
    let before = caller.snapshot();

    let transport = InMemoryPeerTransport::default();
    let route = common::config(MailboxMessageKind::Call)
        .binding
        .claims
        .route_id;
    transport.register_route(route).unwrap();
    let cancel = CancellationToken::default();
    assert_eq!(
        caller.send_call_prepared(&calls, &call, &prepared, &transport, &cancel, at(1)),
        Err(ServiceError::Conflict)
    );
    assert!(transport.drain(route).unwrap().is_empty());
    assert_eq!(
        caller.retry_call_prepared(&calls, &call, &prepared, &transport, &cancel, at(1)),
        Ok(wire.len())
    );
    assert_eq!(transport.drain(route).unwrap(), [wire]);
    let retained = budget.snapshot().unwrap();
    assert_eq!(retained.queued_bytes, charged.queued_bytes);
    assert_eq!(retained.queued_messages, charged.queued_messages);
    assert_eq!(retained.active_calls, charged.active_calls);
    assert_eq!(retained.identities, charged.identities);
    let after = caller.snapshot();
    assert_eq!(after.calls, before.calls);
    assert_eq!(after.accepted, before.accepted);
    assert_eq!(after.fifo_scopes, before.fifo_scopes);
    assert_eq!(after.fifo_entries, before.fifo_entries);
    assert_eq!(
        caller.poll_call(&calls, &call, at(1)).unwrap(),
        CallStatus::Replied(Some(response.to_vec()))
    );
    assert_eq!(budget.snapshot().unwrap().queued_bytes, 0);
    assert_eq!(budget.snapshot().unwrap().queued_messages, 0);
    assert_eq!(
        caller.poll_call(&calls, &call, at(1)).unwrap(),
        CallStatus::Replied(None)
    );
    assert_eq!(
        caller.retry_call_prepared(&calls, &call, &prepared, &transport, &cancel, at(1)),
        Err(ServiceError::Conflict)
    );
    assert!(transport.drain(route).unwrap().is_empty());
}

#[test]
fn registered_call_send_is_exact_and_does_not_advance_other_services_accounting_clock() {
    let budget = HostBudget::new(ServiceLimits::default()).unwrap();
    let mut service = LocalMailboxService::new(budget.clone()).unwrap();
    let binding = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let mut options = MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared = service_sender::prepare(&binding, b"exact\0call\0", &options, at(0)).unwrap();
    let wire = encode_envelope(prepared.envelope()).unwrap();
    let call = service.register_call(&binding, &wire, at(0)).unwrap();
    let mut other = LocalMailboxService::new(budget).unwrap();
    other.expire(at(2)).unwrap();
    let transport = InMemoryPeerTransport::default();
    let route = common::config(MailboxMessageKind::Call)
        .binding
        .claims
        .route_id;
    transport.register_route(route).unwrap();
    assert_eq!(
        service
            .send_call_prepared(
                &binding,
                &call,
                &prepared,
                &transport,
                &CancellationToken::default(),
                at(1)
            )
            .unwrap(),
        wire.len()
    );
    assert_eq!(transport.drain(route).unwrap(), [wire]);
}

#[test]
fn registered_call_send_rejects_changed_generation_request_lifecycle_or_time_before_transmission() {
    for mode in [
        "generation",
        "request",
        "cancel",
        "shutdown",
        "unix-expiry",
        "monotonic-expiry",
        "clock",
        "foreign-service",
    ] {
        let budget = HostBudget::new(ServiceLimits::default()).unwrap();
        let mut service = LocalMailboxService::new(budget.clone()).unwrap();
        let binding = fixture::binding(MailboxMessageKind::Call, true, false, true);
        let mut options = MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
        options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
        let mut prepared = service_sender::prepare(&binding, b"original", &options, at(0)).unwrap();
        let call = service
            .register_call(
                &binding,
                &encode_envelope(prepared.envelope()).unwrap(),
                at(0),
            )
            .unwrap();
        let transport = InMemoryPeerTransport::default();
        let route = common::config(MailboxMessageKind::Call)
            .binding
            .claims
            .route_id;
        transport.register_route(route).unwrap();
        let mut time = at(1);
        let expected = match mode {
            "generation" => {
                let mut policy = common::config(MailboxMessageKind::Call)
                    .policy
                    .snapshot()
                    .unwrap();
                policy.accepted_content_types = vec![fixture::content_type()];
                binding.replace_policy(policy).unwrap();
                ServiceError::Unauthorized
            }
            "request" => {
                let mut different =
                    MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
                different.reply_to = options.reply_to;
                prepared =
                    service_sender::prepare(&binding, b"different", &different, at(0)).unwrap();
                ServiceError::Conflict
            }
            "cancel" => {
                service.cancel(&binding, &call, at(0)).unwrap();
                ServiceError::Cancelled
            }
            "shutdown" => {
                service.shutdown(at(0)).unwrap();
                ServiceError::ShuttingDown
            }
            "unix-expiry" => {
                time.unix_ms = 2000;
                ServiceError::Expired
            }
            "monotonic-expiry" => {
                time.monotonic_ms = 600;
                ServiceError::Expired
            }
            "clock" => {
                time.monotonic_ms = 99;
                ServiceError::Clock
            }
            "foreign-service" => {
                service = LocalMailboxService::new(budget).unwrap();
                ServiceError::Unauthorized
            }
            _ => unreachable!(),
        };
        assert_eq!(
            service.send_call_prepared(
                &binding,
                &call,
                &prepared,
                &transport,
                &CancellationToken::default(),
                time
            ),
            Err(expected),
            "{mode}"
        );
        assert!(transport.drain(route).unwrap().is_empty(), "{mode}");
    }
}
