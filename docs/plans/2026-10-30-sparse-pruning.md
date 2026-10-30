# Implementation plan: SC-OPT-007 sparse candidate pruning

Baseline: `cc0199ffd255b9e2e002afe8c96f10462a3e474f`.
Design: [safe sparse-pruning decision](../specs/2026-10-30-sparse-pruning-design.md).
Status at plan creation: implementation/performance/allocation gates NOT_RUN.
The user approved this bounded next step and the existing proposal's gates.
Production changes are confined to sparse candidate rejection; explicit and
adaptive byte decisions must remain identical.

## Phase 1: freeze the reference and prove the tests

1. **Immutable oracle** (`crates/spiralcore-transfer/tests/support/delta_legacy_cc0199f.rs`
   and `tests/support/protocol_legacy_cc0199f.rs`). Preserve the baseline files
   byte-for-byte, independently of the new rejection logic; adapt imports only
   through a wrapper. Record original blob/hash and wrapper identity in
   `docs/network/evidence/2026-10-30-sparse-pruning/oracle.json`. Verify the oracle
   matches the unchanged production encoder before editing it. Dependency: none.
   Risk: medium; a shared changed helper would invalidate independence.
2. **Behavioral RED** (`crates/spiralcore-transfer/src/delta.rs` test module,
   `crates/spiralcore-transfer/tests/sparse_pruning.rs`). Add the smallest relevant
   failing rejection/identity test and observe its failure before implementation.
   Retain the command and expected reason. Assert recipe bytes and operation count,
   not merely that decoding succeeds. Dependency: step 1. Risk: medium.
3. **Baseline measurement qualification**
   (`crates/spiralcore-transfer/examples/sparse_pruning.rs`). Establish the native
   harness against the unchanged encoder/oracle and fixed workloads. Confirm
   fixtures, input ownership, process ordering, timer scopes and result identities
   before the production change. Dependency: step 1. Risk: medium.

The pre-edit baseline diagnostic contains 21 observations in `baseline.jsonl`.
It establishes the recorded starting behavior and CPU/wall samples; it is not a
full profiler attribution or a claim about what percentage of CPU belongs to
packing. The registered paired acceptance remains a separate subsequent run.

## Phase 2: minimal production change

4. **Checked bound helper** (`crates/spiralcore-transfer/src/delta.rs`). Add the
   positive-operation bound using checked totals and format-derived constants.
   Keep it private. Test empty/positive operation lists and prove `LB <= packed
   cost` over deterministic generated cases. Do not assert the formula over
   counted zero-length operations skipped by `pack`. Dependency: step 2.
   Risk: high because a wrong overestimate can change the selected encoding.
5. **Pre-pack rejection** (`src/delta.rs`). Calculate Raw cost before packing.
   Skip a candidate only when its bound plus the 32-byte base cost cannot beat
   Raw. Additionally skip aligned packing when its bound cannot strictly beat
   the incumbent's actual cost. Keep final exact comparisons, fallback filter,
   operation generation and packer unchanged. Dependency: step 4. Risk: high.
6. **Preservation matrix** (`tests/sparse_pruning.rs`, existing `tests/sparse.rs`,
   `tests/adaptive.rs`). Compare errors/None, every frame byte and operation count
   with the frozen oracle, then apply each recipe and compare exact target bytes.
   Cover empty/one-byte/31/32/33-byte inputs, embedded/trailing zeros, all edit
   geometries, unrelated data, structured and opaque compressed inputs, fixed-seed
   edits, literal bounds 61,439/61,440/61,441, frame operation counts 255/256/257,
   and overall counts 8,191/8,192/8,193 including splitting. Exercise simple wins,
   aligned wins, strict ties, Raw boundary and both candidates losing.
   Dependency: step 5. Risk: medium.
7. **Sender invariants** (`tests/sparse_pruning.rs`, `tests/adaptive.rs`). Prove
   unchanged explicit/adaptive mode, candidates/costs, uploaded frames and replies
   with equal inputs; retain missing/evicted base, authorization, repeat/probe,
   malformed reply, no reoffer and restart fidelity cases. Keep the sender's
   16 MiB and core's 64 MiB bounds. Dependency: step 6. Risk: high.

Each production edit should remain reviewable on its own: bound helper, simple
candidate gate, aligned incumbent gate. Do not mix serializer refactoring, renamed
wire constants elsewhere, candidate generation changes or zstd-prefix skipping.

## Phase 3: separate diagnostic allocation qualification

8. **Diagnostic allocator** (`examples/sparse_pruning.rs` or its diagnostic-only
   support module). Compile only under a named custom diagnostic configuration;
   the normal timing binary keeps the normal allocator. Use the bounded TLS
   registry and frame-header allocation classification specified in the design.
   Track actual requested capacities and alloc-copy-free overlap under the same
   forced-moving diagnostic policy for both arms. Discovery stops on return;
   registered retained buffers are followed through drop. Dependency: phase 2.
   Risk: high; all unsafe instrumentation stays outside production runtime.
