# Independent verification of SC-OPT-006

Reviewer: `/root/selector_tests`, independent of the production selector and
measurement-harness authors. This directory retains a jq implementation of the
receipt arithmetic, its output, mutation qualification and source/binary/cache
custody checks. The reviewer authored the adaptive integration tests separately.

Input is the unchanged `../measurements.jsonl`, SHA-256
`f2e3f74af52b89da1b8197a588f0d2c5ebc20595df632a43098000b6e641387d`.
It contains 3,043 JSONL records. Source is
`850272f01d580d1cf7564d25fb628fd50f58d956`; preregistration was committed in
`791ec790eca25bd3f79cc21bdb38cfa27013b32a` before measured collection.
The separate three-trial smoke run is excluded from acceptance statistics.

Run these commands from the repository root using `jq-1.7.1-apple` and Bash:

```sh
review=docs/network/evidence/2026-10-29-codec-selection/independent-review
jq -s -f "$review/verify.jq" \
  docs/network/evidence/2026-10-29-codec-selection/measurements.jsonl \
  > "$review/result.json" 2> "$review/verification.stderr"
bash "$review/qualify.sh"
bash "$review/check-custody.sh"
```

All three commands exited 0. Their exit receipts are retained. The positive
verification produced no stderr. Nine mutated inputs were rejected with exit 5
at the intended gates; `negative-tests.jsonl` preserves every mutation and error.
An initial no-input syntax invocation was rejected by the row-cardinality guard,
also exit 5; it did not modify measured observations. No measured trial was
discarded, regenerated or replaced during this review.

The checker independently verifies the exact operation schedule, unique paired
identities and scopes, matching object and base identities, 600 Raw seed receipts,
1,200 target receipts, and 16,043,212,800 total compared bytes. All 240 adaptive
target costs equal the smallest measured configured fixed-arm cost and their own
predictions. It verifies body-envelope arithmetic, candidate costs, nested timer
ordering, cleanup coverage and terminal counts. It reconstructs all 40 complete
summary objects: seven metrics, count/min/midpoint median/nearest-rank p95/max,
paired ratios, mode counts, and separate seed and seed-plus-transfer statistics.
Recomputed objects match the author's recorded objects exactly.

`check-custody.sh` checks all 60 source-manifest entries, the absence of runtime
changes since the pinned revision, and the frozen executable hash. Each of the
two owned cache roots contains only its `.selector-owner`; its value matches
that run's start identifier. No cache from another task is inspected or removed.
`command-events.jsonl` retains selected native command event metadata and public
commands, not the full private chat histories. Exact output receipts remain in
the parent evidence directory.

This is receipt/statistics verification, not an independent rerun of all byte
reconstructions or regeneration of every fixture. The pinned Rust harness made
the actual reopen-and-compare checks before deleting successful synthetic caches.
The numeric check therefore verifies those receipts, identities and arithmetic;
it does not recreate deleted cache contents. Body counts are local serialized
HTTP-body equivalents. There is no new WAN, network-only send-time, energy,
peak-memory, MVM or FIPS result here. CPU measurements are whole-process serial
sender-plus-receiver values; nested exchange and receiver measurements must not
be added to the total.

The results retain unfavorable measurements. At 16 MiB, localized changes use
600 body-equivalent bytes, but adaptive CPU median is 86.4015 ms against Raw's
35.2865 ms and local wall median is 99.909 ms against 48.9755 ms. Unrelated
structured data similarly uses more CPU. Cold structured data improves bytes
and local CPU versus Raw. Whole-frame prefix compression's result on unrelated
structured content does not prove useful base reuse; a no-base whole-file zstd
comparator remains unmeasured.

`SHA256SUMS` binds this directory's retained artifacts. The final pipeline verdict
is in `../audit.json`; final post-audit code/security review and publication
verification are separate subsequent gates.
