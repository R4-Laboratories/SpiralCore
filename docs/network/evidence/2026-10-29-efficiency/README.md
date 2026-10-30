# Efficiency evidence, October 29, 2026

Read [the report](../../efficiency-results-2026-10-29.md) for interpretations and scope.

- `warm/`, `prefix/`: original remote plan, rows, summary, capture accounting, machine/source/binary
  identities and raw capture counters; local reopened-cache verification, process snapshots and
  tunnel counters. The original remote files are copied byte-for-byte. Private capability files
  are not included and were deleted after the runs.
- `independent-*-analysis.json`, `independent-efficiency-wan-summary.json`: separate per-trial
  recalculations, including summed endpoint CPU before quantiles, source/fixture checks, seed costs
  and a complete independent prefix-PCAP accounting pass. These are analysis artifacts, not added
  production functionality.
- `builder/`: unchanged Rust probe source, baseline/build identities and raw balanced process
  observations. `builder-paired.json` records the initial direct-builder result, including its
  repetitive-data regression. Initial pair 0 is preserved but excluded because it overlapped review
  tests. `builder-final-paired.json` includes all ten final pairs after the bounded memo repair.
- `rsync/`: standalone diagnostic Rust source, final build identities, original per-operation
  receipts, pilot qualification and summary. Each case's seed/target fixture identities match its
  own WAN plan. The original build manifest retains a reference to the task's unit-test transcript;
  its structured `test_result` records nine passes. Diagnostic console logs were removed during
  cleanup. Raw stdout/stderr embedded in measurement receipts is retained as evidence for the
  protocol-byte counters.
- `coverage.json`: LLVM coverage summary for transfer production files; test files and the imported
  crate are excluded. Branch instrumentation is absent.
- `validation.json`: code/test/build/campaign scope and cleanup receipt.
- `SHA256SUMS`: hashes of every retained evidence file except itself, using paths relative to this
  directory. This does not rewrite historical export manifests.

Large header PCAPs remain local, outside Git, with these identities:

| Capture | Local path | SHA-256 |
|---|---|---|
| Warm | `/Users/casey.allard/spiralcore/local/efficiency-20261029/wan-warm/remote/headers.pcap` | See `warm/capture-sha256.txt` |
| Prefix | `/Users/casey.allard/spiralcore/local/efficiency-20261029/wan-prefix/remote/headers.pcap` | `2f54716a9b1807aa47f8b2eff75eed739d7248c3735c44b22531fee8c4359d9c` |

Hosted GitHub artifacts have one-day retention. Local receipts and original PCAPs are preserved.
Warm capture attribution is incomplete. Prefix capture passed current completeness checks for the
runner-to-relay leg only; neither measures Cloudflare internals or the relay-to-Mac leg completely.
Successful file/directory sync means the specified OS durability boundary, not a power-loss test.