9. **Accounting tests** (diagnostic harness test module). Test new allocation,
   growth, move, failure, frees, zero final live bytes, retained return buffers,
   registry overflow, unknown/stale lifecycle and unrelated-allocation exclusion.
   Qualify the unique nine-byte/alignment-one frame-header classification in both
   engines. Compare diagnostic output byte-for-byte with normal output. Fail
   closed on incomplete instrumentation. Dependency: step 8. Risk: high.
10. **Resource result contract**
    (`docs/network/evidence/2026-10-30-sparse-pruning/registration.json`). Declare
    peak/cumulative requested frame capacity, growth overlap, retained-at-return
    capacity and drop lifecycle. Record that this is controlled diagnostic-policy
    memory, not normal allocator peak, internal usable size or process RSS.
    Dependency: step 9. Risk: medium.

## Phase 4: frozen paired experiment

11. **Freeze fixtures and sources** (same `registration.json`, source manifests
    and harness). Pin baseline/candidate/oracle, compiler/features, normal and
    diagnostic binary hashes, deterministic generator/seeds and the seven cells.
    Require the three primary fixtures to demonstrate intended rejection and
    exact legacy parity before timing. Freeze any repeated-call policy for tiny
    clock values now. Dependency: phases 2-3. Risk: medium.
12. **Register exact schedule** (same packet). For each primary/control cell,
    run ten paired fresh-process comparisons: interleaved AB/BA, five of each.
    Keep raw pair IDs/order/results and no-replacement rules. The three primaries
    are 4 KiB prefix insertion, 4 KiB middle insertion and unrelated replacement;
    the four controls are repeat, localized 32-byte edit, aligned-winning scattered
    edits and periodic structured content. Targets stay at 16 MiB; shorten
    insertion bases accordingly. Dependency: step 11. Risk: medium.
13. **Timed normal run** (`examples/sparse_pruning.rs`, evidence JSONL). Measure
    CPU/wall from encoder entry through return only. Verify the actual result
    outside that interval, then measure destruction separately. Record the
    verification gap and contiguous entry-to-release observation without treating
    it as encoder performance; any encode-plus-drop sum is explicitly disjoint.
    No owned build/test/profile runs concurrently. Preserve failures and host
    interference limits. Dependency: step 12. Risk: medium.
14. **Untimed diagnostic run** (diagnostic executable, evidence JSONL). Record
    qualified frame allocation lifetimes under the controlled allocator, including
    proof that bound-rejected candidates allocate/materialize no serialized frame
    bytes. Do not use instrumented CPU timings as the performance result.
    Dependency: steps 9 and 12. Risk: medium.
15. **Independent analysis**
    (`docs/network/evidence/2026-10-30-sparse-pruning/independent-review/`). Verify
    exact row/pair order, fixture/result/binary identities, all observations,
    geometric mean ratios, strict win counts and every resource/control gate.
    Qualify the checker with missing/duplicate/reordered/wrong-result/changed-cost
    and incorrect-summary cases. Dependency: steps 13-14. Risk: high.

## Unchanged gates and disposition

| Gate | Required result |
|---|---|
| Exact preservation | Same error/None or ordered frame bytes and operation count; exact reconstructed target and unchanged adaptive choice |
| Every primary CPU cell | Geometric mean legacy/candidate CPU at least 1.10 and at least 7 strict candidate wins of 10 |
| Every primary memory cell | At least 50% lower peak requested frame-buffer capacity under the qualified diagnostic policy; zero materialization for rejected candidates |
| Every control CPU cell | Geometric mean candidate/legacy CPU at most 1.05; no favorable reinterpretation as reverse ratio 0.95 |
| Resource/trust behavior | Existing limits, pre-Begin behavior, authorization, no hidden retry and exact publication remain intact |

Wall distributions and encode/drop/gap scopes are always reported, including
regressions. No CPU, memory, end-to-end, WAN or energy benefit is assumed before
these results exist. A failed primary/control/memory/correctness gate is retained;
do not change a threshold or replace observations to obtain a pass. Revert/defer
an unqualified production candidate while keeping its source and negative evidence.

## Phase 5: review, record and authorized delivery

16. **Fresh qualification and reviews** (Cargo targets and packet receipts). Run
    appropriate full tests, ignored large tests, doctest, format, Check/Clippy and
    release builds before the timed run; rerun affected checks only if source
    changes. Obtain independent Rust/code/security and diagnostic-allocator review.
    After measurement, confirm unchanged source, complete pipeline audit and final
    code/report gate. Dependency: phases 2-4. Risk: high.
17. **Report and preserve** (`docs/network/sparse-pruning-results-2026-10-30.md`,
    `docs/network/optimization-ledger.md`, `docs/network/delivery.md`, evidence
    packet). Record every pass/failure, baseline/candidate identities, CPU/wall
    distributions, diagnostic memory scope and adoption/revert decision. Keep
    SC-OPT-006 and prior WAN evidence untouched. Dependency: step 16. Risk: medium.
18. **Publish qualified work** (existing isolated branch and authorized main
    delivery path). Seal and verify evidence hashes, recheck current origin/main,
    preserve user originals and publish only after the stated gates. Verify the
    actual remote revision afterward. Dependency: step 17. Risk: medium.

No new infrastructure, other-lab reconfiguration or MVM mailbox binding belongs
to this task. A passing local pruning result only justifies a later separately
registered adaptive/WAN comparison; it is not itself that comparison.
