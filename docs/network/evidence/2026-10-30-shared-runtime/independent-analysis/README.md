# Independent local shared-runtime analysis

The registered local experiments confirm exact recovery and large conditional
reductions in serialized transfer bytes. They do **not** establish a speed or CPU
win: the runtime arm is slower and uses more CPU than raw transfer on every
target-only paired trial, including the unrelated control. This packet contains
local endpoint measurements; it makes no WAN or energy claim.

The analysis is pinned to measured source
`b6627deb7fb8c80a46c0e89d5e074afaf50f37f0`. The frozen measured executable has SHA-256
`49c0aa577105718fd4a7019ee719d1815e2b1f16e6d27e909632ecb228386d93`.
The later WAN-launcher change is outside this local experiment. Source custody
was checked against Git blobs at the measured revision, not the current tree.

## Completeness and fidelity

[results.json](results.json) independently accounts for all **480 operations**:
60 paired inputs, 240 checkpoint seeds, and 240 target transfers. Both raw files
contain exactly 662 rows in their registered order. All 480 operation receipts
report exact byte equality after closing and reopening the receiver store,
covering 1,799,742,000 compared bytes. All 120 adaptive/runtime projected selected
costs equal their observed serialized body costs. All 240 owned-scope cleanup
receipts occur after successful exact verification. No failed, interrupted,
extra, or missing operation was accepted.

The analyzer verifies these receipts and their pinned implementation; it does
not repeat byte comparisons against caches the measured harness already removed.
The actual byte comparisons were performed during the source-pinned measurement.
The checks include run/plan/executable identity, fixture object identity and
length, independently derived checkpoint/trace lengths, recipe encoding, scoped
identity, arm order, byte arithmetic, strict-minimum candidate choice, forced-arm
cost agreement, and terminal completion. [custody.json](custody.json) records
input hashes, 56 source-file hashes checked against the frozen commit, and the
hashes of the actual retained executable files.

## Observed target-only transfer results

Times below are medians in milliseconds over ten trials per cell. CPU is the
combined sender and receiver process CPU in this local implementation. Wall time
runs from input-ready through durable commit acknowledgement. A separate reopened
exact-byte verification follows that timer. Bytes are the sum of both serialized
HTTP-body-equivalent directions, including protocol requests and replies. There
was no HTTP connection, TLS, or WAN in these local runs.

| Target bytes | Case | Arm | Body bytes | CPU ms | Wall ms |
| ---: | --- | --- | ---: | ---: | ---: |
| 142,738 | Generated | Raw | 143,505 | 1.7675 | 8.8190 |
| 142,738 | Generated | Zstd | 56,725 | 1.1995 | 8.9685 |
| 142,738 | Generated | Adaptive | 55,033 | 2.2840 | 9.9460 |
| 142,738 | Generated | Runtime | 747 | 2.9895 | 11.3580 |
| 142,738 | One-bit edit | Raw | 143,505 | 2.0720 | 8.6715 |
| 142,738 | One-bit edit | Zstd | 56,725 | 2.4405 | 8.7825 |
| 142,738 | One-bit edit | Adaptive | 55,034 | 4.4380 | 11.6315 |
| 142,738 | One-bit edit | Runtime | 766 | 5.7100 | 12.6160 |
| 142,738 | Unrelated | Raw | 143,505 | 1.8470 | 9.1075 |
| 142,738 | Unrelated | Zstd | 143,505 | 2.1255 | 9.7340 |
| 142,738 | Unrelated | Adaptive | 143,505 | 2.2005 | 9.5385 |
| 142,738 | Unrelated | Runtime | 143,505 | 3.8705 | 11.7110 |
| 14,851,384 | Generated | Raw | 14,883,938 | 35.5870 | 53.8245 |
| 14,851,384 | Generated | Zstd | 3,513,668 | 38.7570 | 58.9700 |
| 14,851,384 | Generated | Adaptive | 3,201,258 | 86.3640 | 106.5535 |
| 14,851,384 | Generated | Runtime | 747 | 524.5780 | 552.2895 |
| 14,851,384 | One-bit edit | Raw | 14,883,938 | 34.9930 | 54.1675 |
| 14,851,384 | One-bit edit | Zstd | 3,513,672 | 38.2295 | 56.8440 |
| 14,851,384 | One-bit edit | Adaptive | 3,201,257 | 87.3960 | 106.8265 |
| 14,851,384 | One-bit edit | Runtime | 766 | 521.5790 | 544.3665 |
| 14,851,384 | Unrelated | Raw | 14,883,938 | 35.4185 | 52.0930 |
| 14,851,384 | Unrelated | Zstd | 14,883,938 | 38.8125 | 58.8945 |
| 14,851,384 | Unrelated | Adaptive | 14,883,938 | 57.1400 | 75.9250 |
| 14,851,384 | Unrelated | Runtime | 14,883,938 | 269.9580 | 290.8015 |

