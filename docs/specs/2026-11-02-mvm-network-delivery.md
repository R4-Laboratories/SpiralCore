# MVM networking delivery through canonical MVM services

Date: 2026-11-02. Decision: SpiralCore implementation direction following the
user-approved NIC-less host-mediated design. This does not activate an upstream
MVM service or supersede an MVM ADR.

## Authority and scope

The implementation follows accepted MVM ADR-042 (one authenticated FlowMux
network path), ADR-057 (canonical capability-bound mailbox semantics), and
ADR-056 (owner-mediated application access). The live upstream review identifies
`6ae461a54d85a1b70477c1efb97ea4a33ef3b331`; the existing native SpiralCore witness
uses retained MVM source `3104a12c5b071a6851323fa8115d2afaf3f0bb8b` plus its
recorded patch. Portability and dependency identity are checked before changing
that pin; current source is not silently substituted into a measured binary.
The narrow source comparison found canonical mailbox verification and ADRs
042/056/057 unchanged between those revisions; canonical mailbox DTO changes
only expose existing private base64 helpers. The newer transport resource ledger
bounds peer/stream/retry/relay work and does not implement local
tenant/mailbox/sender queue admission. This slice therefore retains the existing
source pin and verifies its required behavior instead of repinning six crates.

MVM #4629 remains the dedicated local mailbox-service implementation contract;
#4632 provides the canonical transport interfaces; #4633 specifies Iroh as a
replaceable ciphertext carrier. #4607 is a research/design umbrella and does
not authorize a new guest NIC or parallel production data plane. Accepted ADRs
are project architecture authority. Any external standards approval is recorded
only against the actual supplied approval artifact and its scope; it is not
inferred from an ADR status or a functional test.

The delivery has three independently testable surfaces:

| Surface | Canonical MVM owner | SpiralCore contribution |
|---|---|---|
| Immutable object mailbox | Dedicated service, capability/envelope/receipt, activation and admission | Existing object-v1 codec before encryption and after verified decryption; bounded local service integration |
| TCP/UDP network flows | Authenticated FlowMux 5253 and per-VM network endpoint, signed plan, DNS, transforms, rate/audit | Host-mediated cooperative encoding only after the canonical decision, with explicit endpoint mapping |
| Owner access | Existing DevOnly access or declared typed application operations | Carry admitted operations through the same service boundaries; no sealed shell or general exec |

There remains one SpiralCore object encoding/state-machine family. Different
endpoint semantics are specified explicitly; mailbox acceptance, object
publication, stream delivery and application execution are never interchangeable.

## First implementation: bounded canonical local mailbox service

Implement the standalone local service first, then the FlowMux carrier and
access modes. The first service is ephemeral: it may issue canonical
`AcceptedEphemeral`, never `AcceptedDurable` without an encrypted journal and
durable replay/receipt transaction.

The service consumes `mvm_contract::protocol::mailbox` values and uses
`mvm_core::mailbox` verification and `sign_receipt`/`verify_receipt`. It creates
no replacement mailbox envelope, signature domain, receipt DTO, or `mvmd`
dependency. Existing object-v1 `application/spiralcore-object` payloads dispatch
only through the shared profile-enforcing endpoint after canonical verified open.

Local admission is one serialized transaction:

1. Validate outer bounds and current route, tenant, recipient, boot/execution
   epoch, capability, content type, time and policy using canonical verification.
2. Stage replay observation without permanently accepting the message. The
   canonical `ReplayProtector` hook currently runs before queue admission; a
   transactional implementation must not turn a refused/full message into an
   accepted duplicate or let an unauthenticated message create state.
3. Check the authenticated stable message identity and immutable commitment.
   Authenticated duplicates do not enqueue or dispatch twice. Conflicting content
   under an existing identity is refused. Reconciliation uses the canonical
   receipt vocabulary rather than an invented ACK.
4. Reserve queue bytes/depth and outstanding-call state against one shared host
   budget plus narrower tenant/mailbox/sender limits. Multiple service instances
   cannot each allocate the advertised host maximum. Check arithmetic before
   allocation. A full/refused operation leaves no accepted queue/dedup entry.
5. Commit owned message, dedup state and signed destination acceptance together.
   Only the configured destination authority signs its receipt. Transport
   connection, host relay, object Published and sender enqueue cannot sign or
   stand in for destination acceptance.

