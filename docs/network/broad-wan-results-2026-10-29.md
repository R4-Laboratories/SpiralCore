# Broad 16 MiB WAN results — campaign in progress

Measured source: `9dce4c57d1f41cef7365b297cd690849afce34e7`.
Run: `d6aaa0f15da9ccbc62c005a1802532b3e85ad6d901886eea6fbd0076b2d312e4`.
Two of twelve registered cases are complete and independently reviewed. Remaining
cases must not be inferred from this result. The complete registration is in the
[workload plan](../plans/2026-10-29-broader-wan-and-scaling.md).

This v4 comparison uses the same mandatory verified-byte publication policy for
raw, ordinary zstd, zstd prefix delta and sparse COPY/LITERAL. Optional raw/zstd
DAG warming is disabled equally. Earlier v3 CPU results included that additional
raw/zstd work and must not be substituted for this fairer comparison.

## Completed case: 32-byte localized edit

Thirty paired trials use campaign-derived arm-order permutations: all 24 orders
occur before repetition. Inputs are 16,777,216-byte pseudorandom objects with a
32-byte midpoint overwrite. Each arm pays for its own authorized raw seed; five
fresh-cache shards contain six trials each. GitHub Linux senders transfer through
a temporary Cloudflare tunnel to the native Apple M1 receiver.

| Warm update | Median request + reply HTTP-body bytes | Median elapsed, including computation | Median sender + receiver-handler CPU |
|---|---:|---:|---:|
| Raw full send | 16,814,026 | 1,594.631 ms | 100.872 ms |
| Ordinary zstd | 16,814,026 | 1,599.863 ms | 95.911 ms |
| Zstd prefix delta | 26,245 | 240.178 ms | 101.087 ms |
| Sparse COPY/LITERAL | 600 | 237.108 ms | 79.839 ms |

Sparse uses 99.99643155% fewer HTTP-body bytes than raw, with a 6.725348× ratio
of median elapsed times. Its median combined CPU is approximately 20.85% lower.
The separate paired geometric means are 7.116085× elapsed and 1.277492× CPU.
The configured zstd prefix control has a similar median completion time; sparse
uses substantially fewer bytes and less CPU on this particular fixture. These
are comparisons against the measured configurations, not every possible raw or
delta implementation.

Including the actual seed and one update, raw uses 33,628,052 body bytes and a
3,153.206 ms median elapsed time. Sparse uses 16,814,626 bytes and 1,873.202 ms.
The warm-update speedup therefore does not describe a first transfer's total cost.
Long successor chains remain a separate experiment.

All 240 seed/update operations reopened byte-for-byte exactly, totaling
4,026,531,840 reconstructed bytes. Independent fixture identities, canonical
plans, source/binary custody and statistical vectors agree with the author analysis.
All five captures are complete and agree with independent PCAP packet-total replay.
Across all thirty pairs, median runner-to-relay IP totals are 17,489,112 bytes for
raw and 5,925 bytes for sparse. These include both captured directions on that leg;
they exclude the relay-to-receiver leg and physical-link overhead.

The [complete case packet](evidence/2026-10-29-broad-wan/cases/localized_32/README.md)
contains raw rows, exact-reopen receipts, source/executable identities, capture
accounting, independent analysis and custody. Raw ZIPs and PCAPs remain local and
hash-bound. Hosted measurement artifacts were removed only after verified local
preservation and a Git commit; the [cleanup receipt](evidence/2026-10-29-broad-wan/cleanup/localized_32.json)
identifies the five exact artifacts. The qualified sender artifact remains available
while later cases run.

## Completed case: 32 scattered 32-byte edits

Thirty paired trials spread 32 separate 32-byte overwrites across each 16 MiB
pseudorandom object. Source, policy, four arms, private seed charging and exact
reopening match the localized case.

| Warm update | Median request + reply HTTP-body bytes | Median elapsed, including computation | Median sender + receiver-handler CPU |
|---|---:|---:|---:|
| Raw full send | 16,814,026 | 1,679.259 ms | 100.112 ms |
| Ordinary zstd | 16,814,026 | 1,726.421 ms | 100.562 ms |
| Zstd prefix delta | 27,304 | 276.078 ms | 104.242 ms |
| Sparse COPY/LITERAL | 2,137 | 248.104 ms | 80.241 ms |

Sparse uses 99.98729037% fewer HTTP-body bytes than raw, with a 6.768379× ratio
of median elapsed times and approximately 19.85% lower median combined CPU.
The zstd prefix control also reuses the shared base successfully. Both preserve
exact bytes; these observations establish the measured configurations' behavior
for this scattered-edit distribution, not arbitrary rearrangements.

Including seed and one update, raw uses 33,628,052 body bytes and a 3,402.020 ms
median elapsed time. Sparse uses 16,816,163 bytes and 2,029.033 ms; zstd prefix
uses 16,841,330 bytes and 2,034.308 ms. All 240 operations reopened exactly,
totaling 4,026,531,840 reconstructed bytes. All five captures were complete and
independent review agreed without modifying the checker or repeating transfers.

The [complete scattered case packet](evidence/2026-10-29-broad-wan/cases/scattered_32x32/README.md)
contains the raw receipts and reviewed statistics. Its five hosted measurement
artifacts were deleted only after committed, hash-verified local preservation;
[the exact cleanup receipt](evidence/2026-10-29-broad-wan/cleanup/scattered_32x32.json)
records those deletions. The raw ZIPs and PCAPs remain locally retained.

## Qualification and measurement boundaries

The source-qualified producer passed 167 tests including both large HTTP tests,
ordinary and instrumented Linux capture qualification, and independent artifact
custody checks. Optimized-profile line coverage is 7,673/8,476 (90.526%); branch
coverage was not collected. Production release hashes were unchanged by coverage.
The [producer packet](evidence/2026-10-29-broad-wan/qualification/linux-producer-2/README.md)
retains the evidence. Earlier failed capture setups and the debug-profile coverage
timeout remain separate records and contributed no WAN observations.

Before measurement, the analysis gate was corrected to use the originally specified
ratio of median completion times; the paired geometric mean is descriptive. After
the first case, independent review exposed a checker-only task-projection mismatch.
Its failed output, original executable/source identity, regression and successful
v2 review are preserved. No transfer was rerun or omitted to repair analysis.

Elapsed starts after fixture generation, with inputs in memory, and ends after
verified durable publication acknowledgment and timed retained-state teardown.
It already includes encoding and reconstruction computation;
CPU must not be added to elapsed again. Combined CPU sums sender process and receiver
handler measurements before statistics. Receiver transport work outside that timer,
tunnel/relay work, fixture generation and post-run reopening are separate. Pure
network-only time, peak process memory and energy are not established by these rows.

This is an unshaped, relayed Internet experiment with synthetic data. Direct/shaped
WAN acceptance, other content distributions, sustained version chains, recovery,
larger clients and MVM boundaries remain distinct work. Final MVM integration must
include the canonical vsock mailbox bridge for a small MVM environment and its
orchestrator; WASM remains deferred.
