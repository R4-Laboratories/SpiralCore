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
local campaign and will not be pooled with them.

The [real WAN run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41177063558)
completed once, without retries, at launch source
`c83f7656e2a9f92b7d6bc8557af987391d147516`. Its runtime is unchanged from
`f662f5a`; only diagnostic declarations changed after the local freeze. A Linux
GitHub-hosted sender reached the local macOS receiver through temporary HTTPS
Cloudflare relays. The timed campaign ran 06:36:24–06:39:58 UTC on October 31.
Six paired trials at each size covered all six arm orders and 36 persistent
sessions: 360 targets, 36 initial bases and 24 actual schema publications.
All **420 operations reopened byte-for-byte exactly**, with the original
lengths. All baseline/candidate selected modes, body costs and normalized frame
hashes matched. The [complete raw evidence and summary](evidence/2026-10-31-unrelated-cpu/README.md)
retain every observation and negative result.

### Unrelated-object replacements

Medians, milliseconds except bytes; six observations per row. CPU sum is the
per-operation sender process plus receiver handler CPU, summarized separately
from its components; independently computed medians need not add exactly.
Exchange is the nested HTTP exchange, not pure serialization/wire time.

| Size | Sender | Bidirectional body bytes | Sender CPU | Receiver CPU | CPU sum | HTTP exchange | Total through durable ACK |
|---|---|---:|---:|---:|---:|---:|---:|
| 1 MiB | Previous selector | 1,051,338 | 8.625 | 6.055 | 14.592 | 324.658 | 331.477 |
| 1 MiB | Optimized selector | 1,051,338 | 7.430 | 6.310 | 13.680 | 359.274 | 365.196 |
| 1 MiB | Prepared Whole zstd/Raw | 1,051,338 | 3.207 | 5.888 | 9.179 | 528.169 | 530.036 |
| 16 MiB | Previous selector | 16,814,026 | 136.069 | 46.393 | 171.555 | 2,005.344 | 2,121.758 |
| 16 MiB | Optimized selector | 16,814,026 | 114.758 | 57.316 | 164.809 | 1,758.593 | 1,842.863 |
| 16 MiB | Prepared Whole zstd/Raw | 16,814,026 | 48.715 | 49.591 | 99.980 | 1,911.149 | 1,945.072 |

Against the previous selector, paired sender CPU ratios are **0.857884 / 0.845370**
at 1/16 MiB: 14.21% / 15.46% reductions, 6/6 wins at each size. Paired completion
ratios are **1.013810 / 0.912606**, with 3/6 and 4/6 wins: small completion
regresses 1.38% despite less sender CPU. Large candidate p95 completion is
3,328.989 ms versus previous 2,743.422 and Whole 2,136.254 ms; with six trials,
this nearest-rank p95 is the maximum and is not a stable tail estimate.

Whole remains materially cheaper computationally: candidate/Whole paired sender
CPU ratios are 2.347273 / 2.265449. All three send identical body sizes here.
Candidate/Whole completion ratios happen to favor the candidate in this run
(0.605940 / 0.941156, 6/6 and 4/6 wins), but that cannot be attributed to byte
savings or superior encoding: there are none on these replacements. Relay and
link variation dominate this small sample. The local large-object latency
regression remains a counterexample to a general speed claim.

### Warm updates and complete mixed sessions

The two localized-random positions per trial retain **600 body bytes** at both
sizes. These rows aggregate 12 updates per arm/size and do not count the earlier
base seed, which is charged in the subsequent lifecycle table.

| Size | Sender | Body bytes | CPU sum, ms | HTTP exchange, ms | Total, ms |
|---|---|---:|---:|---:|---:|
| 1 MiB | Previous selector | 600 | 7.846 | 130.000 | 131.566 |
| 1 MiB | Optimized selector | 600 | 7.866 | 128.416 | 129.551 |
| 1 MiB | Prepared Whole zstd/Raw | 1,051,338 | 8.973 | 372.560 | 374.969 |
| 16 MiB | Previous selector | 600 | 49.575 | 166.687 | 180.141 |
| 16 MiB | Optimized selector | 600 | 57.295 | 169.503 | 181.259 |
| 16 MiB | Prepared Whole zstd/Raw | 16,814,026 | 96.278 | 1,735.817 | 1,772.465 |

