# Plan: persistent canonical MVM mailbox service

Date: 2026-11-03. Base: `b154fb7` on
`codex/spiralcore-persistent-service`. Delivery authority:
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49), with canonical
service work under [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43)
and parent [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).

Contracts: [persistent service](../specs/2026-11-03-persistent-mvm-service.md),
[host budget v1](../api/persistent-host-budget-v1.md), and the existing
[canonical MVM mapping](../specs/2026-11-02-mvm-network-delivery.md).
This plan continues the [sustained results](../network/sustained-broker-results-2026-11-02.md);
it does not replace their source identities or repair their failed cells.

## Delivery boundary and order

The immediate slice removes object reconstruction/storage from global dispatch
while preserving current proof, policy and reply checks. It is independently
valuable and must be reviewed and measured before the larger ownership change.
Next establish one transport owner for links to independent worker processes;
then add installation exclusivity and a reviewed service partition authority
before claiming true host-wide canonical service aggregation. Qualify
persistent sessions beyond 2,048 Calls, including retry, expiry and
FIFO safety. Only after those controls pass, evaluate a bounded negotiated
pipeline under the same canonical protocol and shared object-v1 implementation.

The existing `HostBudget` is `Arc`-shared inside one process. The existing native
roles create separate process-owned budgets. Giving their contexts the same
name, adding snapshots, or summing configured guest maxima is not host-wide
enforcement. Distinguish host-reserved transport/worker credits from plaintext
service accounting inside guests. The former can provide a conservative host
resource bound; it cannot prove the latter's canonical admission transaction.

All new mailbox acceptance remains `AcceptedEphemeral`. Durable object commit
and exact stopped-store reopen remain separate observations. Mailbox journal,
crash-persistent receipts/FIFO, exactly-once application execution, native Windows,
canonical WAN and FlowMux/access integration remain separate release gates.

## Small commits and dependencies

Each numbered item is a reviewable commit. Within a commit, write its focused
RED regression first and make one bounded production edit at a time, targeting
roughly 50 changed production lines per step; split a larger state transition
into another commit rather than burying it in a broad refactor. New paths below
are proposed owned files, not assertions that those APIs already exist.

### A. Detach expensive work; keep pipeline one

1. **Dispatch regression** — files
   `crates/spiralcore-mvm-live/src/campaign/canonical.rs`.
   Add a private `canonical::application_phase` seam with a deterministic
   barrier-controlled worker: block its object phase and prove independent
   service expiry sharing the broker completes before that worker is released.
   Retain the worker join/watchdog so a failed regression cannot hang its process.
   The test must fail because object work currently owns global dispatch, not
   because of an arbitrary elapsed-time threshold. Dependency: none. Risk: medium.
2. **Three-phase handler** — files
   `crates/spiralcore-mvm-live/src/campaign/canonical.rs`,
   `crates/spiralcore-mvm-adapter/src/service_object.rs` and
   `crates/spiralcore-mvm-adapter/tests/service_object.rs`.
   Keep canonical admit/receive under dispatch; sample current trusted time in
   a short phase before `application_phase` runs `ObjectMailbox` outside
   dispatch; reacquire dispatch and sample again for reply preparation/tracker
   mutation. Retain the private admitted proof,
   object policy/authority lock and exclusive endpoint worker. Do not expose a
   reusable namespace authority or share mutable codec state between workers.
   Add expiry/revocation/epoch tests between phases: a completed storage mutation
   does not authorize an expired Reply or imply cancellation rollback. Socket
   waits remain outside dispatch. Dependency: A1. Risk: high.
3. **Scoped measurement** — files
   `crates/spiralcore-mvm-live/examples/broker_objects.rs`,
   `docs/network/evidence/2026-11-03-persistent-mvm-service/registration.json`
   and `docs/network/persistent-mvm-service-results-2026-11-03.md`.
   Add actual authenticated receipt/control-progress measurement beside blocked
   expensive object work; the helper regression alone does not establish that
   live exchange. Record handler time, dispatch wait/hold and control progress for a
   small raw/zstd/Combined comparison; retain exact fidelity and the historical
   large-raw refusal. Publish this as dispatch isolation only. Dependency: A2.
   Risk: low. A1–A3 form the first PR; do not wait for pipeline work to deliver it.

