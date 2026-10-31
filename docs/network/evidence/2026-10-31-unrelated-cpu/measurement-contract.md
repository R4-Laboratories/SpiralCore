# SC-OPT-019: mixed persistent before/after measurements

This contract follows the committed [design](../../../specs/2026-10-31-unrelated-cpu-design.md)
and [plan](../../../plans/2026-10-31-unrelated-cpu.md). The production baseline is
`56e15db5c77fe2ecdc03476d8e09664cd4213fd1`. Freeze the common measurement source,
the exact baseline production overlay, candidate source and both executables
before either comparison campaign. Any approved additional production-file
change must appear explicitly in that overlay and its source profiles; a
delta-only overlay cannot stand in for an older schema implementation.

## Baseline stage diagnostic

The independent profile has three fixed trial seeds, two exact sizes (1 and
16 MiB), localized and unrelated families, and six stages: **72 observations**.
Stages are delta encoding, prepared schema encoding, prepared Whole encoding,
prepared schema-zstd encoding, object identity and public warm-selector
preparation. Contexts are reused for the three seeds within a stage/cell and
released before the next context. Setup, generation, verification and drop are
separately recorded; the shared conservative sender allowance stays below
512 MiB. Newline and record counts expose the schema whole-literal threshold.
All first-use observations remain. These independent calls overlap algorithmic
work and **must not be summed as components of a real transfer**. Selector-plan
verification consumes that plan in separately timed, untimed-for-encoding real
Endpoint transfers and stopped-store readback; its later wrapper drop is not
the original plan's complete destruction cost. No network/energy claim follows.

The completed first profile remains retained as `baseline-profile.jsonl`. Its
mistyped caller-supplied revision is corrected by the accompanying provenance
sidecar, not by rewriting the original observation. Compiled source hashes and
the executable binding identify the actual profile source.

## Common sessions and exact sequence

The three arms are `baseline-selector`, `candidate-selector` and `whole`.
Both selector arms retain **WarmSession**, fit the unchanged 512-record training
corpus once per session and construct one selector context. Whole retains one
equally configured prepared encoder. The baseline and candidate use the same
new harness; only explicitly pinned production-file overlays differ. All arms
pay the same actual Raw prior-object seed, use the same retained HttpClient and
32-request/500 ms origin keep-alive policy, and preserve the receiver's exact
digest/length checks, authorization and durable commit. Whole does not require
that prior-object seed; report its paid cost as an equal-state comparison.

Every object is exactly 1,048,576 or 16,777,216 bytes. Start with the unchanged
trial/size-keyed xorshift fixture. Ten successive targets are:

| Position | Target |
|---:|---|
| 1 | Clone the initial random base; XOR its midpoint 32 bytes by `0xa5`. |
| 2 | Independent pseudorandom replacement using the existing persistent escape domain and trial/size seed. |
| 3 | Clone the acknowledged replacement; XOR its midpoint 32 bytes by `0xa5`. |
| 4 | A zero-filled object. |
| 5 | Clone the zero base; XOR its midpoint 32 bytes by `0xa5`. |
| 6 | Held-out matching structured records. |
| 7 | Independently changed held-out matching records. |
| 8 | Held-out lexical and field-order variants. |
| 9 | Held-out schema drift. |
| 10 | Clone the acknowledged drift object; XOR its midpoint 32 bytes by `0xa5`. |

Positions 6–9 reuse the existing `record(false, index, trial*10+position, family)`
generator, with families `normal`, `normal`, `variants`, `drift`. Prefix each
record's existing quoted `note` value (or `message` for drift) with eight copies
of `held-out persistent telemetry value `, preserving its original raw value
bytes and lexical escapes. This increases value width without fitting new
templates and keeps the 16 MiB record count below 65,536. Concatenate complete
records until the requested size, then take only the remaining bytes of the
last record. A final partial record is original fixture data and requires exact
literal handling. Record exact target hashes, byte lengths and record counts;
no target is fitted. All eleven prior/target IDs must differ within a session.