For generated and edited inputs, the runtime arm selects the shared-runtime
recipe/residual mode. Its target-only body reduction versus raw is respectively
99.4795% and 99.4662% for the small object, and 99.9950% and 99.9949% for the large
object. For unrelated bytes it chooses raw, saves no bytes, and still pays for
the unsuccessful prediction/candidate computation. Runtime has **0/10 CPU wins
and 0/10 wall-time wins against raw in each of these six cells**.

The conventional adaptive arm compares the configured raw, zstd, sparse, and
prefix candidates against the checkpoint. That checkpoint is not a previous
full trace. These results do not compare against the best possible prior-trace
cache, every zstd setting, rsync, or VCDIFF.

## Setup costs and practical cold start

The small checkpoint contains 346 bytes and costs 847 serialized body bytes to
install. The large checkpoint contains 3,382 bytes and costs 3,883 body bytes.
Every arm was seeded to make the measured setup equal. Raw and zstd do not need
this checkpoint, so their target-only cost is the appropriate practical cold
baseline. The table adds the runtime seed and target within each paired trial
before computing the median; it does not add independently computed medians.

| Target bytes | Case | Runtime seed + target bytes | CPU ms | Wall ms | Byte reduction versus raw target only |
| ---: | --- | ---: | ---: | ---: | ---: |
| 142,738 | Generated | 1,594 | 4.0435 | 25.8775 | 98.8892% |
| 142,738 | One-bit edit | 1,613 | 7.4455 | 25.4820 | 98.8760% |
| 142,738 | Unrelated | 144,352 | 5.4350 | 25.1565 | -0.5902% |
| 14,851,384 | Generated | 4,630 | 525.3170 | 564.3940 | 99.9689% |
| 14,851,384 | One-bit edit | 4,649 | 522.2505 | 559.0755 | 99.9688% |
| 14,851,384 | Unrelated | 14,887,821 | 270.6710 | 303.6045 | -0.0261% |

Negative savings mean additional bytes. Full paired comparisons against raw,
zstd, and adaptive are retained in [results.json](results.json): 18 warm
comparisons, 18 equal-setup seed-plus-target comparisons, and 12 practical cold
comparisons against raw/zstd target only. No checkpoint cost is silently
amortized across targets.

## Timing scopes and separately measured generation

The complete 72 metric cells retain medians, nearest-rank p95, minima, maxima,
sums, and sample counts for operation CPU/wall, receiver CPU/wall, exchange
CPU/wall, post-commit verification CPU/wall, both body directions, total body
bytes, and batch counts. With ten observations, this p95 is the observed maximum
and is not a strong estimate of a population tail. Receiver and exchange scopes
are nested inside the local operation; adding them would double count.
Exchange wall includes serialization, receiver work, durable storage and waiting;
it is not pure byte transmission time.

Generation is recorded once per paired input, separately from all four transfer
arms. Its median CPU/wall milliseconds are:

| Target bytes | Case | Generation CPU ms | Generation wall ms |
| ---: | --- | ---: | ---: |
| 142,738 | Generated | 0.5560 | 0.5690 |
| 142,738 | One-bit edit | 1.7860 | 1.8350 |
| 142,738 | Unrelated | 2.1385 | 2.1395 |
| 14,851,384 | Generated | 211.7755 | 212.7355 |
| 14,851,384 | One-bit edit | 211.8890 | 213.6980 |
| 14,851,384 | Unrelated | 238.2165 | 239.6955 |

This generation scope constructs the experimental workload, including the
prediction even for unrelated bytes. It is not an unavoidable raw/zstd cost for
transferring an existing file. It may be composed once with transfer intervals
only for an application that actually creates its data this way; that would be
a model composed from separate intervals, not a new directly observed clock.

## Independent analyzer qualification and reproduction

[analyze.rs](analyze.rs) is a separate Rust implementation of the checks and
statistics. It does not call the measurement executable. Its clean input passes,
and all 17 deliberately malformed in-memory variants are rejected, including
missing/duplicate/extra rows, failure or false verification, wrong source,
executable, plan, scope, fixture, mode, cost, ordering, metric scope, cleanup,
completion, and control case. See [negative-checks.json](negative-checks.json).
Original inputs are never edited by these checks.

A second independent implementation, [crosscheck.jq](crosscheck.jq), calculates
the 72 groups and 12 metrics directly from raw operation rows, including
per-pair seed-plus-target aggregation. It found zero differences from the Rust
statistics: [statistics-crosscheck.json](statistics-crosscheck.json).

[analysis-receipt.json](analysis-receipt.json) retains exact commands, exit codes,
compiler target, dependency hashes and analyzer executable custody. The executable
is kept in the task-owned local tooling cache rather than committed as a binary.
Reproduction requires the source-pinned raw inputs, plan files, source manifests,
frozen executables for custody checks, and the listed locked Rust dependencies.
The reported source/plan identity should never be replaced by current checkout
identity merely because later launch or documentation changes exist.

The findings apply to these synthetic model-derived and unrelated fixtures on
one local host, with deterministic rotating arm order. They establish byte
reduction under a shared model and expose its present compute cost. They do not
establish energy savings, arbitrary-file compression gains, cross-host timing,
or MVM integration.