### B. One transport owner, then explicit host-wide service gates

4. **Owner accounting and refusal** — files
   `crates/spiralcore-mvm-live/src/host_budget.rs`,
   `crates/spiralcore-mvm-live/src/lib.rs` and
   `crates/spiralcore-mvm-live/tests/host_budget.rs`.
   Implement `HostTransportBudget`, trusted `TransportScope`, non-clone
   `LinkReservation` and `FrameReservation` from the reviewed contract. Count
   links, frame slots and reserved serialized-buffer capacity at owner/tenant
   scopes. RED tests cover combined refusal even when each link request fits,
   overflow, zero/oversized requests, foreign scopes and unchanged accounting on
   refusal. A live frame retains its link charge after the outer link drops;
   underflow/poison fences the owner. These credits reveal no inner identities,
   keys or receipt class and cannot replace canonical signed admission.
   Dependency: reviewed API contract; A need not block pure accounting tests.
   Risk: high.
5. **Bounded relay buffers across process links** — files
   `crates/spiralcore-mvm-live/src/orchestrator.rs`,
   `crates/spiralcore-mvm-live/src/host_budget.rs`,
   `crates/spiralcore-mvm-adapter/src/live.rs` and
   `crates/spiralcore-mvm-live/tests/host_budget.rs`.
   Construct one owner before activation and inject clones into all admitted
   links to separate worker processes. Reserve a bounded frame slot and maximum
   buffer capacity before an allocating read; hold the guard through forwarding
   and destroy its buffer before release. Preserve current framing. Test partial
   read/write, route refusal, child death, slow forwarding, cancellation and
   teardown. A blocked I/O task must not own the ledger mutex or prevent another
   tenant's permitted reservation. Report decoded-copy/RSS exclusions and actual
   traffic separately. Dependency: B4. Risk: high. This has no IPC/lease expiry
   and is labeled `scope=orchestrator` until B6a passes.
6. **Canonical service integration** — files
   `crates/spiralcore-mvm-live/src/linux_host.rs`,
   `crates/spiralcore-mvm-live/src/host.rs`,
   `crates/spiralcore-mvm-live/src/guest.rs`,
   `crates/spiralcore-mvm-live/src/campaign/canonical.rs` and the retained
   `crates/spiralcore-mvm-live/mvm-patch/` provenance files.
   Bind each admitted transport link to the existing `GuestService::Mailbox` registration,
   independent queues and credits, current route/capability/epoch and bounded
   worker count. Keep workload-exit, machine control, telemetry and FlowMux
   resources separate. Test aggregate link/frame/buffer saturation across two
   real worker processes, matching counters and release after buffers/tasks end.
   The existing guest plaintext budgets remain independently owned; B4–B6 do
   not complete canonical aggregate plaintext admission or total host memory.
   Dependency: B5 and A2. Risk: high; review the exact MVM source/patch delta.
   This is a useful separate PR, with #43's broader acceptance explicitly open.

   Two additional separately reviewed commits are required for the full milestone:
   **B6a**, in `crates/spiralcore-mvm-live/src/host_budget.rs`,
   `crates/spiralcore-mvm-live/src/orchestrator.rs` and
   `crates/spiralcore-mvm-live/tests/host_budget.rs`, adds one stable private
   installation owner with OS-held exclusion and crash/orphan reconciliation;
   test competing orchestrators and owner death with live children before
   allowing `scope=host`. **B6b**, in `docs/api/persistent-host-budget-v1.md`,
   `crates/spiralcore-mvm-live/src/config.rs`,
   `crates/spiralcore-mvm-live/src/orchestrator.rs`,
   `crates/spiralcore-mvm-live/src/campaign/canonical.rs` and
   `crates/spiralcore-mvm-live/tests/host_budget.rs`, specifies and enforces
   conservative service partitions whose sum fits one installed host ceiling,
   or a shared authoritative plaintext service. Choose/review that contract
   before implementation; never import guest counters as authority. Test all
   process creation/default paths against aggregate refusal and owner fencing.
   Dependencies: B6a then B6b. Risk: high; general dynamic IPC remains deferred.

