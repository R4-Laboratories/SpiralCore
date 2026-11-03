# Plan: canonical MVM networking delivery

The design is [MVM network delivery](../specs/2026-11-02-mvm-network-delivery.md).
The user authorized the NIC-less host-mediated sequence: canonical local mailbox
service, FlowMux integration, then separately admitted access modes. Each phase
is independently reviewable and must retain the same SpiralCore codec core.

## Phase A — Standalone bounded canonical mailbox service

Risk: high at authentication/admission transaction; medium elsewhere. This is
the immediate implementation, not a new research audit.

1. Add RED tests in `crates/spiralcore-mvm-adapter/tests/service.rs` for canonical
   call/cast admission and receipt verification, full/refused retry preserving
   stable message identity, duplicate without redispatch, conflicting identity,
   aggregate shared limits, cancellation/late reply and shutdown. Use the
   existing synthetic canonical fixtures; no new security protocol.
2. Add `crates/spiralcore-mvm-adapter/src/service.rs`, exported from `src/lib.rs`.
   Implement a service-owned transactional replay/dedup and admission owner using
   canonical verification and receipt types. Make shared aggregate limits
   explicit and private; do not layer admission after the existing `Direction`
   replay insertion. First implement ephemeral ownership only.
3. Add typed call/cast APIs with monotonic local deadlines and bounded terminal
   correlation state. Preserve signature/recipient/epoch checks before exposing
   existence or processing. Calls and casts have distinct application-reply
   behavior. An object-v1 cast cannot manufacture cache readiness.
4. Connect the canonical service's admitted object content type to the existing
   `v1::FrameEndpoint` / `ObjectEndpoint` without copying the codec or storage
   state machine. Test exact empty/binary/zero/boundary objects, normal updates,
   interrupted publication and durable object-store reopen independently of
   ephemeral mailbox acceptance.
5. Reuse and verify the existing `GuestService::Mailbox`, port and independent
   channel limits in `crates/spiralcore-mvm-live/mvm-patch`. Extend only the actual
   service/queue hook through its pinned preparation and provenance process.
   Preserve the source-pinned original bundle and measured receipts.
   Do not mutate the retained prepared working tree or default-enable production.
6. Update `crates/spiralcore-mvm-live/src/campaign.rs` and the owned native harness
   to exercise actual service admission and distinct receiver-local signed
   acceptance, object publication and application reply. Implement the explicitly
   proposed canonical encrypted receipt carrier from the design: reserved receipt
   content type, strict encoding of existing SignedDestinationReceipt, independent
   Cast grant/return route, bounded reserved control capacity and no recursive
   receipt. Demultiplex authenticated receipt Casts independently of application
   Replies and retain the original object frame bytes. Supply this MVM-side
   extension for upstream owner review; do not label it previously adopted.
   Never expose a bare receipt to the host relay or invent an implicit ACK frame.
   Test queues/credits remain isolated from
   control, telemetry, FlowMux and workload-exit, including saturation/teardown.
7. Run one guest/host, then two guests with a small owner on Linux/Firecracker
   and macOS/HVF. Use source-pinned public fixtures, no guest NIC and bounded
   existing host resources. Record bytes, CPU and elapsed plus reboot/readback
   and service-negative witnesses. Publish gaps rather than closing the upstream
   contract prematurely.

Dependencies: canonical MVM DTO/crypto APIs and compatible pinned prepared source.
The current integration depends on the explicit outside-contributor fork commit
`ae12e9deb41cc9a3264196ce22ebc72d4042883d` over upstream `6ae461a`, including the
tested canonical Reply-operation correction. New source must pass its own
qualification; it does not inherit the old native result.
If current upstream requires a source-pin bump, review only the API/source delta,
retain the old witnesses and rebuild/measure under the new exact identity.

## Phase B — Host-mediated FlowMux carrier

Risk: high. Depends on A's shared endpoint integration, but not on a new mailbox
packet plane. Add a normative ordered-stream endpoint mapping to
`docs/specs/spiralcore-v1.md` before Rust implementation, with explicit framing,
bounded sequence/close/reset/ownership semantics and profile identity.

1. Supply a narrow retained MVM extension after `handle_open_tcp` admission and
   transformation checks, at `open_opaque_flow`'s host connector seam. Retain the
   original policy, DNS-pinned IPs, registry slot, audit and credit ownership.
2. Add the cooperative adapter using shared Rust object records/codecs, not a
   second networking stack. Configure one explicit peer route; preserve normal
   TCP ordering/half-close/reset and refuse an unavailable required adapter.
3. Demonstrate one admitted ingress and egress plus unlisted-destination refusal,
   identity/epoch failure, payload-free audit, route failure, backpressure and
   cleanup on both supported backends. Typed HTTP forward-leg and UDP datagram
   integration are separate bounded follow-ups with their own tests.
