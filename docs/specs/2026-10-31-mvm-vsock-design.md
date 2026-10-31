# Experimental MVM guest mailbox binding for SpiralCore

Date: 2026-10-31. Status: approved implementation direction; live results are
recorded separately. This follows SC-MVM-001 at SpiralCore `ede2f0f` and uses MVM
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b` plus a reproducible local patch.

## Decision

Run the existing SpiralCore codecs inside a real Linux guest booted by MVM's
native macOS HVF backend. Connect the guest to the host through a dedicated
experimental mailbox vsock service, then run two guests with a bounded opaque
host relay. Use canonical MVM envelopes throughout. Preserve the existing
`SCMOBJ01` application format and selector policy.

The local service uses port **5257**. This is an experimental allocation in the
pinned patch, not an upstream reservation or a claim that MVM #4629 is complete.
At the inspected pin, the service registry assigns 5251–5256 to exit reporting,
machine control, FlowMux, telemetry, display and GPU; a targeted source/docs
search found no 5257 assignment. Put the new constant in the shared mailbox
contract and derive every caller from it. Never route mailbox data through
FlowMux 5253, machine control, telemetry or a console service alias.

One guest/host is the first acceptance step. Two guests are separate HVF worker
processes supervised by one small test orchestrator; do not instantiate multiple
HVF machines in one process, where MVM's hypervisor/GIC state is process scoped.
The initial service has one ordered outstanding call per connection. This is
enough to exercise real transfers without implementing a fleet router or a
general multi-tenant mailbox broker.

Alternatives considered:

| Alternative | Decision |
|---|---|
| Wait for upstream #4629/#4633 | Unnecessary for a local experimental integration; retain them as upstream delivery gaps. |
| Carry bytes over FlowMux or agent RPC | Reject: changes the security and service boundary and obscures the mailbox path. |
| Use a generic unregistered vsock port or ConsoleData alias | Reject: no typed service ownership or explicit fail-closed binding. |
| Build full mvmctl/fleet/Iroh integration first | Defer: larger dependency and admission surface than the first live witness requires. |
| Patch the cached Cargo source or publish an upstream change | Reject: preserve cache and upstream ownership; retain a source-pinned patch bundle in SpiralCore. |
| Native MVM HVF runner plus local dedicated service | Selected: actual guest, actual virtio-vsock, canonical crypto, limited source changes. |

## Source evidence and reuse

All MVM links below are pinned to the same source revision.

| Source | Implementation consequence |
|---|---|
| [GuestService](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-net/src/channel.rs) | Add `Mailbox`, its shared constant mapping, display name and uniqueness tests. No existing Mailbox variant exists. |
| [Guest socket leaf](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-agentd/src/vsock/sys.rs) | Reuse `dial(HOST_CID, MAILBOX_PORT)` and its owned descriptor; do not copy Linux socket structs/syscalls. |
| [HVF HostChannels](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-runtime/src/backends/hvf/host_channels.rs) and [kernel boot](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-runtime/src/backends/hvf/kernel_boot.rs) | `KernelBootUntilParams::builder_file`, `initramfs`, `disks`, `vsock`, `channels`, `stop` and `boot_kernel_until` provide a public live runner without mvmctl. Add a distinct optional `mailbox_socket`. |
| [Vsock handler registry](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-vmm/src/vmm/vsock_handlers/mod.rs) | Reuse `StreamRelayHandler`; register mailbox with an independent budget. An absent host endpoint must refuse the connection. |
| [Runtime bindings](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-vmm/src/vmm/vsock/bindings.rs) | Host paths are runtime authority, not guest snapshot content. Add mailbox endpoint to paths, setter/rebind and clear behavior. |
| [Relay budget](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-vmm/src/vsock_egress_bridge/substitution_bridge.rs) | Existing workload budget is 4 MiB/s sustained, 8 MiB burst, 128 streams. Preserve and report it; application admission below is stricter. Do not silently use the unbounded trusted-builder budget. |
| [Mailbox open](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-core/src/mailbox.rs) | The current function verifies a complete header but discards it and returns only payload. Expose the verified result through a compatibility-preserving API. |
| [Authorized transport](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-core/src/mailbox/transport.rs) | `AuthorizedMailboxFrame` owns private bytes; external transports need a read-only accessor. Keep its constructor private. `ChannelLimits` also needs getters so external channels enforce requested bounds. |

The cached source is read-only. Apply patches only in a managed MVM checkout,
record the original revision, patch digest, affected original blob identities
and patched hashes, and make patch application refuse another base. Keep the
patch and small source manifest in SpiralCore; do not vendor the full MVM tree.
Cargo must resolve all MVM crates in this run to one patched source, avoiding
duplicate unpatched/patched contract types.

MVM's AGENTS permits explicit native macOS HVF live lifecycle tests; this runner
uses that exception. Nix builds/evals, Firecracker and ordinary mvmctl commands
remain inside its builder boundary. No Lima or billable cloud is needed for the
selected path. Required `ponytail` and `rust-skills` were unavailable in the
installed host/plugin skills and MVM checkout; record that qualification and
follow the agreement's reuse-first, small-change, checked-error and test rules.
Graft tool, executable and index were unavailable; discovery used targeted source
reads rather than recursively reading specifications.

## Local MVM patch contract

The patch has two independent pieces, so both can be reviewed and replaced when
upstream supplies equivalent APIs.

1. **Canonical transport/verified-open access.** Add
   `AuthorizedMailboxFrame::encoded_bytes(&self) -> &[u8]`, and read-only
   `ChannelLimits::{max_frame_bytes,max_pending_bytes}` accessors. Add a private-field
   `OpenedMailboxEnvelope` with borrowed `header() -> &MailboxEnvelopeHeader` and
   `payload() -> &[u8]` accessors and consuming `into_payload() -> Vec<u8>`.
   `open_verified_envelope` takes the same seven arguments as existing
   `open_envelope`, executes the current
   verification path once and returns this value only after all existing checks,
   including replay. Existing `open_envelope` delegates and returns the payload,
   preserving its API and wire behavior. No public verified-result constructor,
   second crypto implementation, unverified header sidecar or arbitrary-byte
   authorized-frame constructor is added.
2. **Dedicated native-HVF mailbox channel.** Add `MAILBOX_PORT = 5257` to
   `mvm-contract::protocol::mailbox`; reexport it through `mvm-agentd::vsock` and
   derive `GuestService::Mailbox.port()` from it. Register a distinct
   `StreamRelayHandler` in the VMM. Thread `mailbox_socket` through
   `HostChannels`/`RunInputs` and `mailbox_endpoint` through `VsockHostBindings`,
   path enumeration, setters, restore/rebind and clear. Use the existing GPU
   endpoint wiring as the mechanical model, with a separate budget. Existing
   full-struct literals must be updated with `None` even when that binary is
   outside the selected live runner. A missing binding fails closed.

The minimal runner supplies `HostChannels` directly. It does not advertise a new
mvmctl command or claim general supervisor launch support. Snapshot handoff and
restore of this experimental service are explicitly refused in the first runner;
they cannot silently carry a stale mailbox binding. Cold start and process/store
restart tests come first. No upstream issue or PR is changed by this delivery.

## Stream framing and call/reply API

One connection begins with the eight-byte transport discriminator `SCMVS001`,
written by the guest and echoed by the host. It is a version selection and
readiness event, not authentication. Subsequent messages are exactly:

```text
u32 big-endian encoded_envelope_length
encoded canonical MVM envelope bytes
```

The prefix is outside the envelope. Permit lengths 1 through 262,144, reject
oversize before allocation, require the complete body, and close the stream on
malformed framing, wrong version, timeout or authorization failure. Do not put
the canonical JSON bytes inside another JSON/base64 wrapper. Count the prefix
and discriminator separately from canonical envelope bytes; measure every hop
in the two-guest relay and report logical versus physical-hop totals separately.

Use one bounded synchronous writer per stream. A partial write makes the stream
unusable; close it instead of retrying a fragment. A successful write proves
transport progress only. Each operation has a 30-second monotonic deadline,
bounded by its signed expiry, with cancellation closing the owned stream.
Handshake/read/write timeouts are checked, not silently ignored. No automatic
reconnect, implicit fallback port or unbounded queue is allowed.

Extend the adapter around two private proof types:

```rust
pub struct SentCall { /* private outbound identity, policy generation and deadline */ }
// Existing VerifiedFrame retains its private authenticated header and frame.
```

The concrete operations are `send_call`, `open`, `reply_to_verified` and
`accept_reply`. `send_call` stores the fresh canonical message ID/correlation,
expected reverse capability/address/key, deadline and exact SpiralCore
transfer/sequence. `open` creates `VerifiedFrame` only from upstream
`OpenedMailboxEnvelope`. `reply_to_verified` derives correlation and causation from that
proof, verifies that the signed `reply_to` equals the configured reverse mailbox
and capability, and uses the independent reverse Reply grant. `accept_reply`
consumes `SentCall` only if the verified Reply header has the same correlation,
causation equal to the request message ID, expected sender/recipient/mailbox,
reverse capability, ordering scope and current epochs; it also checks the
application transfer/sequence and legal reply kind. Late, unsolicited, repeated
or wrong-correlation replies cannot complete another call.

Keep the compatibility Direction API for SC-MVM-001, but the live binaries must
use these proof-bearing operations. The synthetic harness's remembered outbound
metadata is not an acceptable substitute for a verified inbound request header.
Transport reconnect or timeout requires an explicit new application attempt:
fresh envelope/correlation IDs, the same immutable SpiralCore frame when retrying
that application operation. This is application idempotence, not MVM's future
stable-MessageId retry/duplicate-receipt service.

The concrete live adapter signatures are:

```rust
StreamTransport::new(stream, allowed_route: RouteId, timeout: Duration) -> Result<Self>
StreamTransport::handshake(role: HandshakeRole, cancel: &CancellationToken) -> Result<()>
StreamTransport::receive(cancel: &CancellationToken, deadline: Instant) -> Result<Vec<u8>>
StreamTransport::counters() -> Result<StreamCounters>
Direction::send_call(frame, reverse_direction, transport, cancel, deadline) -> Result<SentCall>
Direction::open_live(wire: &[u8]) -> Result<VerifiedFrame>
Direction::reply_to_verified(request: &VerifiedFrame, reply, transport, cancel) -> Result<usize>
Direction::accept_reply(call: SentCall, reply: VerifiedFrame) -> Result<Frame>
SentCall::context() -> &Context
SentCall::deadline() -> Instant
```

`SocketIo` extends Read/Write/Send with checked read/write timeout and shutdown
operations; implementations wrap UnixStream/TcpStream, including the existing
owned guest vsock descriptor. StreamTransport owns one mutex-protected stream,
has no unbounded pending queue, and implements the canonical PeerTransport trait.
`open_live` additionally checks the verified sender BootId against its trusted
configuration: upstream OpenContext contains no expected-boot field. The live
proof retains the authenticated header. Existing `open` preserves SC-MVM-001's
synthetic-clock compatibility while live authority uses real time and expiry.
The blind relay uses bounded raw-envelope framing on separate owned streams; it
does not forge an AuthorizedMailboxFrame or treat framing as authentication.
`SentCall::context` derives the existing namespace from a successfully authorized
canonical outbound call; it installs no receiver authority lease. The live sender
uses a real Status probe and constructs WarmSession only after a matching canonical
reply passes `accept_reply`, never merely because a token supplied a Context.

## Authority, clocks and admission

All identities, signer keys, capabilities, signed routes and epoch values are
provisioned by the trusted test orchestrator. A CID, UDS pathname, route token,
VM name or connection is never principal authority. The endpoint uses canonical
OpenContext verification and exact signed grant scope before object/cache access.
Forward grants admit only Call/application-spiralcore-object-v1; reverse grants
admit only Reply of that content. The recipient never receives the sender's
private signing key. The two-guest host relay holds routing state only during
forwarding; the privileged orchestrator/HVF host remains in the experimental
trust boundary, so this is not hostile-host confidentiality qualification.

Fix SC-MVM-001's synthetic-time limitation: read real UTC time before sealing
and opening, derive a bounded local monotonic deadline, and recheck absolute
expiry and grant validity immediately before endpoint handling/publication.
The live profile admits at most five seconds of pairwise clock uncertainty.
Backdate the signed issue timestamp by exactly 5,000 ms using checked arithmetic;
keep OpenContext at actual UTC and leave canonical MVM verification unchanged.
Expiry remains actual sender time plus at most 30 seconds, further capped by the
grant, route and call deadline. The resulting signed issue-to-expiry interval is
at most 35 seconds; the local call lifetime is at most 30 seconds. Live receiver
and authority leases conservatively require actual UTC plus 5,000 ms to remain
strictly before the earliest envelope/grant/route expiry, with an equivalent
monotonic bound. Refuse a remaining validity window of five seconds or less.
This prevents a lagging receiver from publishing past the declared expiry.

Provision grant and route `not_before` at fresh bootstrap UTC minus ten seconds.
Set guest UTC using the existing `restore_clock::resync` and fresh trusted epoch
seconds immediately before spawn. Record public guest-ready UTC timestamps and
the host receive-time uncertainty interval; admit the campaign only if the
resulting pairwise offset bound is at most five seconds. Two separate absolute
offsets of ±5 seconds are not sufficient, because their difference can be ten
seconds. Reject stale bootstrap/slow-boot clock bounds instead of substituting a
fake verification clock. Keep positive test calls at the specified 30-second
profile; short-deadline cancellation negatives remain valid.
Keep wall-clock refresh distinct from policy generation changes; moving the
clock must not accidentally revoke every in-flight proof, while an expired
lease must fail even without `Policy::replace`. Explicit policy/epoch/revocation
updates increment generation and invalidate pending calls and cached authority.

For this milestone, one exclusive service owner executes each
`open → handle → reply` transaction. The demonstration does not expose a concurrent
policy-update control plane or hold a policy mutex across endpoint handling.
Such a spanning mutex would conflict with the authority callback's own policy
access. A future control plane must serialize updates between transactions before
acknowledging revocation; this milestone does not establish atomic concurrent
revocation from the existing `Authority::authorize -> ()` callback. Public APIs
outside this owner retain their documented concurrency limitation. A reply after
expiry or an observed policy-generation change is refused even if object commit
already happened; explicit recovery discovers the actual publication state.

Initial limits are one live connection per admitted guest route, one outstanding
call per connection, four active object transfers per receiver, 64 KiB application
frames, 256 KiB encoded envelopes, 16 MiB demonstration objects, 4,096 replay
identities per direction/session, and the existing shared 512 MiB resident/1 GiB
maximum cache ceilings. The 256 MiB guest disk uses a stricter 128 MiB cache quota
with reserved staging/headroom; failure to reserve space refuses the transfer.
Admit the connection/route before reading variable bodies. Enforce
the signed capability's payload, queue contribution, messages-per-minute and
outstanding-call ceilings as well; never treat signed limit values as enforcement
by themselves. A full replay set ends admission without eviction. A new execution
epoch is a lifecycle operation with fresh capabilities/routes, not a capacity
reset trick. Start each guest with 1 GiB RAM and measure RSS/guest memory before
claiming a smaller envelope.

Generate ephemeral credentials in memory for each run and pass them through
owned private pipes and in-memory initramfs buffers. Do not place keys in argv,
environment, console, Git, evidence or an unencrypted persisted initrd. The
guest's private bootstrap files are mode 0600 and available only to its endpoint;
the host's local UDS directories are mode 0700. Preserve only public identifiers,
source hashes and typed results in the evidence. Synthetic public data permits
fixture commitments in measurement records; this is not permission to log
production plaintext hashes under MVM's threat model.

## One guest, then two guests

**One guest/host.** Bind the host mailbox UDS before boot. The Linux guest opens
AF_VSOCK to CID 2/5257. MVM's actual virtio-vsock device and stream relay terminate
it at that UDS. The guest runs the existing Rust sender/codec and the host runs
the canonical receiver plus persistent object Endpoint. Compare raw, prepared
whole-object zstd and WarmSession under identical framing/crypto/durability.
The host independently reopens and compares exact reconstructed bytes and length.

**Two guests.** Bind two distinct UDS endpoints and launch two separately owned
HVF worker processes. Guest A is the sender; guest B owns receiver keys and
Endpoint. Both establish guest-initiated persistent vsock streams. The host
orchestrator pairs only those two preadmitted routes, forwarding complete bounded
canonical envelopes A→B and B→A without decoding plaintext. Validate the expected
outer route tokens/binding IDs and expiry as routing filters; these do not replace
recipient cryptographic authorization. No guest-selected host path, destination
CID, arbitrary TCP target or new route is accepted. This is a two-node local
orchestrator integration, not Iroh or a WAN.

For exact receiver verification, generate the registered public fixture again in
the receiver verifier outside the object cache and compare the reopened object
byte-for-byte; the verifier never supplies a base to the decoder. Record verifier
time separately. Use an owned 256 MiB writable ext4 data disk for guest B's cache,
passed through existing `DiskImage::open(path, false)`, and flush it before orderly
guest stop. After a graceful stop, a second read-only verification boot mounts
that disk without a mailbox socket or signing configuration. A trusted fixture
verifier is confined to the recorded historical Context/namespace and compares
public fixture bytes through a bounded read-only store API that reuses publication
validation. Existing `Endpoint::open` opens `.owner` for writing and collects
staging files, so it is not a read-only-mount API. The verifier does not accept
network messages or authorize a new guest owner. This proves persisted
disk contents, not a live fresh-epoch cache resume. Initramfs/tmpfs-only cache tests can prove process restart but cannot
be labelled durable across guest power loss. Disk image creation, physical disk
use, credential teardown and worker termination are recorded; delete only owned
test artifacts after verification.

## Retry, crash and teardown

This delivery does not implement an accepted-message journal. It emits no
`AcceptedDurable` or `AcceptedEphemeral` destination receipt. Canonical open,
transport write, verified object publication and application consumption remain
distinct events. Process-local replay plus object idempotence support explicit
fresh-envelope application retries only. A crash after open and before handle
exposes no committed object; a lost reply after commit leaves the object available
for explicit recovery. Receiver process restart within the guest preserves the
store. A new active mailbox owner requires a fresh BootId and greater execution
epoch, plus new grants/routes. The existing cache namespace binds policy epoch,
execution epoch and the capability-grant digest; old records cannot become an
authorized warm base under a new epoch without an explicit migration mechanism.
Live fresh-epoch resume/migration is deferred. The separate read-only verification
boot above proves persistence only. Durable inbox and stable-MessageId duplicate
receipts remain gated.

The orchestrator owns each worker process, listener, accepted stream, cache and
disk. Arm readiness/exit observation before launch; use process handles/events and
bounded deadlines, not sleep polling. On any failure close streams, signal the
worker stop handle, join/terminate the owned workers within a deadline and verify
exit. Preserve a small failure record and public boot diagnostics, never keys or
payload. Snapshot/clone, fleet movement and general route discovery are refused
until their own epoch/key/binding semantics are integrated.

## Acceptance and evidence

Require actual guest evidence: kernel and guest executable hashes, Linux boot,
successful AF_VSOCK exchange on 5257, patched MVM revision/patch identity, observed
worker lifecycle, receiver-side exact comparison and clean teardown. In-memory,
Unix-pair and host-only tests are prerequisites, not substitutes for a live guest.

Begin with 1 MiB cold, unchanged, localized edit, unrelated replacement and one
length change. After that passes, repeat at 16 MiB. Reuse one receiver for each
mixed session. Include one carried-through ternary/Kuramoto/CA runtime fixture so
the integration does not accidentally narrow SpiralCore to one codec. Do not
expand codec optimization during this integration.

Record application bytes, canonical bytes, transport-prefix bytes and both
physical hops for the two-guest case; separate setup/seed/retry costs. Measure
sender CPU, receiver CPU, host-relay CPU, send/receive elapsed and full operation
elapsed through verified publication/reply. Guest-local monotonic clocks measure
durations; do not subtract unsynchronized clocks across machines. Capture the
4 MiB/s relay budget, guest memory, object size, selected codec and persistent
cache state with every result. These are local guest/vsock performance results,
not new WAN or government/FIPS qualification claims.

Negatives cover absent/wrong service binding, frame truncation/oversize, wrong
recipient/key/capability, wrong reply correlation/causation, tamper, stale boot or
epoch, live expiry, revocation between transactions, cancelled/partial writes,
bounded queue/rate/replay exhaustion, disconnect before commit and lost reply
after commit. A wrong-route relay may drop traffic but cannot obtain plaintext or
cause the wrong receiver to publish. Complete patched workspace/gated checks
required by MVM before claiming upstream-ready qualification; explicitly list
any unrun broad checks for this local experimental patch.

## Native HVF kernel compatibility observation

The first two real 4 KiB guest/host attempts reconstructed and independently
verified all eleven objects, then failed the new runner's zero-exception shutdown
condition. Diagnostic attempt 02 recorded normal `PSCI_SYSTEM_OFF`, no watchdog,
no reset, and exactly two non-HVC exceptions with class `0x18` (ARM system-register
trap). The pinned MVM [native Linux baseline](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/specs/benchmarks/327-hvf-quota-spike-findings.md#L244-L257)
already documents two exceptions in every one of twenty successful real Linux
boots, matching its unhammered control. Its backend records and advances past
non-HVC exceptions; this witness does not change that backend behavior.

The runner therefore permits either zero such exceptions, or exactly two of
class `0x18` for kernel SHA-256
`0f97f109154a173352c07db223ecff48d44b38514b500f39cd9897701dbac0b6` only.
Hashing and loading use the same held file descriptor. Other counts/classes and
unpinned kernels retain refusal. Normal power-off, no reset/watchdog, one final
successful guest marker, disk synchronization and exact readback remain required.
Success receipts retain the actual counts/classes and compatibility-profile name.
The two register identities have not been investigated; this is a bounded trusted
test-image allowance, not a qualification of arbitrary guest instructions or the
hypervisor. Both failed attempts remain in the evidence and are excluded from the
registered performance campaign.