### C. Safe persistent sessions before removing the witness cap

7. **Bounded pending operation** — files
   `crates/spiralcore-mvm-live/src/campaign/canonical.rs`,
   `crates/spiralcore-mvm-live/src/campaign/session.rs` and
   `crates/spiralcore-mvm-live/tests/persistent_session.rs`.
   Extract a bounded session state machine with one pending operation and an
   explicit terminal outcome. Allocate `MessageOptions` once and retain exact
   message ID, scope, sequence, issue/expiry and nonce for retries; bound attempts
   by the existing receipt contract. RED tests refuse tracker/control admission
   before send, retry the original identity after capacity returns, and detect
   conflicting reuse. An unknown FIFO gap is not skipped on timeout. Dependency:
   A2; use B6a/B6b before claiming the host-wide canonical aggregate milestone.
   Risk: high.
8. **Authenticated event demultiplexing** — same session/test paths plus
   `crates/spiralcore-mvm-adapter/src/receipt.rs`,
   `crates/spiralcore-mvm-adapter/tests/receipt.rs` and
   `crates/spiralcore-mvm-adapter/tests/service_sender.rs`.
   Replace the assumption that exactly two incoming frames finish every Call
   with bounded current-request demultiplexing. Exercise receipt-before-Reply,
   Reply-before-receipt, duplicates, lost receipt, late earlier Reply, malformed
   and unknown control frames, revoked grant, deadline and cancellation. Keep
   receipt acceptance and application Reply distinct. No receipt-of-receipt,
   new ACK, unbounded receive loop or fresh sequence for an uncertain send.
   Before caller Late/Duplicate integration, follow the approved additive Rust
   [registered Reply observation contract](../api/registered-reply-observation-v1.md)
   and its linked C8 architecture addendum. First add focused RED contract tests
   for matched/unknown Replies, unchanged strict behavior, authentication errors,
   known semantic mismatch, expiry/commitment conflict and unchanged unknown-state
   accounting. Then implement `observe_registered_application_reply` and the
   strict wrapper with one canonical verification and no new retained state.
   `None` means only a genuinely unknown authenticated request; when no complete
   match exists, known correlation OR causation with failed complete-match
   constraints remains `Unauthorized`. Preserve ambiguous-match `Conflict` and
   all other existing errors; the strict wrapper maps `None` to `Unauthorized`.
   Finally add the caller Late-then-replayed-Duplicate regression and integrate
   those dispositions: Late skips correlation and sends its actual negative
   receipt; an unmatched Duplicate sends only its actual duplicate receipt.
   Neither advances current progress. AcceptedEphemeral requires `Some`, and
   only the authenticated current handle may advance Reply state. Retain the
   16-event bound and ordinary readiness/FIFO poll gate; enable no retry here.
   Upgrade both peers and the keyless relay: its current four-frame choreography
   cannot forward a retried Call while waiting for a lost reverse event, even
   with pipeline one. Use bounded independent directional pumps under the same
   transport owner, at most one reserved frame per direction, with no accounting
   lock held across I/O. Distinguish clean half-close from truncated framing and
   test cancellation, reverse progress and duplicate/reordered events through
   both direct and relayed sessions before enabling automatic retransmission.
   Dependency: C7. Risk: high.
9. **Retention and FIFO lifecycle** — files
   `crates/spiralcore-mvm-adapter/tests/service.rs`,
   `crates/spiralcore-mvm-adapter/tests/receipt.rs`,
   `crates/spiralcore-mvm-adapter/src/service/ordering.rs` and the session files.
   Add deterministic tests immediately before/at deadline, rate expiry and full
   tracker/service-retention expiry. Reclaim only terminal records whose safety
   window ended. Keep stable scope checkpoints; retain known terminal entries
   behind an unknown gap. Bound authorized owner/scope churn and explicitly
   refuse new scopes when full; reclaim checkpoints only through a reviewed
   owner/epoch-fencing rule, never LRU/idle expiry. Test replay/old handles after
   reclaim and sequence exhaustion without wrapping. Dependency: C8. Risk: high.