4. Compare the same admitted transport with raw and equally tuned whole-object
   zstd, then direct/explicit-relay WAN. Record all setup, actual traffic, CPU and
   verified end-to-end time. Preserve ADR-042's applicable latency, throughput and
   RSS gates; no transparent general-packet or encrypted-traffic savings claim.

## Phase C — Owner access and production lifecycle

Risk: high. Reuse ADR-056's existing admitted DevOnly access for development and
declared typed handlers for sealed workloads. Add no sealed exec/PTY/file verbs
and no automatic development fallback. Demonstrate wrong owner/tenant/epoch,
revoked grant, shutdown and sealed-shell refusal under canonical audit.

Implement encrypted durable mailbox journal/replay/receipt transactions before
`AcceptedDurable`, then restore/reconnect/epoch fencing and three-peer recovery.
Use canonical `PeerTransport` for Iroh direct/customer-relay paths; public relays
stay opt-in. Production source adoption, broader isolation/load tests, qualified
host-blind tiers and concrete validated-crypto requirements remain explicit
delivery gates, not conclusions drawn from local codec demonstrations.

## Scale and long-lived operation acceptance

[Scale qualification #49](https://github.com/R4-Laboratories/SpiralCore/issues/49)
is required before the final MVM contribution. The two-guest pilot and native
client matrix establish functional boundaries. They do not qualify fleet size.

The [source-backed scale boundary review](../network/evidence/2026-11-02-mvm-service/qualification/scale-boundary.md)
sets the immediate sequence; these steps are not yet implemented or measured:

- **Broker context** — files `crates/spiralcore-mvm-live/src/campaign/canonical.rs`
  and `crates/spiralcore-mvm-adapter/tests/service.rs`: introduce explicit shared
  budget plus monotonic-clock ownership for multiple endpoints. Add a regression
  for aggregate refusal across independently created sessions first. Why: the
  witness currently constructs both per Session. Dependency: none; risk medium.
- **Persistent ordering** — files `crates/spiralcore-mvm-adapter/src/service_sender.rs`
  and `crates/spiralcore-mvm-adapter/tests/service_sender.rs`: test and expose a
  stable owned scope with checked increasing sequences. Preserve fresh-message
  compatibility and replay refusal; do not silently retire cursors. Dependency:
  broker context; risk high because ordering identity is protocol state.
- **Bounded capacity probe** — new Rust example
  `crates/spiralcore-mvm-adapter/examples/service_capacity.rs`: use that shared
  context for 2/8/32 logical endpoints, fresh/stable scope controls, hot/slow peers
  and repeated expiry windows. Record the counters specified by the review,
  including refusals and tail latency. Stop at declared limits; no additional
  native guest allocation. Dependency: the preceding controls; risk medium.
- **Choose the next optimization from measurements** — document results under
  `docs/network/evidence/` and update #49 before changing locks or adding a
  negotiated pipeline window. Test gap repair, epoch/revocation, malformed floods
  and control progress under saturation. Dependency: capacity probe; risk high
  for authority/ordering changes. Keep the current serial native path as control.

1. Freeze a capacity registration per role: peers/MVMs, active streams/transfers,
   workload sizes, cache/staging/metadata budgets, CPU, RSS and descriptor limits.
   Exercise logical endpoint ladders separately from actual booted guests, with
   each native run inside its approved host resource budget.
2. Qualify long-lived sessions beyond the 2048-Call witness cap. Prove bounded
   FIFO, dedup/receipt and scope metadata under sustained work and churn; retain
   replay, stable retry identity and epoch safety while reclaiming terminal state.
3. Evaluate bounded pipelining/batching against the current serial consumer.
   Specify any negotiated window/profile in the shared contract, keeping one
   protocol and distinct admission/publication/application acknowledgments.
4. Prove fairness and overload behavior with hot/slow peers and tenants, cold
   caches, cancellation, relay loss and hostile input. Charge crypto/replay work
   within aggregate budgets; large reconstruction must not starve controls.
5. Record actual bytes, verified goodput, p50/p95/p99 queue/commit latency, CPU,
   RSS, cache/staging/metadata growth and FDs through saturation and recovery.
   Compare raw and tuned zstd under matching load/auth/durability on steady-state
   and bounded-soak local/WAN runs. Preserve losses and unqualified configurations.

## Per-phase completion

Run relevant RED/GREEN tests and focused independent Rust/security review,
strict warnings and backend contracts; capture immutable source/patch/executable
identities. Commit compact reproducible evidence and report every unrun gate or
measured regression. Merge through a reviewed PR to SpiralCore `origin/main`,
preserving user runtime/saves and other lab state. Do not rewrite old results or
claim an upstream issue completed from a partial compatibility implementation.