`call` returns a correlated application reply or typed timeout/cancellation.
Its monotonic deadline is derived from current signed expiry. Terminal
correlation state prevents a late reply from satisfying a later call. A timeout
does not promise the handler did not execute; cancellation is not rollback.
`cast` returns after the declared local admission and produces no application
reply; it is not a lossy overload mode. Object-v1 repair/publication exchanges
continue to use calls because cache promotion requires verified replies.

Keep bounded terminal dedup/correlation state through the declared expiry and
retention window. Queue consumption does not permit an accepted duplicate to
execute again. State limits fail closed; do not evict a live identity to admit a
new one. Shutdown stops new admission and disposes accepted ephemeral state
through its named epoch; reconnect requires current authorization, not reused
vsock CID authority.

The typed guest service must have its own registration, framing, listener,
queues, credits and worker budget. Existing experimental port 5257 is not
declared an upstream production registration. The retained opt-in native patch
already supplies `GuestService::Mailbox`, its port and independent channel
limits; reuse and verify that binding instead of adding another registration.
Extend its queue/service hook only where required, while default activation
stays disabled pending MVM owner adoption. Sharing machine-control, FlowMux,
telemetry or workload-exit queues is prohibited.

The inspected canonical contract defines `SignedDestinationReceipt` and its
signature/verification functions, but does not specify a receipt transport frame
or content type. The user has authorized SpiralCore to implement this missing
MVM-side binding for upstream review. The extension below is a proposed MVM
service mapping, not a claim that it was previously standardized or adopted.
A bare receipt must not cross a keyless relay: it contains inner message/address
identities. The original object `SCMOBJ01` frames remain unchanged.

## Proposed canonical encrypted receipt carrier

Define the reserved content type `application/vnd.mvm.mailbox.receipt`, version
1, in `mvm-contract::protocol::mailbox`, with a 4,096-byte maximum payload. The
payload is the existing `SignedDestinationReceipt`, encoded as canonical JCS
JSON. Add bounded canonical encode/decode helpers in `mvm-core::mailbox`:

```rust
pub fn encode_destination_receipt(receipt: &SignedDestinationReceipt)
    -> Result<Vec<u8>, MailboxError>;
pub fn decode_destination_receipt(bytes: &[u8])
    -> Result<SignedDestinationReceipt, MailboxError>;
```

Decode checks the byte bound before deserialization, canonical versions and
field/signature bounds, rejects unknown fields, and requires re-encoding to
match the exact input bytes. These helpers introduce no signature domain or
replacement receipt structure: `sign_receipt` and `verify_receipt` remain the
canonical authority. A cryptographically valid signature is insufficient until
the expected outstanding request and recipient key binding also match.

Carry that payload in an ordinary canonical encrypted `MailboxEnvelope` with
kind `Cast`, `reply_to=None`, and the reserved receipt content type. Its sender
is the original destination authority; its recipient is the original sender's
explicitly configured receipt mailbox. The envelope has its own stable message
ID, ordering sequence, nonce and bounded expiry, preserving these inner fields
when retried. Set causation to the original request's MessageId and correlation
to its CorrelationId. The application Call still receives its one independent
canonical application Reply. A receipt Cast neither replaces that Reply nor
adds bytes to an `SCMOBJ01` frame.

Each receive binding has an explicit trusted `receipt_return` configuration:
reverse canonical Send/Cast capability restricted to the reserved content type,
reverse transport binding, sender signing key, recipient encryption context,
expected receipt mailbox and bounded control limits. Match tenant/domain,
principal and current policy/epoch to the original authenticated relationship.
An ordinary Cast has no `reply_to`; never invent its receipt route from a
received payload, arbitrary address or transport connection. A native binding
requiring sender-observed receipts refuses startup when the reverse grant is
absent. Both peers pin this service profile through trusted configuration; this
adds no unauthenticated negotiation or fallback to the old local-only mode.

The reserved content type terminates in a dedicated bounded infrastructure
handler after canonical envelope verification. It never enters an application
queue, invokes object decoding or triggers an application callback. It performs
canonical receipt decode/signature verification and updates the matching
outbound tracker idempotently. **It never emits a receipt for a receipt.** This
is an explicit service control-class rule, not a lossy mode for ordinary Casts.
All ordinary application Casts retain normal local admission and destination
outcomes. Receipt-control data is not accepted merely because a sender names
the reserved type: its separate grant and known outstanding request are required.

