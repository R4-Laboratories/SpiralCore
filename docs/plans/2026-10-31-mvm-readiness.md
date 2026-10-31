# Implementation plan: canonical MVM envelope readiness

Use the approved [adapter design and contract](../specs/2026-10-31-mvm-readiness-design.md)
and existing [SCMOBJ01 application contract](../contracts/spiralcore-object-mailbox-v1.md).
The user authorized the canonical mailbox direction and now requests actual
implementation toward MVM. Freeze codec policy; no new speculative CPU campaign.
Each helper/test below should be implemented in focused increments, ordinarily
under 50 lines per helper, with the observed failing test before its implementation.

## Phase 1: independent crate and real canonical envelope path

1. `crates/spiralcore-mvm-adapter/Cargo.toml`, `Cargo.lock`: create a standalone
   crate, exact-pin mvm-core/mvm-contract at `3104a12c5b071a6851323fa8115d2afaf3f0bb8b`,
   defaults off, and local spiralcore-transfer. Use Rust 1.97.1/shared target,
   no full upstream workspace build. Run metadata/dependency qualification before
   implementation. Dependency: committed design. Risk: medium, dependency size.
2. `crates/spiralcore-mvm-adapter/tests/canonical.rs`: add the smallest failing
   contract test: real canonical sealed request and reverse reply carrying one
   existing frame through a bounded upstream in-memory transport. Assert exact
   payload and response frame identity; a raw plaintext route must not exist.
   Dependency: phase 1.1. Risk: medium, authority/correlation boundary.
3. `crates/spiralcore-mvm-adapter/src/lib.rs`: implement typed bounded adapter
   helpers around prepare/authorize/send/decode/open. Reuse upstream types and
   crypto; bind independently authorized directions. Add an explicit process-local
   bounded replay implementation and typed error mapping, no auto-retry. Dependency:
   observed phase 1.2 RED. Risk: high, trust boundary; independent security review.
4. Same test/module: add negatives one behavior at a time for oversized input,
   malformed frame, altered ciphertext, recipient/content/capability/epoch mismatch,
   revoked grant, queue full and cancelled delivery. Ensure endpoint dispatch is
   absent on every failed authentication/admission path. Dependency: phase 1.3.
   Risk: high; preserve upstream error semantics rather than synthetic success.

## Phase 2: existing transfer engine and recoverable application retry

5. `crates/spiralcore-mvm-adapter/src/lib.rs`: derive SpiralCore context only
   after open, binding verified grant and current authority. Add an exchange seam
   compatible with the existing `WarmSession`/client batch callback, serializing
   one bounded frame per canonical envelope. Dependency: phase 1 qualification.
   Risk: high, namespace isolation and order; no core codec changes.
6. `crates/spiralcore-mvm-adapter/tests/transfer.rs`: failing integration test
   for real durable seed plus localized warm update through canonical envelopes,
   exact reopened bytes/length and unchanged selected application frames. Cover
   append/truncate and an unrelated replacement without changing selector policy.
   Dependency: phase 2.5 API contract. Risk: medium, object/resource accounting.
7. Same tests/library: test duplicate canonical envelope rejection, new-envelope
   application retry, interruption after open/before endpoint, lost Published
   reply and explicit session recovery after restart. Add only the required
   helpers/fault-injection test seam. Do not claim durable inbox semantics.
   Dependency: phase 2.6. Risk: high, separate acknowledgment boundaries.

## Phase 3: executable handoff and recorded bounded comparison

8. `crates/spiralcore-mvm-adapter/examples/mailbox_transfer.rs`: drive prepared
   WarmSession and prepared Whole zstd through the same canonical adapter, at
   1 MiB and 16 MiB with cold seed, localized update, unrelated replacement and
   mixed persistent sends. Record both frame and envelope byte totals, setup,
   sender/receiver work where separately measurable, process CPU and elapsed.
   Preserve synthetic fixture/source/executable IDs and raw rows. Dependency:
   phase 2 exactness. Risk: medium; do not call local time WAN time.
9. `docs/network/evidence/2026-10-31-mvm-readiness/`: register the bounded dose
   and source before timing; one campaign, no overlapping builds, retain failures.
   Stop/reopen every store for exact readback, then delete only those owned stores.
   Reuse targets, disable incremental, maintain a 4 GiB disk floor. Dependency:
   frozen example and focused independent review. Risk: low, evidence custody.
10. `crates/spiralcore-mvm-adapter/README.md`, `docs/network/mvm-readiness-results-2026-10-31.md`,
    `docs/network/delivery.md`, `docs/network/optimization-ledger.md`: document
    executable commands, measured overhead and actual passed/failed scope. Pin
    upstream APIs and list #4629/#4633, verified reply handles and durable inbox
    as remaining live-service dependencies. Dependency: retained results.
    Risk: low; distinguish host adapter from MVM guest delivery.

## Qualification and completion

Use focused RED/GREEN receipts while developing, then this standalone crate's
all-target tests, Clippy and example release build once stable. Independently
review Rust/input/security and result arithmetic; reuse existing core qualification
when its files are unchanged. No new broad repository audit. Complete means a
working canonical-envelope host adapter, exact WarmSession transfer/recovery
tests, recorded byte/CPU/elapsed results and a precise guest integration handoff.
It does not mean MVM vsock/Iroh is implemented upstream, production security,
FIPS validation, or a new WAN qualification. Publish by reviewed PR under the
existing authorization, preserve original local runtime/saves and raw evidence.
