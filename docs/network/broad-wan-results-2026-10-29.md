# Broad 16 MiB WAN results — campaign in progress

Measured source: `9dce4c57d1f41cef7365b297cd690849afce34e7`.
Run: `d6aaa0f15da9ccbc62c005a1802532b3e85ad6d901886eea6fbd0076b2d312e4`.
Seven of twelve registered cases are complete and independently reviewed. Remaining
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

## Completed case: 4 KiB prefix insertion

Each base contains 16,773,120 bytes; insertion produces an exact 16,777,216-byte
target. This changes alignment while preserving a large unchanged suffix. All
thirty pairs use the same four methods and paid-seed policy.

| Warm update | Median request + reply HTTP-body bytes | Median elapsed, including computation | Median sender + receiver-handler CPU |
|---|---:|---:|---:|
| Raw full send | 16,814,026 | 1,686.289 ms | 89.878 ms |
| Ordinary zstd | 16,809,951 | 1,682.304 ms | 96.987 ms |
| Zstd prefix delta | 26,221.5 | 231.881 ms | 88.078 ms |
| Sparse COPY/LITERAL | 4,651 | 226.397 ms | 79.081 ms |

Sparse uses 99.97233857% fewer HTTP-body bytes than raw, with a 7.448386× ratio
of median elapsed times and approximately 12.01% lower median combined CPU.
The fractional zstd-prefix byte median is the midpoint of the two central trials;
individual transfers have integer byte counts. Zstd prefix also achieves substantial
reuse with similar elapsed time on this fixture.

Including the seed and one update, raw uses a median 33,623,823 body bytes and
3,422.530 ms; sparse uses 16,814,448 bytes and 1,950.600 ms. All 240 operations
reopened exactly, totaling 4,026,040,320 compared bytes. Independent review agreed
with all raw receipts and author statistics, and all five captures were complete.
The [complete prefix case packet](evidence/2026-10-29-broad-wan/cases/prefix_4096/README.md)
preserves the thirty pairs and custody. This does not establish arbitrary shifted
block matching: the current encoder can directly reuse this unchanged suffix.

## Completed transfers: 4 KiB middle insertion

Bases contain 16,773,120 bytes; inserting 4 KiB in the middle produces exact
16,777,216-byte targets. All thirty pairs and 240 seed/update operations are
retained and reopened exactly, totaling 4,026,040,320 compared bytes.

| Warm update | Median request + reply HTTP-body bytes | Median elapsed, including computation | Median sender + receiver-handler CPU |
|---|---:|---:|---:|
| Raw full send | 16,814,026 | 1,600.969 ms | 93.113 ms |
| Ordinary zstd | 16,810,128 | 1,662.860 ms | 99.168 ms |
| Zstd prefix delta | 26,222 | 205.980 ms | 101.474 ms |
| Sparse COPY/LITERAL | 4,664 | 199.853 ms | 87.463 ms |

Sparse uses 99.97226125% fewer HTTP-body bytes than raw and has an 8.010733×
ratio of median elapsed times, with approximately 6.07% lower median combined
CPU. Zstd prefix also reduces elapsed time substantially, but uses more median
CPU than raw in this case. Including paid seed and one update, raw uses median
33,623,823 body bytes and 3,457.029 ms; sparse uses 16,814,461 bytes and
1,875.199 ms; zstd prefix uses 16,836,019 bytes and 1,861.559 ms.

Independent review agrees with all raw receipts and author statistics. Packet
attribution is incomplete for shard 1: one reused TCP tuple is quarantined by
the parser, leaving two measured rows without assigned packets (trial 6 raw
transfer and trial 11 sparse seed). The 24,928 quarantined packets leave assigned
bytes below the application-body lower bound. All 447,603 saved, captured and
received packet counts match, with zero reported drops; this is attribution
ambiguity, not evidenced capture loss. Shards 0, 2, 3 and 4 meet the completeness checks.
There is no complete thirty-pair IP-byte result and no silently substituted
four-shard ratio. The original failure remains in the
[complete middle-insertion packet](evidence/2026-10-29-broad-wan/cases/middle_4096/README.md).
This capture limitation does not remove any recorded application transfer or
change its exact-recovery, body-byte, elapsed or CPU observation. The separately
reviewed [tuple-reuse diagnosis](evidence/2026-10-29-broad-wan/qualification/middle-tuple-reuse/README.md)
preserves the observed handshakes and parser condition without replacing the
original incomplete accounting result.