Immediately before target 6, each selector **publishes the frozen 169-byte
artifact once and receives its actual validated ACK**. This is a distinct paid
`schema_seed` operation. It preserves cold-schema controls at positions 1–5
and supplies genuinely acknowledged schema readiness at positions 6–10.
Whole publishes no artifact. Never substitute a local readiness flag for that
operation. Any failed seed/send stops the sample; no automatic recovery/retry.

## Dose, capacity and timing

Use the existing shuffled six arm permutations and source-independent fixture
and scope binding, with ten local paired trials and six WAN paired trials per
size. Each receiver store has one trial, both sizes and all three arms:
six Raw base seeds, sixty targets and four artifact publications = **70 actual
transfers**. Local total: **700 transfers**, sixty sessions, forty fits. WAN
total: **420 transfers**, thirty-six sessions, twenty-four fits. The WAN result
is one six-pair pilot, not thirty-pair acceptance. All observations remain.

Retained prior/target payload is 561 MiB; add four 169-byte artifacts, 40 MiB
codec-cache allowance, 16 MiB transient store allowance and 70×4096-byte metadata.
That conservative bound remains below 768 MiB/store. Local stores are sequential
and removed only after all seventy original objects have been independently
regenerated and compared against the closed/reopened receiver. Retain failures.
Require 4 GiB free disk before each store. Keep only the acknowledged base and
one generated target in the sender, retaining the previous shared 512 MiB
conservative admission and external-buffer capacity accounting.

Reuse the existing disjoint operation timers and generation-inclusive outer
session timer. Time and record actual fitting/context setup, base seed, schema
publication, each target and fixture generation. Add sender-process CPU to
disjoint receiver-handler CPU once. Exchange wall/CPU is a nested interval,
not pure wire time. Count both HTTP bodies, including reply metrics; headers,
retransmissions and additional relay hops are outside that count. Retained
contexts are owned-state counters, not process RSS or energy measurements.
Buffer serialization and normalized-frame hashing until the outer timer stops.

## Fixed acceptance gates and result reporting

At both sizes, with ten local paired trial ratios:

- Replacement target 2 sender-CPU median candidate/baseline **≤0.95**.
- Per-trial sum of localized random targets 1 and 3 sender-CPU median ratio
  **≤1.05**. Both targets must retain the complete 600-byte selected exchange.
- Actual setup + paid schema publication + all ten target sender-CPU median
  ratio **≤1.05**. Base seed and complete paid lifecycle remain separately shown.
- Every baseline/candidate selected mode, canonical normalized request frame,
  bidirectional body cost and exact reopened byte string must match.

Do not require 600 bytes for compressible, structured or drift targets where
another exact candidate can win. Report every position/family, first use and
reuse, sender/receiver CPU, total durable elapsed, p95, paired wins/losses, setup,
publication and paid lifecycle against both baseline and Whole. A baseline gain
does not establish a win over Whole. Failed gates stop advancement and remain
results; no result-dependent rerun, discarded warmup or threshold adjustment.
No builds, tests, profiling or uploads overlap timing. Parent owns that lane.

CLI follows the existing persistent harness with explicit `unrelated-*` names.
`unrelated-paired PLAN BASELINE_EXE CANDIDATE_EXE URL RECEIVER_SHA NEW_JSONL`
uses both frozen executables; plans and verification use the candidate source.
`unrelated-summary` requires six ordered plan/raw/verification triples;
`unrelated-local-summary` requires ten. Source and verification environment names
are `SPIRALCORE_UNRELATED_SOURCE`, `SPIRALCORE_UNRELATED_OBSERVATIONS` and
`SPIRALCORE_UNRELATED_RECEIVER_SHA256`; the unprinted transport token retains
`SPIRALCORE_WAN_TOKEN`. Historical `persistent-*` behavior remains unchanged.