Reserve mailbox-owned control capacity independently of application queues so
an application Full outcome can still be sent: host 1 MiB / 256 queued control
items; tenant 512 KiB / 128; receipt mailbox 256 KiB / 64; sender 128 KiB / 32.
Intersect signed grant limits and charge actual encoded control-envelope bytes.
Each receipt envelope is at most 16,384 bytes. These are reservations inside
the mailbox service's configured aggregate owner, not borrowed FlowMux/control/
telemetry capacity and not extra independent allocations per service instance.
With the receipt profile enabled, subtract these byte/item reservations from
the total application admission allowance (for example the 16-MiB/1,024-item
host total leaves at most 15 MiB/768 application items). Smaller custom bounds
must reserve consistent positive control capacity or refuse this profile.
Check control capacity before application acceptance; if no
response can be owned, apply bounded backpressure without accepting the message.
After acceptance, a transport failure leaves ownership uncertain at the sender;
bounded resend of the same receipt or request identity reconciles it. Never
claim a failed send rolled back destination acceptance.

The source outbound tracker retains, for each bounded attempt, the request's
stable MessageId, destination address/epoch, delivery ID and ciphertext
commitment. Verification checks all these fields, the expected destination
receipt key, receipt-envelope sender/recipient/content type, policy and expiry.
Receipt causation and correlation must also match the tracked original request.
Retries may have distinct outer delivery IDs/ciphertext but identical inner
request semantics; retain only the configured maximum eight attempt bindings.
A signed receipt for an unknown request, wrong attempt, tenant, recipient,
epoch or ciphertext cannot update delivery state. Duplicate receipts are
idempotent and cannot satisfy a later call. A canonical `Duplicate` outcome does
not itself imply durable acceptance; original receipt/disposition remains
separately recorded. Application reply and admission observation can arrive in
either order and remain independently inspectable.

Extend the local API with a bounded send tracker and receipt carrier helpers:

```rust
pub struct SendHandle { /* private source/epoch/message/attempt identity */ }
pub struct ReceiptReturnConfig { /* reverse canonical binding/grant and keys */ }
pub enum ReceiptObservation { Pending, Verified(SignedDestinationReceipt), Expired }
impl LocalMailboxService {
    pub fn register_send(&mut self, binding: &MailboxBinding, encoded: &[u8],
                         now: ServiceTime) -> Result<SendHandle, ServiceError>;
    pub fn observe_receipt(&mut self, binding: &MailboxBinding,
                           encoded_receipt_envelope: &[u8], now: ServiceTime)
                           -> Result<(), ServiceError>;
    pub fn take_receipt(&mut self, binding: &MailboxBinding, send: &SendHandle,
                        now: ServiceTime) -> Result<ReceiptObservation, ServiceError>;
}
```

`register_call` additionally tracks the application reply; `register_send` also
supports ordinary Cast without allocating an application-call slot. Existing
send handles and call handles refer to the same request/attempt record rather
than duplicating quota. Receipt payload consumption frees its byte charge, while
the bounded terminal identity remains. Infrastructure replay handling records
only authenticated matches and refuses unknown control messages without
unbounded allocation. A valid old receipt is harmlessly idempotent for its own
terminal record and cannot resurrect an expired owner or replace another call.

The native client demultiplexes authenticated content types while awaiting its
application Reply. The server owns and sends the signed receipt Cast separately,
then dispatches an accepted application message and sends its application Reply.
Queue order may optimize the immediate case but is not the correlation contract.
Both directions count receipt bytes, crypto/processing CPU and elapsed time.
Tests cover receipt-before-reply, reply-before-receipt, lost receipt/retry,
duplicate/conflicting receipt, receipt replay/late delivery, receipt queue full,
invalid content type/signature/correlation, shutdown and zero recursive traffic.
Only encrypted envelope bytes cross the keyless host relay.

## Phase A public Rust API contract

The following is the required service surface; helper names may be adjusted in
the same reviewed change, but authority, ownership and outcomes may not weaken.
All types here except canonical MVM envelopes/receipts are local Rust API types,
not additional wire DTOs. There is no JSON metadata exchange or new signature
domain. `ServiceError` and `CallStatus` are local typed observations.