## Completed case: 4 KiB deletion

Each base contains 16,777,216 bytes; deletion produces a 16,773,120-byte target.
All thirty pairs preserve that shorter length exactly. The four methods pay for
their own seed under the same publication policy.

| Warm update | Median request + reply HTTP-body bytes | Median elapsed, including computation | Median sender + receiver-handler CPU |
|---|---:|---:|---:|
| Raw full send | 16,809,797 | 1,859.642 ms | 92.330 ms |
| Ordinary zstd | 16,809,797 | 1,770.188 ms | 89.979 ms |
| Zstd prefix delta | 26,185 | 255.436 ms | 93.433 ms |
| Sparse COPY/LITERAL | 563 | 237.681 ms | 77.855 ms |

Sparse uses 99.99665076% fewer HTTP-body bytes than raw, with a 7.824109× ratio
of median elapsed times and approximately 15.68% lower median combined CPU.
Including seed and one update, raw uses median 33,623,823 body bytes and
3,502.654 ms; sparse uses 16,814,589 bytes and 2,127.901 ms. Zstd prefix uses
16,840,211 bytes and 2,045.040 ms for that lifecycle, so its median paid-seed
elapsed time is lower than sparse despite the sparse warm update being faster.

All 240 operations reopened exactly, totaling 4,026,040,320 compared bytes.
Independent analysis agrees with the raw receipts and author statistics; all five
captures meet the completeness criteria. The
[complete deletion packet](evidence/2026-10-29-broad-wan/cases/delete_4096/README.md)
retains all observations and their custody.

## Completed case: 4 KiB append

Appending 4 KiB to each 16,773,120-byte base produces an exact 16,777,216-byte
target. Thirty paired trials use the unchanged four-method and paid-seed policy.

| Warm update | Median request + reply HTTP-body bytes | Median elapsed, including computation | Median sender + receiver-handler CPU |
|---|---:|---:|---:|
| Raw full send | 16,814,026 | 1,871.920 ms | 89.128 ms |
| Ordinary zstd | 16,809,949 | 1,917.746 ms | 91.971 ms |
| Zstd prefix delta | 26,202 | 252.625 ms | 89.854 ms |
| Sparse COPY/LITERAL | 4,651 | 222.133 ms | 74.978 ms |

Sparse uses 99.97233857% fewer HTTP-body bytes than raw, with an 8.427023×
ratio of median elapsed times and approximately 15.88% lower median combined
CPU. Including seed and one update, raw uses median 33,623,823 body bytes and
3,747.770 ms; sparse uses 16,814,448 bytes and 2,169.994 ms; zstd prefix uses
16,835,999 bytes and 2,187.982 ms.

All 240 operations reopened exactly, totaling 4,026,040,320 compared bytes.
The unchanged independent checker agrees with raw receipts and author statistics,
and all five captures meet the completeness criteria. The
[complete append packet](evidence/2026-10-29-broad-wan/cases/append_4096/README.md)
retains the complete observations and custody.

## Completed case: 4 KiB truncation

Truncating each 16,777,216-byte base produces a 16,773,120-byte target. Thirty
paired trials retain the same four methods and separate paid seeds.

| Warm update | Median request + reply HTTP-body bytes | Median elapsed, including computation | Median sender + receiver-handler CPU |
|---|---:|---:|---:|
| Raw full send | 16,809,797 | 1,975.651 ms | 93.868 ms |
| Ordinary zstd | 16,809,797 | 1,946.954 ms | 94.925 ms |
| Zstd prefix delta | 26,198 | 249.798 ms | 96.581 ms |
| Sparse COPY/LITERAL | 550 | 227.314 ms | 70.224 ms |

Sparse uses 99.99672810% fewer HTTP-body bytes than raw, with an 8.691284×
ratio of median elapsed times and approximately 25.19% lower median combined
CPU. Including seed and one update, raw uses median 33,623,823 body bytes and
3,938.798 ms; sparse uses 16,814,576 bytes and 2,170.173 ms; zstd prefix uses
16,840,224 bytes and 2,216.160 ms.

All 240 operations reopened exactly, totaling 4,026,040,320 compared bytes.
Independent review agrees with every raw receipt and the author statistics;
all five captures meet the completeness criteria. The
[complete truncation packet](evidence/2026-10-29-broad-wan/cases/truncate_4096/README.md)
retains the observations, verification and custody.

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
