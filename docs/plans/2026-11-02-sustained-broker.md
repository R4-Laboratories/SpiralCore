# Plan: sustained broker operation and recovery

Design: [sustained broker](../specs/2026-11-02-sustained-broker-design.md).
The user approved the roadmap and autonomous implementation on 2026-11-02.
Reuse isolated branch `codex/spiralcore-sustained-broker` from `95fc3c45`.
Each phase is independently reviewable. GitHub #49 owns the capacity milestone;
#42/#43 retain the larger MVM delivery and canonical service requirements.

## Phase A — Causal local diagnostics

1. File `crates/spiralcore-mvm-adapter/tests/service.rs`: add one focused RED
   assertion per actual host/tenant/mailbox/sender/class/signed/rate refusal and
   unchanged admitted accounting. Dependency: none. Risk: medium.
2. File `crates/spiralcore-mvm-adapter/src/service.rs`: define fixed-size public
   diagnostic enums/record and saturating counter in `BudgetSnapshot`; instrument
   the first actual failing quota branch. Keep wire and error variants intact.
   Make each branch edit independently testable. Dependency: A1. Risk: medium.
3. Files `crates/spiralcore-mvm-adapter/tests/receipt.rs` and `src/receipt.rs`:
   RED then add aggregate tracker snapshot and a causal local-ceiling refusal.
   Exercise full tracker, no accounting mutation, stale diagnostic distinction,
   and no identity/payload exposure. Dependency: A2. Risk: medium.
4. Files `crates/spiralcore-mvm-adapter/examples/service_capacity.rs` and
   `tests/capacity_probe.rs`: capture diagnostics around the actual failing
   dispatch; preserve original error and stage, and explicitly distinguish no
   new quota record. Keep prior results immutable. Dependency: A3. Risk: low.

## Phase B — Full retention-cycle recovery

1. Files `crates/spiralcore-mvm-adapter/tests/receipt.rs` and `tests/service.rs`:
   add deterministic RED/GREEN tests at one tick before/at tracker expiry and
   rate-history expiry, followed by exact new admission without owner reset.
   Assert terminal/tracker safety and retained FIFO scope/replay authority.
   Dependency: A. Risk: high because expiry releases shared reservations.
2. File `crates/spiralcore-mvm-adapter/examples/service_capacity/fixtures.rs`:
   add an explicit longer-lived bounded synthetic fixture profile for >5-minute
   runs. Preserve the historical short profile and production defaults. Record
   fixture expiry in output. Dependency: B1. Risk: low.
3. Files `crates/spiralcore-mvm-adapter/examples/service_sustained.rs` and
   `tests/sustained_probe.rs`: add a bounded real-time recovery mode, diagnostic
   sampling and exact completion accounting. Use small payloads and stable scope;
   saturate a registered ceiling, wait/expire through its real retention period,
   resume on the same services/transports and verify bytes. Tests use short
   functional mode; the recorded >5-minute run remains explicit. Dependency:
   B1/B2. Risk: medium.
4. Folder `docs/network/evidence/2026-11-02-sustained-broker/`: preregister limits,
   run/source/binary/host/profile identities and full-cycle success/failure gates,
   then record compact actual rows. Include transient versus persistent state.
   Dependency: B3. Risk: low; no inferred all-state plateau.

## Phase C — Concurrent pairs and interference

1. Files `examples/service_sustained.rs` and `tests/sustained_probe.rs` in the
   adapter crate: RED then add bounded worker-per-pair execution sharing the
   broker, synchronized start and pipeline one per pair. Socket waits stay
   outside dispatch. Exercise 2/8/32 total endpoints with real TCP loopback.
   Dependency: B. Risk: medium; output must retain panic/timeout failures.
2. Same files: independently test and add hot-sender, slow-transport,
   cancellation and malformed-input scenarios beside healthy tenants. Reuse
   canonical verification; do not introduce an alternate envelope. Record each
   tenant's bytes, completed/incomplete/refused operations and latency, plus
   aggregate CPU/RSS/FDs. Dependency: C1. Risk: high for trust-boundary behavior.
3. Evidence folder from B4: register bounded mixed interference profiles and
   pass/fail criteria before runs; retain fairness losses and quota attribution.
   Do not call the logical count a booted-MVM population. Dependency: C2.
   Risk: low. If a gate fails, reproduce with a focused regression before repair.

## Phase D — Mixed object transfer through the same core

1. Files `crates/spiralcore-mvm-live/tests/campaign.rs` and
   `crates/spiralcore-mvm-live/tests/combined_recovery.rs`:
   add exact mixed-session/object recovery checks first, using raw/whole-zstd/
   combined arms and the injected shared broker. Preserve the native Call cap.
   Dependency: C. Risk: medium.