The warm byte reductions versus Whole are 99.9429% and 99.9964%. This round
preserves that byte advantage rather than creating a new warm CPU win: paired
CPU ratios for the sum of the two warm updates are 1.072623 / 1.006415 against
the previous sender. The 1 MiB WAN warm sender CPU therefore regresses 7.26%,
even though both separately registered local warm controls passed.

Paid mixed lifecycle medians include fitting/context setup, the real initial
base, schema publication and all ten distinct targets. Ordinary Whole is given
the same equal-state base seed even though it does not need it; these are
matched-state experimental sessions, not a claim about every deployment's cost.

| Size | Sender | Body bytes | Sender CPU, ms | CPU sum, ms | Total, ms |
|---|---|---:|---:|---:|---:|
| 1 MiB | Previous selector | 2,326,493 | 73.439 | 143.326 | 2,334.425 |
| 1 MiB | Optimized selector | 2,326,493 | 68.338 | 144.544 | 2,263.804 |
| 1 MiB | Prepared Whole zstd/Raw | 4,477,566.5 | 39.665 | 100.823 | 2,729.594 |
| 16 MiB | Previous selector | 37,193,928 | 1,052.879 | 1,536.517 | 7,475.079 |
| 16 MiB | Optimized selector | 37,193,928 | 959.316 | 1,464.881 | 7,217.048 |
| 16 MiB | Prepared Whole zstd/Raw | 71,658,713.5 | 559.814 | 963.260 | 10,636.151 |

The candidate retains about **48.0% / 48.1% fewer body bytes** than Whole over
these complete mixed sequences (ratios of the displayed medians). Paired paid
completion ratios versus Whole are 0.846201 / 0.709372, with 6/6 wins each;
paired CPU-sum ratios are 1.400336 / 1.562563, with zero CPU wins. Versus the
previous selector, paid completion ratios are 0.994489 / 0.990112, only 3/6 wins
each. Setup plus schema publication plus ten-target sender CPU improves
7.94% / 8.38% by the paired median. Thus the mixed session still trades more
computation for less traffic and lower elapsed time against Whole in this run.

### Scope, qualification and disposition

Both selector arms capture winning frame bytes inside their measured operation
for the exact-frame parity check; Whole does not. Hashing and JSON reporting of
those captured bytes happen after the outer timer. This matched selector
instrumentation adds allocation/copy cost when comparing either selector with
Whole; it is not a production-only CPU measurement. The CPU subtotal excludes
receiver HTTP/logging overhead and relay CPU. Exchange includes network waiting,
HTTP work and receiver computation; do not add it to total time or label it pure
send time. Body counts exclude headers, TLS, packet retransmission and additional
relay hops. Fixture generation is reported in separate outer-session clocks.

The Linux sender uses the same release/default feature build for both selectors;
the frozen macOS receiver uses hardware SHA-256. This differs from the local
Apple-hardware-SHA sender and helps explain why cross-platform CPU gains differ.
No 30-pair qualification, direct or shaped WAN, energy, FIPS or MVM result is
claimed. The two earlier local failures and this six-pair diagnostic remain
separate. The raw harness's inherited `six-pair-wan-pilot` dose label is retained;
the prospective registration controls its **unqualified diagnostic** purpose.

Hosted qualification passed **431 tests, zero failures**, with two preexisting
ignored tests across 55 targets; both frozen release builds and Clippy passed.
Phase-2 focused changed-file line coverage is 97.00% for schema and 97.33% for
delta, with 100% of their functions covered; branch coverage was not collected.
No source or binary rebuild overlapped the timed campaign. After successful
stopped-store verification, all six owned caches were removed, reclaiming
3,448,488 KiB (3.29 GiB allocated), and all six receivers/tunnels, nine temporary
GitHub variables, the capability secret and its local file were removed/stopped.
Compact raw measurements, failure receipts and checksums remain reviewable.

**Disposition: implemented and measured, not performance-accepted.** The exact
allocation/scan changes are retained as a bounded candidate; no codec policy or
format changed. Both local large-replacement CPU gates remain failed, and the
WAN small-warm CPU regression remains explicit. The next useful optimization
is avoiding more futile candidate preparation through exact lower bounds or
reusable state, while keeping minimum-byte selection and useful warm/schema
candidates intact. No probabilistic entropy guess or disabled fallback is
justified by these measurements.