```rust
pub struct ServiceTime { pub unix_ms: i64, pub monotonic_ms: u64 }
pub struct ServiceLimits { /* checked positive ceilings described below */ }
pub struct HostBudget { /* Arc-owned private shared accounting */ }
pub struct ServiceBindingConfig { /* canonical OpenContext, grants/binding, keys */ }
pub struct MailboxBinding { /* trusted policy, canonical grant/binding and keys */ }
pub struct DestinationReceiptSigner { /* explicit recipient identity + signing key */ }
pub struct DeliveryHandle { /* private service generation + immutable identity */ }
pub struct CallHandle { /* private caller/epoch/correlation identity */ }
pub struct AdmittedMessage { /* private verified header + payload + handle */ }

impl HostBudget {
    pub fn new(limits: ServiceLimits) -> Result<Self, ServiceError>;
}
impl MailboxBinding {
    pub fn new(config: ServiceBindingConfig,
               destination_receipt_signer: Option<DestinationReceiptSigner>)
               -> Result<Self, ServiceError>;
    pub fn replace_policy(&mut self, context: OpenContext) -> Result<(), ServiceError>;
}
impl LocalMailboxService {
    pub fn new(budget: HostBudget) -> Result<Self, ServiceError>;
    pub fn admit(&mut self, binding: &MailboxBinding, encoded: &[u8],
                 now: ServiceTime) -> Result<SignedDestinationReceipt, ServiceError>;
    pub fn receive(&mut self, binding: &MailboxBinding,
                   now: ServiceTime) -> Result<Option<AdmittedMessage>, ServiceError>;
    pub fn register_call(&mut self, binding: &MailboxBinding,
                         encoded_envelope: &[u8], now: ServiceTime)
                         -> Result<CallHandle, ServiceError>;
    pub fn poll_call(&mut self, binding: &MailboxBinding, call: &CallHandle,
                     now: ServiceTime) -> Result<CallStatus, ServiceError>;
    pub fn cancel(&mut self, binding: &MailboxBinding, call: &CallHandle,
                  now: ServiceTime) -> Result<CallStatus, ServiceError>;
    pub fn reply(&mut self, binding: &MailboxBinding, call: &DeliveryHandle,
                 request: SealRequest<'_>, now: ServiceTime)
                 -> Result<MailboxEnvelope, ServiceError>;
    pub fn dispatch_object(&mut self, binding: &MailboxBinding,
                           message: AdmittedMessage, endpoint: &mut ObjectEndpoint,
                           now: ServiceTime) -> Result<Frame, ServiceError>;
    pub fn expire(&mut self, now: ServiceTime) -> Result<usize, ServiceError>;
    pub fn shutdown(&mut self, now: ServiceTime) -> Result<(), ServiceError>;
}
impl AdmittedMessage {
    pub fn header(&self) -> &MailboxEnvelopeHeader;
    pub fn payload(&self) -> &[u8];
    pub fn handle(&self) -> &DeliveryHandle;
}
```

`ServiceBindingConfig` is trusted local configuration containing canonical
`OpenContext`, signed mailbox capability, signed transport binding, encryption
key/context, sender verification key, authority verification key, and an optional
local sender signing key. The primary constructor validates those canonical
identity/grant/route facts and the separately supplied destination signer. It
must not take policy or keys from received payloads. It owns a small private
generation-fenced policy; `replace_policy` revalidates the canonical context and
increments that generation before old proofs or handles can act. Generation
overflow fails closed.

Call/Cast use the exact canonical Send operation; Reply uses the exact canonical
Reply operation and independent reply capability. The existing pilot
`DirectionConfig`/`Policy` is Send-only and therefore is not the primary service
configuration. Leave that API unchanged; an optional `from_direction` helper is
only a compatible constructor for its already-admitted Call shape. It must not
turn a Call grant into a Cast or Reply grant. A receive-only peer configuration
does not need the remote sender's signing secret. The destination receipt key
is separately provisioned and bound to the addressed recipient; it is never
borrowed from a relay or remote sender. The optional signer is absent only for
a send-only binding; a receive/admission operation without its addressed
destination signer is refused before queue mutation. A recipient/signing-key
association is trusted local provisioning, never an unauthenticated wire claim.

