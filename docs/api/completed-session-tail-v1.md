# Owned completed-tail drive and explicit Reply replay v1

Development-witness consumer contract for completed-reply-tail-v1, approved
before implementation. One canonical protocol; no automatic
traffic, consumed-Call retry, closed-stream/crash resume or limit/TTL increase.

## Public owned caller

```rust
pub struct CallerEndpoint<S: SocketIo> { /* owns stream, cancel, Session/broker */ }
pub enum TailDriveOutcome { NoTail, Idle, AckWritten }
impl<S: SocketIo + 'static> CallerEndpoint<S> {
    pub fn new(bootstrap: &Bootstrap, broker: &BrokerContext,
        socket: S, role: HandshakeRole) -> Result<Self>;
    pub fn call(&mut self, request: &Frame) -> Result<(Context, Frame)>;
    pub fn call_with_completed_tail(&mut self, request: &Frame) -> Result<(Context, Frame)>;
    pub fn call_with_completed_tail_and_reconciliation(&mut self, request: &Frame,
        on_call: &mut impl FnMut(ReconciliationSignal) -> Result<()>) -> Result<(Context, Frame)>;
    pub fn drive_completed_tail(&mut self) -> Result<TailDriveOutcome>;
    pub fn counters(&self) -> Result<StreamCounters>;
    pub fn service_report(&self) -> Value;
}
```

new requires Sender bootstrap, reuses canonical Session and handshake, and keeps
one supplied BrokerContext/transport alive. call is strict. Only the explicit
tail methods use charged extraction. Existing sender convenience APIs stay
strict and create no new channels/Idle polling. Callback remains embedding-owned,
per-Call, outside dispatch and carries no payload/authorization. No new native
guest opt-in is implied. Initial qualification is actual Unix TCP/component
relay; native Windows consumer/MVM/WAN roles remain separate.

At completion require current receipt disposition, authenticated Reply and ACK
Sent, then canonical FIFO readiness. Use atomic charged extraction before
returning the application Frame. A Full leaves original PendingCall/wire/payload
owned; do not return payload, resend, allocate a new identity or silently reset.
This slice may refuse/fence until close rather than invent general resume.
Decode failure fences; never return an application result twice.

Store at most one CompletedTail: opaque charged adapter lease, original SendHandle,
original absolute Instant and carried event/ACK counts/state. Drop original Call
payload/prepared/encoded allocations on extraction. The lease charges actual
wire capacity C with original W+C overlap; it is non-Clone/nonserializable and
does not grant a send permit. Strict poll behavior stays unchanged.

## Caller drive and next identity

drive owns the only reader. NoTail performs no I/O. A tail drive validates fresh
both bindings, original mailbox owner, dual-clock caps and original Instant,
then reads one event or returns zero-prefix Idle after250ms. It has no command
timer and sends zero bytes on silence. Complete frames consume the original
cumulative16-event allowance. Controls are canonically authenticated without
receipt-of-receipt and contribute no application result.
Tail ACK writes share a cumulative16-invocation bound, including the original
ACK sent before application completion; burn each invocation before transport.

Only a byte-identical retained normal Reply may enter tail handling; require
current canonical Duplicate admission and the original tracker SendHandle.
Prepare/reserve its actual Duplicate receipt, mark InvokedUncertain before send,
and write outside dispatch through before(originalInstant). Return AckWritten
only for successful local I/O, never RemoteComplete. Keep the tail afterward.
Errors preserve identity/charge, maintain expiry and fence new reads/Calls;
close/Drop releases custody without claiming rollback.

Every next Call retains the old tail until authenticated AcceptedEphemeral/
AcceptedDurable or new-Reply progress for that next identity. Sending/allocating
the next Call and Duplicate alone cannot retire it. Retire before storing the
next Reply wire, preserving one tail slot. Old-tail replay during a current Call
routes to its exact ACK handling and contributes no current facts. An expired
or uncertain unresolved tail fences before allocating another sequence; no FIFO
gap reset. Retained tail ACK precedes a next Call on healthy FIFO TCP; that is
ordering, not new signed remote-completion proof. Existing2048 guard stays.
An already exhausted tail refuses before a next identity is allocated or sent;
while it remains retained, check its event allowance before each new read.

