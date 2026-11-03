# Sustained broker operation and recovery

Status: implementation design under the user-approved autonomous roadmap.
Base: SpiralCore `95fc3c45dc849ccdb6d766b6b641fb4c782b4d54`.
Delivery authority: [#49](https://github.com/R4-Laboratories/SpiralCore/issues/49),
within [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).

## Decision

Keep the existing canonical mailbox protocol, shared codec core, signed grants,
aggregate limits and retention periods. Add bounded local diagnostics, qualify
expiry/recovery, and exercise independent concurrent sessions before choosing a
negotiated pipeline or dispatch-sharding change. This extends the published
[broker measurements](../network/evidence/2026-11-02-broker-scale/README.md).

Alternatives considered: raising ceilings only moves the observed limit;
retiring replay/receipt state early changes safety; a protocol/window redesign
before identifying the limiter cannot establish its benefit. Local diagnostics
and measured recovery are the smallest useful next implementation.

## Source-backed system map

- `crates/spiralcore-mvm-adapter/src/broker.rs`: cloneable shared budget and clock;
  a synchronous dispatch mutex samples time after acquisition. Socket I/O must
  remain outside dispatch. It supplies serialization, not a fair scheduler.
- `crates/spiralcore-mvm-adapter/src/service.rs`: `HostBudget` charges total and
  data/control portions at host/tenant/mailbox/sender scopes and signed-grant
  contributions. Several distinct failure branches currently return `Full`.
  Arrival/rate histories retain identities for 60 seconds; service accepted and
  Call retention is separate. FIFO scopes remain persistent replay authority.
- `crates/spiralcore-mvm-adapter/src/receipt.rs`: `ReceiptTracker` retains entries
  until the request deadline plus 300,000 ms, including terminal observations.
  Entries own control-identity reservations; `expire` releases them. Its local
  ceiling is at most 4,096 entries. Successful receipt consumption does not
  immediately free this retained identity.
- `crates/spiralcore-mvm-adapter/examples/service_capacity.rs`: authenticated TCP
  loopback, one whole operation at a time, alternating Calls and generic Casts.
  Its prior 65-second results did not cross the tracker retention cycle.
- `crates/spiralcore-mvm-adapter/examples/service_capacity/fixtures.rs`: existing
  synthetic signatures expire after 240,000 ms. A full-cycle real-time run needs
  an explicitly named longer-lived synthetic fixture profile; changing a test
  fixture does not extend production capability validity.
- `crates/spiralcore-mvm-live/src/campaign.rs` and `campaign/canonical.rs`: shared
  object-v1 raw/whole-zstd/combined arms with exact durable object checks. The
  native campaign retains its explicit 2,048-Call witness bound.

The relevant scaling risk is bounded retained-state exhaustion followed by
unclear attribution. No source finding justifies removing retention or silently
evicting ordering checkpoints. The global dispatch mutex also permits contention;
real concurrent measurements are required before making throughput claims.

## Trusted local diagnostic contract

Add one fixed-size last-refusal record and a saturating refusal counter to the
shared budget snapshot. Identify the actual first failing branch with scope
(`Host`, `Tenant`, `Mailbox`, `Sender`, `Capability`, `Tracker`, or `Service`),
pool (`Total`, `Data`, `Control`, `Signed`, or `Local`), resource (queued bytes,
messages, active Calls, identities, rate or tracker entries), and current,
requested and limit values. Values use checked conversion or explicit saturation;
instrumentation must not turn a refusal into success or mutate admitted usage.

`ReceiptTracker` exposes read-only aggregate entry/attempt/retention counters.
These are trusted-owner APIs. Do not include tenant identifiers, message IDs,
payloads, keys or cache membership, and do not add diagnostics to encrypted
envelopes or remote receipt outcomes. Existing `ServiceError` and canonical
`DeliveryOutcome` behavior stays compatible. A diagnostic is a causal report of
an actual branch, not a speculative check against current usage.

The last record is global across sessions. A recorder must capture its sequence
before and after a failing local dispatch and attribute only a new record from
that dispatch; it must not associate a stale record with unrelated auth or I/O
failure. Tests exercise each implemented category and accounting nonmutation.

## Recovery and concurrency contract

Use trusted-clock boundary tests at deadline, 60-second rate expiry, and full
300-second tracker extension. Supplement them with a bounded real-time run
crossing the full extension: no context/service/tracker reset, no limit increase,
stable ordering scopes, exact payloads, retained old-handle/replay behavior and
successful new admission after expiry. Report persistent owner/scope counts
separately from expiring transient records. A bounded steady population is not
proof that arbitrary tenant/scope churn is reclaimed.

Use independent endpoint-pair workers sharing one `BrokerContext` for 2/8/32
total logical endpoints. Each pair retains pipeline one; pairs may progress
concurrently. Bound worker count, operation counts, duration, payload memory,
descriptors and output size before starting. A slow transport wait must not own
dispatch. Hot, slow, cancelled and malformed sessions run beside healthy ones;
record per-tenant completion, refusal and p50/p95/p99 latency rather than only
aggregate throughput. Classify fairness empirically; `std::sync::Mutex` does not
promise a scheduler fairness guarantee.

The test profile and offered rate must fit the measured retention envelope.
Saturation is an explicit measured result, never an invitation to raise limits.
Keep cancelled/incomplete admission separate from completed application exchange.
All compared payloads must match byte-for-byte and length-for-length.

## Object and deployment measurements

Reuse the existing object-v1 and campaign APIs for mixed empty/binary/zero,
compressible, unrelated, exact-repeat and sparse-change objects, including
1 MiB and 16 MiB where admitted. Compare raw, equally tuned prepared whole-zstd
and SpiralCore selection through matching authentication/durability. Count actual
both-direction traffic, setup/seeding, CPU and verified completion; separate
codec, mailbox admission and durable object commit. No codec copy or substitute
private MVM carrier is allowed. Store generated payloads in bounded temporary
owned storage and remove them after exact verification.

Native guest/host and two-guest runs on supported Linux/Firecracker and macOS
backends follow local qualification. Direct and explicit-relay WAN, including
supported Windows client/gateway roles, remain distinct executed gates. Record
missing host/backend/credentials or failed runs as unavailable/failed; CI client
loopback cannot stand in for native MVM or WAN evidence. No new paid resource or
larger VM allocation is implied by a logical-endpoint test.

## Acceptance and delivery

Focused RED/GREEN tests cover diagnostic accuracy, exact boundary expiry,
retained safety and renewed admission. Runtime evidence must identify source,
binary, host, profile, offered load and limits; retain losses/incomplete work.
Independent Rust/code and security reviews cover changed accounting/diagnostics
and concurrency. Publish compact evidence and merge reviewed validated slices
through PRs; keep #49 and the parent open until their broader criteria pass.

Use the existing shared Cargo target, bounded build jobs and compact JSON/Markdown
records. Preserve saved sessions, historical evidence, `.serena/`, other labs and
existing VM disks. No duplicate dataset, build tree or new VM image is needed for
local broker qualification. No energy, certification or universal speed claim
follows from these measurements.