Every operation accepting a binding rechecks current policy generation, epoch,
expiry, operation and exact recipient scope. Handles are unforgeable through
the public safe API and belong to one service generation/epoch. A handle is not
authority after policy change. Registering a call verifies the encoded canonical
envelope and outbound signed capability against the source binding; a naked
caller-supplied header cannot allocate another principal's call slot.
`reply` requires an admitted Call handle, exact reply route/capability,
correlation/causation and current epoch; canonical sealing owns its cryptography.
An admitted Cast has no reply handle. A Reply is matched to the authenticated
expected sender/recipient and active call; a late Reply records `Late` and never
completes another call.

`ServiceError` contains bounded non-payload variants: `Unauthorized`, `Malformed`,
`Limit`, `Full`, `Backpressured`, `Expired`, `Unavailable`, `ShuttingDown`,
`Conflict`, `UnknownHandle`, `Clock`, and `State`. Authenticated admission outcomes
that have a canonical `DeliveryOutcome` use the signed canonical receipt;
unauthenticated input receives an error without destination existence disclosure.
`CallStatus` distinguishes `Pending`, `Replied`, `TimedOut`, `Cancelled`,
`Unavailable` and `ShuttingDown`; reply data is accessible only under the
current binding and is consumed once. Its retained bytes remain budget-charged
until consumption or bounded expiry; the terminal tombstone remains afterward.
`cancel` does not claim remote rollback and does not free the
terminal identity before its bounded retention expires.

`dispatch_object` consumes the private admitted-message proof, checks its service
generation, exact binding, current policy and content type, then installs the
existing crate-private `MailboxAuthority` lease and calls the shared object-v1
endpoint. This bridge does not construct a public unauthenticated `VerifiedFrame`
or decrypt a second time through `Direction::open`. Failed or stale dispatch
cannot mint a lease. Object publication remains distinct from queue admission.

First default ceilings are 65,536 application bytes, 262,144 encoded envelope
bytes, 16 MiB queued bytes / 1,024 messages / 64 active calls / 4,096 terminal
identities for the shared host owner. Tenant ceilings are 8 MiB / 512 messages /
32 calls / 2,048 identities; mailbox ceilings 4 MiB / 256 messages / 16 calls /
1,024 identities; sender ceilings 2 MiB / 128 messages / 8 calls / 512 identities.
Each accepted item is charged once to every applicable aggregate dimension,
including retained replay/terminal state. These defaults are upper bounds,
intersected with every narrower signed capability. Constructor validation rejects
zero, overflow and inconsistent child/parent limits. Terminal retention is
300 seconds after authenticated expiry; total state is bounded even if the clock
stops. Caller budgets and timeout inputs cannot exceed signed expiry.

Time is supplied by the trusted owner, not by a received message. Map signed
absolute expiry once to a checked monotonic deadline using the current clock
pair; expiry uses the earlier deadline and current authorization expiry. A
backwards monotonic clock or checked arithmetic failure refuses work. `expire`
and `shutdown` perform bounded work over admitted state; neither signs a false
durability or nonexecution assertion.

Deduplication is keyed by authenticated tenant/domain, recipient/mailbox, sender
principal/boot/execution epoch and stable `MessageId`. Its private immutable
request commitment hashes canonical inner `MailboxEnvelopeHeader` bytes and
length-prefixed exact payload bytes. **All inner header fields remain identical
on a retry**, including kind, recipient, correlation/causation, ordering scope,
sequence, content type, expiry, capability reference, policy/execution epoch,
nonce and encryption context. The outer delivery ID, route token/binding and
ciphertext may differ after authorized routing/resealing. Thus freshly sealed
ciphertext is not mistaken for a conflicting application request. A change to
an inner semantic field uses a new message identity; future migration semantics
must be explicit rather than weakened implicitly.

Retain the original signed admission receipt and immutable request commitment.
An authenticated retry with identical semantics returns canonical `Duplicate`,
bound to the current delivery/ciphertext and stable message ID, without queue or
handler work; the original disposition remains recorded internally. A conflict
returns a refusal and leaves the original unchanged. Full/backpressured/unaccepted
attempts commit neither accepted dedup state nor a queue item, so the same stable
message can be retried after capacity returns. Do not implement this by an
always-allow replay protector or by skipping canonical verification.

## Subsequent network carrier and access mappings