10. **Long-running native consumer** — files
    `crates/spiralcore-mvm-live/src/campaign/canonical.rs`,
    `crates/spiralcore-mvm-live/src/config.rs`,
    `crates/spiralcore-mvm-live/tests/persistent_session.rs` and
    `crates/spiralcore-mvm-adapter/examples/service_sustained.rs`.
    Retain the old 2,048-Call witness profile as a control; add an explicit bounded
    persistent profile with offered rate, stop time, operation/byte ceilings and
    current signed authorization. Remove the hard cap only for that qualified
    profile. Test more than 2,048 completed Calls without replacing services,
    trackers, ordering owners, broker or transport. Capacity refusal pauses new
    work and retries its retained pending identity after real expiry; no hidden
    quota/TTL increase or service reset. Dependency: C9 and B6. Risk: medium.

### D. One protocol, negotiated bounded pipeline

11. **Contract and fallback tests** — files
    `docs/specs/2026-11-03-persistent-mvm-service.md`,
    `crates/spiralcore-mvm-live/src/config.rs` and
    `crates/spiralcore-mvm-live/tests/persistent_session.rs`.
    Pin a mutually authorized profile/window before enabling more than one
    pending operation. Default to window one; refuse incompatible mandatory
    profiles and test explicit compatibility. Reuse canonical Calls, Replies and
    encrypted receipt Casts; no parallel envelope or signature domain. Dependency:
    C10 evidence and reviewed version/profile contract. Risk: high.
12. **Bounded window and scheduler** — session files plus
    `crates/spiralcore-mvm-adapter/tests/service_tcp.rs`.
    Add windows two then four behind explicit configuration, each separately
    reviewed. Bound pending handles, encoded bytes, worker tasks and application
    replies inside host/tenant/sender plus signed caps. One serialized reader
    authenticates/demultiplexes frames; writers retain FIFO/retry ownership.
    Require control progress while application credit is full. Test out-of-order
    Replies, gaps, duplicate/control floods, slow readers, backpressure,
    cancellation and shutdown with multiple pending Calls. Dependency: D11.
    Risk: high. Keep pipeline one as the matched control; do not add batching yet.

## Registered qualification cells

Before execution write compact registration and source/binary/profile identities
under `docs/network/evidence/2026-11-03-persistent-mvm-service/`. Rust measurement
entry points reuse the existing probe/campaign engines. No Mac payload collection,
duplicate Cargo target, new VM image or code outside Rust is needed.

| Cell | Scope and bounds | Required observation |
|---|---|---|
| Fast deterministic RED/GREEN | Two sessions, tiny in-memory objects, trusted clock, barrier-controlled worker | Blocked object work does not own dispatch; phase changes recheck authority/time; exact accounting rollback and stable retry |
| Cross-process transport owner | One supervisor, two real child worker processes, deliberately small transport envelope | Combined link/frame/buffer refusal/recovery; live buffer retains its link; scope is orchestrator and plaintext is excluded |
| Host-wide service gate | B6a/B6b only, competing orchestrators and child/owner crash, bounded reviewed service partitions | Installation exclusion plus actual aggregate service refusal; orphan reconciliation before new allocation; otherwise NOT_RUN |
| Logical baseline | 2/8/32 total logical endpoints, 1/4/16 pairs, pipeline one, 4 KiB objects | Retain the prior ten offered operations/s, one-second application profile as a comparable cell; keep production-retention cells separately named |
| Persistent retention | Same owner/session, >=4,096 completed Calls, at least two full production retention cycles, offered rate fits retained-state envelope | No reset/cap/TTL increase; bounded transient records and stable scopes; injected refusal recovers; old replay/receipt/handle stays harmless |
| Interference/churn | Logical 8/32, bounded hot/slow/cancelled/malformed peers plus fixed churn count | Healthy per-tenant progress; actual refusal cause; scope limit/fencing behavior; no silent FIFO checkpoint eviction |
| Wick native | Linux/Firecracker, at most two guests, at most two vCPUs and 2 GiB total allocated guest RAM, no NIC; existing owned disks/harness | Real cross-process owner and typed mailbox binding; >2,048 Calls on persistent profile; exact publication/readback, teardown and stale-epoch negatives |
| Pipeline comparison | Logical 2/8/32 then same Wick two-guest profile; window 1/2/4; raw, equally tuned prepared zstd, Combined | Matched byte strings/auth/object durability; per-window resource charge; actual benefit or loss with retained saturation evidence |