## Explicit receiver replay

```rust
pub struct ReplyReplaySignal { /* private capacity-one SyncSender */ }
impl ReplyReplaySignal {
    pub fn request_replay(&self) -> Result<(), TrySendError<()>>;
}
pub fn receiver_with_broker_and_reply_replay<S: SocketIo + 'static>(
    bootstrap: &Bootstrap, broker: &BrokerContext, cache_root: &Path,
    socket: S, role: HandshakeRole, out: &mut impl Write,
    on_handle: &mut impl FnMut(ReplyReplaySignal) -> Result<()>,
) -> Result<Value>;
```

Private factory creates sync_channel(1) for one receiver handle. Full coalesces;
return/drop disconnects stale signals. Callback runs outside dispatch. Only the
active cached-Reply ACK loop uses receive_or_idle(250ms); no command means no
send. A queued explicit command revalidates the original cached identity, both
bindings, owner, ExchangeDeadline and tracker handle, burns before I/O, then
sends the cached exact Reply. No new Call receipt, object work, sequence or
ACK-of-ACK. Initial send and replays share eight Reply sends; received events
share16 including preamble/Call. Idle consumes no incoming event. Errors retain
the existing receiver-incomplete fence; no resume is added.

The additive adapter prerequisite avoids retaining a second Call ciphertext buffer:

```rust
LocalMailboxService::retry_reply_for_delivery_prepared(
    &mut self, binding: &MailboxBinding, call: &DeliveryHandle, now: ServiceTime,
) -> Result<Option<PreparedMailboxEnvelope>>;
```

The opaque admitted handle selects only its exact service/binding/generation
and original Call entry. Reuse existing retry_reply_prepared's shared accepted
commitment, current incoming/cached-outgoing policy, owner, original Call/Reply
deadline and shutdown checks through one private validator. None still means
known identity/no cache, not application completion. Unknown/foreign/revoked/
expired handles refuse. Existing encoded-Call retry semantics remain unchanged.
No arbitrary charge or alternate proof API; no new retained request wire.

Cache lifetime correction: derive its owner-monotonic deadline once at original
reply_prepared using existing binding.deadlines. Both retry APIs enforce that
cap plus cache signed Unix expiry and original Call caps. Copies/retries never
refresh it. This intentionally fixes the old wire-based API's ability to replay
a shorter-lived Reply past its original monotonic cap after wall rollback.

## RED/qualification gates and accounting

First public missing-API RED, then actual post-completion final-ACK loss:
caller returns once and remains owned; receiver remains cached; explicit replay
and tail drive yield an actual Duplicate ACK, cache release once and next valid
Call. Execute direct TCP and the actual duplex relay, with exact object readbacks,
same Reply bytes, real counters and no extra object execution. No command/Idle
must emit zero bytes. Repeat withholding within carried event/send bounds; stale
signals, different lineage/wire, expiry/revocation and blocked ACK must refuse
under original write cap. Final max-Call tail drive must not relax2048 admission.

Counters use checked arithmetic; report tail-drive events/ACK invocations/writes
and explicit receiver replay commands alongside existing actual stream counters.
Receiver processing includes replay crypto/send preparation and failed cleanup,
excludes idle waiting; local processing totals overlap per-operation domains.
No performance, energy, WAN or native recovery claim follows from this slice.

Initial executed component qualification includes full-object public TCP calls
and the actual bounded duplex relay, withheld final ACK after application
return, explicit byte-identical Reply replay, local ACK write, receiver cache
release and subsequent exact object readbacks. Negative gates cover fresh
generation/expiry refusal, carried event exhaustion before another identity,
eight total receiver Reply writes, and partial ACK write expiry retaining the
tail/fence. The partial-write fixture is bounded SocketIo conformance, not a
measurement of OS/TCP congestion. Native backend, closed-stream/crash resume,
installed aggregate capacity and >2048-session qualification remain open.
