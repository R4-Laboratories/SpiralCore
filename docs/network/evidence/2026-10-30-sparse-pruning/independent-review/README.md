# Independent SC-OPT-007 analysis

`verify.jq` consumes the normal, uninstrumented 140-sample JSONL campaign. Supply
the frozen source and executable identities from the separately retained custody
record; the checker does not trust the producer's self-declared pins alone:

```sh
jq -s --arg expected_source "$frozen_source" \
  --arg expected_executable "$frozen_executable_sha256" \
  -f docs/network/evidence/2026-10-30-sparse-pruning/independent-review/verify.jq \
  path/to/registered-timing.jsonl
```

The checker reconstructs the exact seven-case, ten-pair, alternating AB/BA
schedule and requires all 140 separate process receipts. It checks paired input
identities and lengths, full recipe receipts, source/executable pins, timer
arithmetic, minimum CPU resolution and terminal counts. It computes old/new
paired CPU geometric means, candidate wins and per-arm CPU, wall and drop-time
distributions directly from observations. Primary gates require at least 1.10
geometric mean old/new CPU and seven candidate wins; controls allow at most 5%
geometric mean candidate CPU regression. Invalid records cause an error. Valid
unfavorable measurements remain valid records with acceptance false.

`synthetic-fixture.jq` generates explicitly synthetic checker qualification
records with dummy identities. They are **not measurements**. `qualify.sh`
checks known geometric means, eight corruptions (missing row, wrong pair,
source, executable, output, base, arithmetic and completion), and an unfavorable
but valid control result. All checks passed with jq1.7.1-apple, exit0, before
examining the registered measurements. Each corruption exited5 at its intended
gate. Exact mutations and errors are retained in `checker-tests.jsonl`.

This analysis verifies receipt consistency and statistics. It does not regenerate
every fixture or rerun every decoded-byte comparison independently. The frozen
Rust harness performs those comparisons, and its source identity remains part
of the evidence. Normal timing excludes allocation instrumentation. Allocation
diagnostics and their acceptance are separate; no total RSS, WAN or energy
claim follows from this checker.

`verify-allocations.jq` joins all 140 allocation rows to the normal campaign by
case, pair and arm, requiring identical fixture and recipe receipts. It requires
externally supplied diagnostic executable, candidate source and legacy source
hashes. At-return live bytes must equal the returned recipe's actual Vec
capacities. Registry failures, missing frees, live buffers after drop and
counter inconsistencies fail verification. The registered stronger memory gate
requires **every one of ten pairs in each primary cell** to reduce peak tracked
capacity by at least 50%. For each primary candidate, the observed header count
must equal the winning recipe's frame count plus its final spare header, or zero
for Raw fallback. This is the additional check that rejected candidates did not
materialize frame buffers.

`qualify-allocations.sh` passed eight malformed-record mutations and two valid
negative results: a single pair below the memory threshold, and a rejected
candidate that nevertheless allocated an extra header. Those negative results
remain valid data with failed acceptance. `synthetic-allocations.jsonl` and its
result files are synthetic checker tests only. The diagnostic policy uses actual
allocate-copy-free growth and includes old/new overlap; it does not reproduce
the production System allocator's potential in-place reallocations or measure
process RSS.

The frozen actual campaigns both passed record verification, with all 140
diagnostic rows matching their normal counterparts. Their **registered overall
optimization acceptance failed CPU gates**: middle insertion reached a 1.092043
legacy/candidate geometric ratio, below 1.10; repeat and periodic controls had
candidate/legacy ratios 1.068161 and 1.064015, exceeding 1.05. These are retained
negative results, not malformed observations. All 30 primary memory pairs passed.
See `timing-result.json`, `allocation-result.json`, `timing-summary.tsv` and
`actual-custody.json` for complete values and source/executable/input identities.
Each campaign contains 120 decoded recipes (2,013,265,920 bytes checked) and 20
Raw fallbacks. Raw fallbacks retain the original bytes; they are not counted as
decoded or transferred objects. No observation was discarded or replaced.