Register a hard maximum duration, payload/staging bytes, RSS, FDs and worker count
for each cell. For the persistent cell, choose rate/duration from production
retention and signed validity so 4,096 Calls and two complete retention cycles
are feasible; do not borrow the earlier probe's one-second service retention.
If existing authorization cannot cover the run, obtain a separately named
bounded fixture/renewal profile and test reauthorization instead of disabling
expiry. A refused cell is evidence; raising its quota starts a different cell.

Record actual two-way envelope/framing/socket counters and disclose exclusions;
planned application bytes remain separate. Report setup and sender/receiver CPU,
verified end-to-end completion and full boot/transfer/reopen/teardown lifecycle.
Record queue, dispatch-wait/hold, handler and commit p50/p95/p99, per-tenant
completion/refusal/incomplete counts, RSS/FDs, host reservations, guest local
usage, active Calls, retained identities/attempts, scopes, cache/staging growth
and post-teardown leases. Preserve CPU accounting domains; do not sum overlapping
process clocks or infer energy/send-only time. Every length and byte readback
must match, including empty, binary, zero, unrelated, repeat and sparse-change
objects. Use 1 MiB/16 MiB only where the registered memory/storage envelope fits.

## Review, checks and publication

- For each commit run the focused failing test, implement, rerun the affected
  target, then required locked release adapter/live checks, formatting and strict
  Clippy. Reuse `/Users/casey.allard/spiralcore/target` and bounded build jobs.
  Record actual commands/results against exact source; CI is not native runtime
  or WAN evidence. New patch/dependency identities need fresh qualification.
- Independent Rust/code and security reviews are required for proof detachment,
  host leases/process fencing, retry/replay reclamation and pipeline scheduling.
  Resolve findings before merging the exact reviewed source through a PR.
- Root updates #49/#43/#42 at implementation, measured qualification and delivery
  checkpoints, linking contracts, PR/commit and compact raw evidence. Keep these
  broad issues open while their acceptance gates remain unexecuted.
- Preserve `.serena/`, saved sessions, historical losses, existing VM disks and
  other contributors' files. Check free storage before runs; remove only owned
  disposable payload stores after exact verification and evidence custody.

## Acceptance tracking