The existing FlowMux guest contract remains unchanged. In the inspected source,
`FlowMuxSession::handle_open_tcp` performs destination admission and tool/typed
transform checks before `open_opaque_flow` reserves a slot and calls
`connect_first_admitted`. A cooperative connector belongs after those checks,
inside the existing per-VM endpoint. It must consume admitted, DNS-pinned
addresses and identity rather than re-resolving or treating a digest as a grant.
There is no public connector hook in the inspected source; the required narrow
extension must be supplied and tested explicitly, rather than claimed present.

The first network slice covers admitted opaque TCP to an explicitly configured
cooperating endpoint. It preserves byte order, FIN/half-close, reset,
cancellation, quotas, audit and teardown. It does not carry arbitrary packets,
create a guest interface, add a default route or bypass the endpoint with a
host bridge. Typed transform flows stay in the substitution/redaction pipeline;
the forward leg is a separate integration site. UDP needs an explicit bounded
datagram mapping preserving datagram identity and size; TCP buffering is not a
UDP implementation. A required cooperative route fails closed when unavailable;
ordinary direct routing remains possible only through an explicitly admitted
ordinary route, never as a hidden downgrade.

The first cooperative prototype uses the peer endpoint itself as the exact
already-admitted destination. Its connector may return the existing local
`FlowSocket::Unix` pair to the unchanged FlowMux relay while a bounded worker
opens only the gate-pinned peer IP/port over the actual admitted TCP socket,
with the configured mutually authenticated TLS carrier. A TCP grant does not
authorize opening UDP/QUIC, a different transport port or another destination.
Such a transport requires its own signed MVM grant. The peer has one fixed,
pre-admitted application destination. Admission to destination A is not authority
to dial an unrelated gateway G or to ask G to open arbitrary destinations. Such
delegated exit routing needs its own explicit MVM authorization contract.

The ordered-stream mapping is not enabled by this design alone. Its next
contract slice must bind stream identity to the authenticated carrier/VM epoch,
preserve per-direction sequence and byte offsets, use bounded records backed by
the shared object-v1 codec, and distinguish object publication from stream
delivery. Define FIN/half-close/reset and maximum buffering/flush latency before
implementation. A disconnected live TCP stream fails explicitly; automatically
replaying bytes into a new backend connection is prohibited because the old
application may already have consumed them. A new stream can reuse authorized
verified cache content without claiming application-level exactly-once behavior.

Version the ordered-stream mapping before enabling it. Reuse object-v1 records,
codec implementations and verified cache state with stream-specific bounded
sequencing, ownership and close rules. Do not disguise a byte stream as mailbox
call/cast or force a packet plane into `PeerTransport`. Opaque encrypted Internet
payloads remain opaque; endpoint cooperation or an explicitly authorized
plaintext transformation boundary is needed for meaningful compression gains.

Guest access has two separately admitted modes under ADR-056. Development may
use the existing DevOnly shell/terminal route. Sealed workloads expose only
declared, typed application handlers; no PTY, exec, command/path/argv selection,
file-transfer verb or console is added. Sealed access never falls back to a
development shell. Vsock itself supplies neither SSH authentication nor access
authority. Use the canonical signed-plan, capability, expiry and audit checks.

## Acceptance and release boundary

First require positive call/cast and exact object-v1 dispatch, signed acceptance
verification and negatives for full/backpressured queues, malformed envelopes,
wrong recipient/tenant/epoch, replay/conflicting IDs, expiry, cancellation, late
reply, shutdown, reconnect and isolated service saturation. Demonstrate global
limits with at least two senders/mailboxes/tenants sharing one owner, not separate
independent maxima. Then run one guest/host and two guests with no NIC on both
Linux/Firecracker and macOS/HVF. Any unrun backend or lifecycle gate remains
explicitly unqualified.

Record actual encoded payload/envelope/stream bytes, setup and sender/receiver
CPU, verified-commit elapsed and complete lifecycle separately. Retain exact
readback, source/binary/patch identities, negative evidence and CPU/time losses.
Planned batch-byte counters are not transmitted bytes. Do not infer send-only
time by subtracting CPU, physical energy from CPU, or WAN performance from a
same-host vsock test. Three-peer/restart recovery, Iroh direct/explicit-relay WAN
and broader mailbox contention remain distinct follow-on acceptance slices.

Production lifecycle, host-blind/confidential tier and FIPS/CUI qualification
remain governed by MVM's requirements and concrete witnesses. Implementing a
bounded service does not by itself close #4629, #4633, #4607 or #4621.
