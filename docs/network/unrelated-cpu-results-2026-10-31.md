# SC-OPT-019: unrelated-object sender CPU

The production change removes payload allocation for a provably losing
whole-literal sparse recipe, accelerates exact schema record scanning/counting,
and rejects impossible JSON-object matches before UTF-8 parsing. Sparse scans
and useful COPY candidates, schema formatting,
compression settings, byte-cost selection, authentication, decoding and durable
commit remain unchanged. The direct `memchr = 2.8.3` dependency uses the package
already present in the lockfile; no package version was upgraded.

The production baseline is `56e15db5c77fe2ecdc03476d8e09664cd4213fd1`.
The [measurement contract](evidence/2026-10-31-unrelated-cpu/measurement-contract.md)
fixes the mixed workload, byte/frame parity, CPU gates and bounded WAN dose.
The comparison uses the same new harness with the four explicit production
files overlaid from that baseline. Both selector arms use persistent WarmSession;
Whole uses one equally configured prepared zstd encoder.

## Independent stage diagnostics

Each diagnostic retained 72 observations: three fixed seeds, two sizes,
localized and unrelated inputs, and six independently executed public calls.
Every codec output passed post-timer exact reconstruction; selector plans
passed real local endpoint transfer and closed/reopened byte comparison.
These are separate unpaired diagnostic runs, not randomized transfer trials.
Do not add the stage clocks: schema-zstd and selector calls repeat other work.

Unrelated-input median process CPU, milliseconds:

| Size | Stage | Baseline | Delta-only | Combined |
|---|---|---:|---:|---:|
| 1 MiB | Sparse encoding | 0.258 | 0.172 | 0.169 |
| 16 MiB | Sparse encoding | 5.268 | 2.656 | 2.633 |
| 1 MiB | Schema intermediate | 1.005 | — | 0.355 |
| 16 MiB | Schema intermediate | 15.994 | — | 5.586 |
| 1 MiB | Complete public warm preparation | 5.687 | 11.132 | 4.589 |
| 16 MiB | Complete public warm preparation | 124.473 | 141.411 | 96.103 |

The delta-only whole-selector diagnostic regressed despite its lower isolated
sparse cost. All observations, including first-use and negative results, remain
in [baseline](evidence/2026-10-31-unrelated-cpu/baseline-profile.jsonl),
[delta-only](evidence/2026-10-31-unrelated-cpu/delta-profile.jsonl) and
[combined](evidence/2026-10-31-unrelated-cpu/combined-profile.jsonl) raw files.
The baseline diagnostic's caller-supplied revision was mistyped. Its original
registration is unchanged; the [correction](evidence/2026-10-31-unrelated-cpu/profile-provenance-correction.json)
records the verified source revision, supported by all nine embedded source
hashes. No timing rows were edited or rerun for that metadata error.

## Qualification

The allocation regression first observed four payload-sized allocations for a
131,073-byte uniformly unrelated pair. The optimized encoder makes zero such
allocations. Existing immutable oracles compare complete sparse recipes,
SCVAL001 streams and compressed-schema bytes. Added checks cover borrowed LF
slice offsets, empty/adjacent/trailing lines, binary data, scan boundaries and
the unchanged record-count cutoff.

Focused LLVM coverage on the changed production files is 97.33% of delta lines
and 96.91% of schema lines, with 100% of functions in each. Region coverage is
96.25% and 96.96%. These are focused file metrics; branch/MC/DC and whole-project
coverage are not claimed. The first schema qualification command named a
nonexistent `schema_zstd` test target and failed before compilation; the
corrected command used `schema_reuse`. Both receipts are retained.

## Phase 1 mixed local comparison

Frozen source `a128b3a535964dba569e9bb3c124b0234f6c3a80` completed the
registered 700 transfers: 600 targets, 60 base seeds and 40 actual schema
publications. Every object reopened exactly, every baseline/candidate selected
mode and normalized frame hash agreed, and both localized-random controls kept
600 bidirectional HTTP-body bytes. All ten successful stores were removed only
after verification. Raw records and the complete summary are in
[local](evidence/2026-10-31-unrelated-cpu/local/summary.json).

| Paired sender CPU, candidate/baseline median ratio | 1 MiB | 16 MiB | Gate |
|---|---:|---:|---|
| Unrelated replacement | 0.887294 | **0.959994** | At most 0.95 |
| Sum of the two localized-random updates | 0.938544 | 1.017215 | At most 1.05 |
| Setup, schema publication and ten updates | 0.923138 | 0.893384 | At most 1.05 |

**Phase 1 failed advancement:** the large replacement improved by approximately
4.0% on the registered paired metric, below the required 5%. This is a retained
negative gate, not a successful WAN qualification. Ratios are medians of paired
trial ratios; they are not ratios of separately aggregated medians.

The [phase-2 contract](evidence/2026-10-31-unrelated-cpu/phase2-contract.md)
registers two additional exact schema changes and a separate campaign under the
same thresholds. Phase 1 is not discarded, relabelled or pooled with phase 2.
No energy saving is claimed.

## Phase 2 mixed local comparison

The additional exact prefix preflight and chunked count were tested in a
separate 700-operation campaign at source
`f662f5a54886365de20be17e6ef8219d94157362`. All 700 objects again reopened
exactly, baseline/candidate normalized frame hashes and body costs matched,
and the localized-random exchanges stayed at 600 bytes. All ten stores were
removed after verification. [Complete phase-2 records and summary](evidence/2026-10-31-unrelated-cpu/local-phase2/summary.json).

| Paired sender CPU, candidate/baseline median ratio | 1 MiB | 16 MiB | Gate |
|---|---:|---:|---|
| Unrelated replacement | 0.836511 | **0.960263** | At most 0.95 |
| Sum of the two localized-random updates | 1.034534 | 0.999395 | At most 1.05 |
| Setup, schema publication and ten updates | 0.900670 | 0.916298 | At most 1.05 |

**Phase 2 also failed advancement.** Its small replacement paired CPU gain is
16.35%; the large gain is 3.97%, below the unchanged 5% gate. Both warm and
mixed CPU controls pass. This is not a reason to discard phase 1 or repeat
phase 2 until it passes. Local large-replacement median completion also worsened
from 152.914 to 175.134 ms; sender CPU improvement does not establish a latency
win. Prepared Whole remains cheaper for unrelated data.

## Separately registered WAN diagnostic

The user's requested before/after WAN comparison remains useful for measuring
this limitation. The [prospective diagnostic amendment](evidence/2026-10-31-unrelated-cpu/wan-diagnostic-contract.md)
therefore registers one six-pair, 420-operation diagnostic of the unqualified
phase-2 build. This explicitly changes the earlier stop-before-WAN plan without
lowering the local gate. Registration states `local_advancement:false` and
`performance_acceptance:false`. Results cannot retroactively qualify either
local campaign and will not be pooled with them. WAN results are pending.