Implementation checkpoint: product source `7531985`, [PR #54](https://github.com/R4-Laboratories/SpiralCore/pull/54).
The [recorded results](../network/persistent-mvm-service-results-2026-11-03.md)
retain 18 exploratory before/after host cells and five actual Firecracker cells.

- [x] A1/A2: detach application work, observe independent service dispatch, and
  reject post-work policy replacement/expiry before Reply preparation.
- [x] B4 and relay portion of B5: checked owner/tenant link/frame/buffer credits,
  preactivation partial rollback, malicious-input cleanup and actual two-guest
  link ownership. The budgeted profile explicitly excludes Stage One.
- [ ] A3: authenticated control-contention latency during actual object replay
  remains unmeasured; the serial transfer comparison cannot establish it.
- [ ] B6: installation-wide exclusion/orphan recovery and aggregate plaintext,
  cache and workspace enforcement remain unfinished.

Next source checkpoint: product `91baf28`, merged through [PR #56](https://github.com/R4-Laboratories/SpiralCore/pull/56).
The [retained-identity results](../network/persistent-session-identity-results-2026-11-03.md)
record 22 host cells (11 before/after pairs) and three additional native cells.

- [x] C7 registration portion: original options/envelope/partial handles remain
  owned through Full; substituted frames are Busy; pre-send monotonic lifetime
  does not restart after wall-clock rollback.
- [x] Current registered-send boundary checks original generation/request under
  the existing policy lock; invoked send remains uncertain on failure.
- [x] Receipt-only completion cannot clear pending identity before a Reply;
  actual caller RED/GREEN regression retained.
- [x] C8 relay prerequisite: [PR #58](https://github.com/R4-Laboratories/SpiralCore/pull/58)
  replaces fixed four-frame forwarding with two bounded directional pumps,
  safe half-close and Darwin shutdown recovery. [Six native cells](../network/duplex-relay-results-2026-11-03.md)
  retain 54 exact publications/readbacks, CPU costs and final seven-client CI.
  This does not implement endpoint retry or complete C8.
- [x] C8 bounded endpoint portion: retained current receipt/Reply/ACK progress,
  FIFO waiting, authenticated Late/Duplicate handling and exact receiver cached
  Reply replay. [PR #60](https://github.com/R4-Laboratories/SpiralCore/pull/60)
  and its [recorded results](../network/mailbox-session-events-results-2026-11-03.md)
  retain 22 matched host cells, four native cells, all losses and scoped tests.
  An incomplete receiver fences before another socket read; pipeline one and
  the 2,048-Call guard remain. This is not full C8 completion.
- [ ] C8 recovery portion: bounded idle receive, original-identity retransmission,
  uncertain-send/ACK reconciliation, lost-event and direct/relayed duplicate
  recovery remain before automatic post-send retry. No reconnect/resume or new
  request sequence is implicitly permitted by the delivered event loop.
- [ ] C9/C10 autonomous idle expiry, production-retention cycles and same-owner
  native sessions beyond 2,048 Calls remain unexecuted.

- [ ] A: expensive object work leaves global dispatch; concurrent control and
  expiry/revocation regressions pass on the actual path.
- [ ] B4–B6: one orchestrator enforces combined transport bounds across links to
  actual worker processes, with guard lifetime, refusal and cleanup evidence.
- [ ] B6a/B6b: installed host exclusivity/crash recovery and reviewed aggregate
  service partitions or service authority pass; otherwise host-wide canonical
  service aggregation remains open regardless of transport success.
- [ ] C: persistent canonical session completes >2,048 Calls with safe original
  retry identity, full-retention recovery, bounded churn and retained FIFO proof.
- [ ] D: negotiated bounded windows improve a stated workload or retain an
  explicit loss; pipeline one remains supported under the same protocol.
- [ ] Logical 2/8/32 and constrained Wick two-guest evidence is source-pinned and
  reviewed, with exact fidelity and complete actual-byte/CPU/time/resource rows.

This plan is not executed evidence. macOS native/resource-pressure recovery,
native Windows durable gateway, direct/explicit-relay canonical WAN, durable
mailbox recovery and the full capacity envelope remain separately unqualified
until their own acceptance evidence exists.

## Current checkpoint

The planner independently reviewed both linked 2026-11-03 contracts. The first
transport API is coherent for its declared orchestrator/ciphertext scope; no
host-wide plaintext or total-memory endorsement follows from that review.
The implementation owner reports the private `application_phase` regression
failed against the old dispatch-held behavior at its two-second watchdog and
passed after detaching the phase. This is an initial focused RED/GREEN report;
the planner did not independently rerun builds/tests. Independent Rust/source
review found no blocking defect in the canonical handler split at file SHA-256
`65128439c949c07abb6ae4f6b754f02d898d0652e847488087c145edb71d3cfe`:
the admitted proof retains its live/policy/deadline/owner checks, and fresh
post-work reply preparation rechecks current generation, deadline and epoch.
Required exact-source test/security review records remain pending.
Actual authenticated control-progress measurement and constrained native
qualification remain pending. Acceptance A remains unchecked.
