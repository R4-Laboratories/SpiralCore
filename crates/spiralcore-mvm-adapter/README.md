# SpiralCore canonical MVM mailbox adapter

This standalone Rust crate connects SpiralCore's shared lossless object-v1
endpoint to canonical MVM envelopes, capabilities, cryptography and authorized
`PeerTransport`. With the `live` feature, it includes a bounded ephemeral mailbox
service, encrypted destination receipts, Call/Reply correlation and a private
admitted-message bridge into the existing object store. The codec selection,
ternary/Kuramoto/CA replay, warm cache, sparse changes and schema mechanisms stay
in the shared transfer core; the adapter does not copy those implementations.

The committed manifests and lockfile pin the public contribution-preparation
fork [`ae12e9deb41cc9a3264196ce22ebc72d4042883d`](https://github.com/Casey-allard/mvm/tree/ae12e9deb41cc9a3264196ce22ebc72d4042883d),
based on upstream MVM `6ae461a54d85a1b70477c1efb97ea4a33ef3b331`. Its four-patch
[preparation and provenance bundle](../spiralcore-mvm-live/mvm-service-patch/README.md)
contains the verified-open/experimental transport primitives, proper canonical
Reply-operation authorization, a read-only prepared-header accessor, and the
Firecracker descriptor handoff correction. This is an explicit contributor fork,
not an assertion that upstream has adopted the proposed service or receipt
carrier. No local Cargo patch configuration is required for the default build.

The [object-v1 contract](../../docs/specs/spiralcore-v1.md) and
[MVM delivery contract](../../docs/specs/2026-11-02-mvm-network-delivery.md) define
one object protocol across endpoints. This crate preserves object-v1 frame bytes
inside canonical MVM envelopes. The proposed receipt carrier encodes MVM's
existing signed destination receipt inside an independently authorized encrypted
Cast; it does not replace MVM's signatures, envelope format or cryptography.

## Current service API

Enable `live` for the service and transport APIs. Authority comes from trusted
local configuration, never a message payload or object digest.

| Component | Responsibility |
|---|---|
| `service::MailboxBinding` and `ServiceBindingConfig` | Bind canonical kind, operation, address, sender boot, keys, capability, route and current policy. Receive-only bindings need the remote sender's public verification key, not its signing secret. Destination receipt signing is a separately configured local authority. |
| `service::HostBudget` and `ControlReserve` | Share aggregate tenant/mailbox/sender limits across the host's service, trackers and return paths. Reserved control capacity is inside the same totals; data cannot consume it. Use clones of one owner for cooperating components, not an independent budget per adapter. |
| `broker::BrokerContext` | Own one host budget, monotonic clock origin and synchronous local dispatch gate. Clone the context across endpoint sessions and sample time inside `with_time`; release dispatch before socket I/O. Independent per-session clocks cannot safely drive the same globally monotonic budget. |
| `service::LocalMailboxService` | Authenticate canonical Call/Cast/Reply messages, transactionally admit/deduplicate them, enforce ordering scopes and mailbox-epoch ownership, and manage bounded deadlines, cancellation, replies and terminal state. Admission is ephemeral. |
| `service_sender::MessageOptions` | Construct immutable message metadata. Retrying a stable message preserves the complete inner header and payload; a fresh outer delivery/encryption attempt does not authorize changed semantics under the same message identity. |
| `service_sender::OrderingStream` | Own one stable scope and checked sequence allocator for a canonical address/sender/epoch/kind. Keep it for persistent traffic; reuse allocated options for retries. Invalid metadata or a wrong Reply binding must not advance it. Fresh-per-message metadata remains available for compatibility. |
| `receipt::ReceiptReturnConfig` | Reserve the independent receipt-control path before application admission, then produce an encrypted canonical receipt Cast. The integrated `admit` path uses the same budget owner and authenticates/decrypts the application message once. |
| `receipt::ReceiptTracker` | Verify destination receipts against the locally configured destination key and exact send attempt. Track admission separately from application Reply, retain bounded retry/terminal state, and prevent receipt-of-receipt loops. |
| `service_object::ObjectMailbox` | Consume the private `AdmittedMessage` proof for an object-v1 Call, recheck current policy/owner/expiry, and grant scoped access to the existing object endpoint. No public constructor can forge that proof, and no second decrypt or duplicated codec state machine is needed. |
| `MailboxBinding::send_prepared` | Reauthorize the canonical prepared envelope and send through MVM's authorized `PeerTransport` with cancellation/backpressure. It does not bypass transport admission with a raw socket write. |

Use a newly provisioned `HostBudget` to create one `BrokerContext`, and construct
the local services/trackers from that context's budget inside `with_time`. Each
local protocol phase obtains fresh time under that same dispatch gate. Do not
perform socket I/O, recursively dispatch, or reuse sampled time for later service
operations inside another phase. The synchronous gate deliberately serializes
local work; it does not promise fairness or a parallel reconstruction capacity.

For receipt transmission, call `PreparedReceipt::reserve_send` during local
dispatch, then consume its non-cloneable permit with `send` outside dispatch.
The permit retains queue credit through transmission and rechecks current
source/return policy, cancellation and expiry. Every retry reserves a new rate
attempt. The existing combined `PreparedReceipt::send` remains available to
single-owner callers. Timestamp validation uses the trusted local caller's time;
it does not establish a hard physical-time deadline under arbitrary scheduling.

Native callers can use `campaign::broker_context`, `sender_with_broker` and
`receiver_with_broker` in the live crate to inject one owner across sessions.
The existing `sender`/`receiver` wrappers explicitly create single-endpoint
contexts. Neither API aggregates separate processes automatically. When both
local sides of a Call share a budget, their pending/admitted slots both count
against the signed grant; provisioning must admit those roles explicitly.

`HostBudget::snapshot` and `LocalMailboxService::snapshot` expose read-only local
diagnostics. They do not clear replay state or offer remote cache membership.
FIFO cursors remain retained: stable streams avoid creating a checkpoint per
message, and no silent eviction or authenticated retirement protocol is added.

Application messages remain at most 64 KiB; canonical envelopes are bounded at
256 KiB. The reserved receipt profile has a separate 4 KiB payload / 16 KiB
encoded-envelope bound. Signed capability limits and shared byte/message/call/
identity budgets all apply. Native witness resource tiers are explicit local
qualification settings, not changes to the reusable service defaults.

The service keeps blocked ordering scopes separate, refuses conflicting message
identities, and does not publish accepted state when admission is full/refused.
A higher mailbox execution epoch fences older handlers and cached replies.
Policy replacement invalidates retained proofs; scoped object operations hold
the policy and owner checks through access. Terminal identities and reply state
are retained only within bounded local lifetimes. These are process-local
semantics, not a durable mailbox journal or exactly-once application execution.

## One Call, four encrypted envelopes

A cooperating pair uses independently admitted normal Call and Reply directions,
plus reserved receipt Cast grants in both directions. An encrypted receipt has
its own keys/content type and an explicitly authorized return route. Receipt
traffic cannot infer authority from a received address or request payload.

1. The caller prepares the canonical Call, registers its pending Call and send
   attempt, and sends it through `send_prepared`.
2. The destination uses `ReceiptReturnConfig::admit`, reserves control capacity,
   and returns the signed admission receipt inside an encrypted Cast. Its
   service can then dequeue the private admitted-message proof.
3. `ObjectMailbox` dispatches the existing object frame. The destination prepares
   a correlated application Reply with `reply_prepared`, registers that send
   attempt and sends it through the same authorized transport API.
4. The caller separately admits the Reply and sends its encrypted destination
   receipt. `poll_call` consumes the application reply payload once. The verified
   Reply receipt lets `acknowledge_reply` release the destination's cached Reply;
   a lost Reply can be retried without running the object handler again.

A Cast has no application Reply. A reserved receipt Cast is handled by the
receipt path and is never acknowledged recursively.

**Mailbox acceptance, durable object publication and application processing are
separate facts.** `AcceptedEphemeral` means the bounded local service accepted the
message; it is not `AcceptedDurable`. The object endpoint can independently
verify and durably commit exact bytes. An application Reply or receipt alone
does not promise exactly-once execution. Retries preserve at-least-once delivery
semantics and idempotent object publication within the specified identity and
authorization boundaries.

## Clock issuance policy

Existing constructors retain `issuance::IssuancePolicy::Exact` behavior. A caller
can explicitly select `Bounded` through `MessageOptions::fresh_for_binding`,
`for_reply_with_policy`, and `ReceiptReturnConfig::new_with_policy`. The policy
is applied once when immutable metadata is created, including receipt Casts.

The maximum permitted backdate is 5,000 ms. A grant/route too young to cover the
whole declared backdate is refused, rather than silently clamped. Expiry is
bounded by requested expiry, grant/route validity, and **issued + 30 seconds**.
Thus a five-second backdate allows at most 25 seconds of remaining lifetime;
it does not create a 35-second TTL. Native receive deadlines are clipped to that
actual expiry. Receiver time, canonical future-issued/expired checks, and signed
receipt observation time remain unchanged. See the retained
[clock regression evidence](../../docs/network/evidence/2026-11-02-mvm-service/qualification/clock-issuance.md).

## Run the portable service tests

Use Rust 1.97.1 and the committed standalone lockfile. Run these selected targets
with the portable `live` profile on Linux, macOS and Windows; hardware SHA
assembly is deliberately not enabled:

```sh
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml --features live --test canonical --test service --test receipt --test service_epoch --test service_sender --test service_tcp --test service_clock
```

The seven required targets contain **50 portable tests**: 9 canonical, 16
service, 15 receipt, 3 epoch, 2 sender, 1 TCP and 4 clock. TCP coverage uses real
loopback sockets. This does not test a native guest, WAN behavior, a Windows
hypervisor, or Windows durable object storage.

On supported Unix development hosts, the broader all-feature/all-target suite
contains **82 tests**, including compatibility paths, durable object tests and
examples:

```sh
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml --all-features --all-targets
```

These commands describe the current test surface; their presence is not a claim
that the current dependency pin has completed fresh qualification.

## Compatibility Direction and conformance harness

`Direction::send/open`, `VerifiedFrame`, and the `conformance` example remain
available for compatibility and the retained measurement harness. They provide
canonical sealing/opening, route checks and scoped frame dispatch, but **are not
the canonical mailbox broker described above**. A direction's bounded
process-local replay state must not be reset to bypass its capacity. Use the
service API for shared admission, receipts, generic Call/Cast/Reply lifecycle
and current object-proof integration.

The opt-in `conformance` feature supplies synthetic identities, ephemeral keys
and MVM's bounded `InMemoryPeerTransport`; it is excluded from the default
library. Its fixed fixture clocks/epochs are not a native guest lifecycle.
The historical Unix example can still be built and run with fresh output paths:

```sh
cargo +1.97.1 build --release --locked \
  --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml \
  --example mailbox_transfer --features conformance,hardware-sha256
SPIRALCORE_SOURCE="$(git rev-parse HEAD)" \
  crates/spiralcore-mvm-adapter/target/release/examples/mailbox_transfer \
  /tmp/spiralcore-mvm-new-store /tmp/spiralcore-mvm-new-results.jsonl
```

Both paths must be new. If `CARGO_TARGET_DIR` is set, use the executable under
that directory. The example retains failed sessions, verifies every original
byte after reopening successful stores, and removes only its own fresh stores.
It compares persistent warm and prepared whole-object zstd at 1 MiB and 16 MiB,
records canonical-envelope bytes and combined process CPU/elapsed, and charges
setup separately. It supplies no network-send/WAN-time measurement. Its
[source-pinned historical measurement contract](../../docs/network/evidence/2026-10-31-mvm-readiness/measurement-contract.md)
and old results remain unchanged; they do not qualify the current service.

## Qualification and remaining delivery

The current [seven-platform report](../../docs/network/evidence/2026-11-02-mvm-service/platforms/README.md)
records **358 passing executions** against public MVM fork `ae12e9d`: 50 portable
tests on each of seven native Linux/macOS/Windows rows, plus eight Unix storage
executions. The Linux release workflow also passed its 42 host/adapter/campaign
tests and shared conformance. Original source, runner and artifact identities
remain in the report.

The [current native report](../../docs/network/evidence/2026-11-02-mvm-service/native-ae12/README.md)
records **16 successful local sessions**, on Linux/Firecracker and macOS/HVF,
through the canonical four-envelope admission lifecycle. Both size tiers have
one-guest/host and two-guest runs; actual larger primary objects are 14,851,384
bytes. All exact readbacks passed, including 72 keyless recovery readbacks across
eight recovery boots. Bytes, endpoint CPU, elapsed/lifecycle time and observed
losses are recorded for both arms, without pooling different backend profiles.
These single-trial local runs do not establish WAN or fleet capacity.

The earlier [macOS](../../docs/network/evidence/2026-11-02-mvm-service/mac-native-intermediate/README.md)
and [Linux](../../docs/network/evidence/2026-11-02-mvm-service/linux-native-intermediate/README.md)
intermediate results and failed gates retain their original identities. The
[native witness guide](../spiralcore-mvm-live/README.md) describes its separate
build/run boundary. Mailbox port 5257 remains experimental and production is
disabled. FlowMux port 5253 remains a separate policy-controlled network path.

Open delivery work is tracked in
[#43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
[#44](https://github.com/R4-Laboratories/SpiralCore/issues/44),
[#45](https://github.com/R4-Laboratories/SpiralCore/issues/45),
[#46](https://github.com/R4-Laboratories/SpiralCore/issues/46),
[#47](https://github.com/R4-Laboratories/SpiralCore/issues/47) and
[scale qualification #49](https://github.com/R4-Laboratories/SpiralCore/issues/49).
The [delivery plan](../../docs/plans/2026-11-02-mvm-network-delivery.md) covers
upstream service adoption, durable inbox/replay, production lifecycle,
policy-preserving FlowMux integration, separately admitted access modes,
three-peer recovery, long-lived/fleet scale and real direct/relay WAN tests.
These remain delivery gates. This crate does not activate the production MVM
service, turn ordinary encrypted traffic into compressible plaintext, or provide
a completed Internet router. No production security, CUI compliance or FIPS
validation is claimed.