2. Files `crates/spiralcore-mvm-live/src/campaign.rs`, `campaign/canonical.rs`
   and existing measurement entry points: extend only the bounded measurement
   options needed for paired mixed-object runs. Prefer reuse over new product
   APIs; no copied codec. Dependency: D1. Risk: medium.
3. Evidence folder from B4: record 1 MiB/16 MiB admitted objects, cold/warm/
   unrelated/sparse paths, equal authentication/durability raw/zstd comparisons,
   both-direction actual bytes, setup/seeding, CPU and commit time. Store compact
   records; delete only verified owned disposable object stores. Dependency:
   D2. Risk: low. Retain every measured loss.

## Phase E — Native and WAN qualification

1. File `docs/network/delivery.md`: enumerate available supported native host/client
   combinations and admitted resource budgets before execution. Use existing
   prepared/pinned MVM source and host harness. Dependency: D. Risk: medium.
2. Files `.github/workflows/spiralcore-linux-mvm.yml` and the existing native
   campaign entry points under `crates/spiralcore-mvm-live/src/`: run Linux/Firecracker
   guest/host then two guests and macOS native equivalent within authorized host
   resources. No guest NIC, alternative egress or changes to other labs.
   Record source/binary identities, negatives, teardown and exact readbacks.
   Dependency: E1. Risk: high. Missing backend remains an explicit unavailable gate.
3. Files `.github/workflows/spiralcore-wan.yml`,
   `.github/workflows/mvm-mailbox-portable.yml` and existing WAN harness entry
   points: execute supported direct/explicit-relay
   combinations with the same canonical protocol, including Windows client roles
   where a native runner exists. Compare matched baseline arms and record actual
   traffic, endpoint CPU, completion and recovery. Dependency: E2. Risk: high.
   No invented remote credentials, paid resources or native-Windows claim from WSL.

## Validation, review and publication for each slice

- Run changed-target RED/GREEN then locked release adapter all-feature/all-target
  and affected live tests, formatting, strict Clippy and exact-source checks.
- Independent Rust/code and security reviews cover diagnostic/accounting and
  concurrent paths; infra review applies if workflow files change. Keep scope
  focused. Preserve compact measurement evidence even if a cleanup skill would
  normally delete debug logs; the user explicitly requires recorded results.
- Commit plan/design and implementation/evidence checkpoints; update #49 and
  parent #42 at meaningful stages. Merge through a reviewed PR to `origin/main`.
  Do not close broad issues for a local-only slice or unexecuted platform gate.
- Check free disk before builds/runs. Reuse `/Users/casey.allard/spiralcore/target`,
  bounded jobs and small generated payloads. Preserve user/other-lab state.

## Acceptance checklist

- [x] Exact causal saturation reason without changing wire errors or authority;
  instrumented branches are explicit, other refusal paths remain unattributed.
- [x] Deterministic expiry boundaries and real complete tracker-cycle recovery.
- [x] Concurrent Linux 2/8/32 logical endpoint and interference results with zero
  mismatch; the failed Mac one-second cell and deferred cells remain recorded.
- [x] A bounded Linux offered-load profile and all saturation/losses recorded;
  this is not a production maximum-throughput or fleet capacity envelope.
- [x] Mixed-object raw/zstd/SpiralCore measurements under matched requirements,
  including seven actual Linux native sessions and the shared-budget refusal.
- [x] Available Linux native gates executed; Mac resource-pressure deferral,
  canonical WAN and Windows durable-gateway unavailable gates explicitly retained.
- [x] Reviewed/tested code merged through PR #52 at `111ceca309f25b7538f8a49e9af4ca355fdcabc0`;
  this documentation slice publishes the complete linked evidence and roadmap update.

The selected milestone qualifies the stated Linux profile and recovery mechanism.
The Mac one-second eight-endpoint profile is not accepted. Explicitly recording
an unavailable native/WAN gate preserves the roadmap but does not convert that
broader delivery gate into success. #49 and #42 remain open for production
retention, native Call lifetime, large-object scheduling, cross-process budget
ownership, churn, platform and direct/relay WAN release requirements.

## Execution record

- Plan `0ed221f`; diagnostics/probe `d53cb335`; raw mixed harness `03f05ab7`;
  failure recorder `d940203`; native preregistration/runner `6257930`.
- [Results and retained losses](../network/sustained-broker-results-2026-11-02.md),
  [source/binary identities, raw evidence and reviews](../network/evidence/2026-11-02-sustained-broker/README.md).
- Linux load: 13,684 complete operations, 20,537 exact readbacks, zero mismatch,
  two explicit hot incomplete admissions. Native: 62 publication operations and
  44 keyless readbacks over seven successful sessions.
- Final code CI: native clients 504 plus eight Unix storage passes; Linux 47
  release and 112 shared conformance passes, with overlapping scopes disclosed.
- No new Mac payload store or VM disk. Seven verified owned native payload
  stores were removed on Wick after archiving. Failed stores remain identified.
