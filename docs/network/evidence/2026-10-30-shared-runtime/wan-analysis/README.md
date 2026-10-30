# Independent WAN pilot analysis

This real WAN pilot confirms exact recovery and a large conditional byte
reduction. For the generated and one-bit-edited 14,851,384-byte objects, the
runtime arm completed faster than raw transfer, including its encoding and
receiver work. **It was slower than zstd and the conventional adaptive arm and
used substantially more measured CPU than every baseline.** The unrelated
control saved no target bytes and was slower. This is a three-trial pilot, not
the previously proposed 30-trial WAN acceptance campaign or an energy result.

The sender was an x86-64 Linux GitHub Actions runner; the receiver was the native
ARM64 macOS host, reached through a temporary Cloudflare HTTPS tunnel. The run
used the authenticated existing HTTP adapter and durable store. Reported bytes
are actual request and response body bytes; they exclude HTTP/TLS/IP/TCP headers,
retransmissions and relay traffic. The pilot does not measure full-path wire
bytes or isolated byte transmission time.

## Source, artifact custody and exact recovery

Measured WAN source is `35114daef384186dacd6123221ec9b5db60b3fcc`, GitHub run
`41062532491`. The transfer crate, Cargo files, measurement harness and reference
packet are byte-identical to local-measurement source
`b6627deb7fb8c80a46c0e89d5e074afaf50f37f0`; the later revision fixes the launcher.
The native receiver retains its earlier frozen executable, whose corresponding
code identity was verified separately from the revision metadata.

The analyzer verified all **72 operations**: 36 checkpoint seeds and 36 targets
across nine paired inputs (three cases × three trials × four arms). The sender
file has exactly 164 rows and ends in successful completion with verification
pending. All 72 acknowledged operations join uniquely by scope and phase to
successful exact-read receipts after receiver shutdown and store reopen. That
receiver file has 74 rows including its header and completion. These comparisons
cover **534,771,576 bytes**. Each of the three cases has 24 successful seed/target
comparisons. All 18 adaptive/runtime projected selected costs equal actual body
costs. No failed, missing, or extra observation or outlier was discarded.

The retained native receiver plan is byte-identical to the Linux sender plan,
including the complete generated trace identity. Successful native regeneration
and exact reads qualify
these specific fixtures across the two architectures; it does not prove every
future floating-point execution on every platform.

Custody checks include 50 remote source-manifest entries against exact Git
blobs; all 29 retained artifact-manifest members; downloaded ZIP size and SHA-256
against GitHub artifact `12982310131`; actual Linux sender and native verifier
and receiver executable bytes; registration, job and run identities; and
source/plan/executable bindings in both observation streams.

| Item | SHA-256 |
| --- | --- |
| Sender observations | `9c5b6c065d7a29b808108a3f59907d259383f55bcfdeb198fc1f16bcd9ab549e` |
| Receiver exact-read observations | `c1707cde08665c42f8fb3cb38e9316c6d1a3a0d087bbce38f0df7e67d163dfeb` |
| Byte-identical sender/receiver plan | `310f31590ddccced6fb6d6b1eab374c519fead41a3c1d0c7fa39e21aeaf62fbc` |
| Downloaded artifact ZIP | `cd3918063453ef3c81a7dec063711265f50327aa02b5b66d4bfe4bfc085a1648` |
| Linux sender executable | `c91be1394ef967f29d1b84fda1345310ed7cd570823c0fad65506c8b8777f0be` |
| Native receiver executable | `7b79c143c6e010804ed9a6d990c3333485344607e150d7200965a51f5eba1162` |
| Native verifier executable | `49c0aa577105718fd4a7019ee719d1815e2b1f16e6d27e909632ecb228386d93` |

The complete identity join is [receiver-joins.json](receiver-joins.json), and the
file-level custody record is [custody.json](custody.json). The analyzer validates
source-bound exact-read receipts; it does not rerun network transfers or perform
another physical read of the receiver cache itself.

## Target-only observations

Each object contains **14,851,384 bytes**. All times below are milliseconds.
`CPU subtotal` sums sender process CPU and reported receiver-handler CPU **within
each trial before computing its median**. It excludes receiver HTTP read/network
work, TLS termination and relay CPU. Sender and receiver medians should not be
added to reproduce it. Wall time runs from input-ready through durable commit
acknowledgement and already includes encoding, requests, receiver processing and
durability. With only three observations, nearest-rank p95 equals the observed
maximum and is not a qualified population-tail estimate.

| Case | Arm | Both-direction body bytes | CPU subtotal median | CPU subtotal p95 | Wall median | Wall p95 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Generated | Raw | 14,883,938 | 64.016 | 82.848 | 1,289.771 | 1,364.490 |
| Generated | Zstd | 3,513,668 | 68.321 | 68.890 | 395.862 | 469.149 |
| Generated | Adaptive | 3,201,258 | 154.410 | 161.462 | 442.376 | 482.692 |
| Generated | Runtime | 747 | 846.692 | 846.805 | 956.126 | 959.344 |
| One-bit edit | Raw | 14,883,938 | 75.637 | 77.774 | 1,305.359 | 1,356.199 |
| One-bit edit | Zstd | 3,513,672 | 72.507 | 75.480 | 441.491 | 454.719 |
| One-bit edit | Adaptive | 3,201,257 | 149.040 | 153.273 | 514.517 | 778.704 |
| One-bit edit | Runtime | 766 | 853.439 | 908.484 | 977.758 | 1,107.051 |
| Unrelated | Raw | 14,883,938 | 83.167 | 86.067 | 1,169.926 | 1,360.009 |
| Unrelated | Zstd | 14,883,938 | 67.983 | 88.801 | 1,463.649 | 1,520.865 |
| Unrelated | Adaptive | 14,883,938 | 118.013 | 122.213 | 1,417.490 | 1,521.950 |
| Unrelated | Runtime | 14,883,938 | 589.310 | 592.005 | 1,862.754 | 1,911.397 |

Generated and edited target body reductions versus raw are respectively
**99.994981% and 99.994854%**. The median paired wall speedups against raw are
**1.349× and 1.325×**, with three of three paired wins in each case. Against
zstd, those speedups are **0.419× and 0.462×**, meaning runtime is slower. Runtime
has zero of three wall wins against zstd or adaptive in every case and zero CPU
subtotal wins against every baseline in every accounting scenario. Its warm
CPU subtotal is a median paired **13.228× and 11.283×** raw for the favorable
cases. These paired ratios are computed before aggregation and need not equal
ratios of the displayed medians.

The runtime arm chooses the shared-runtime recipe/residual for generated and
edited traces. For unrelated bytes the candidate is rejected and raw is chosen;
the failed prediction's compute remains inside the timer. This is a measured
fallback penalty, not a compressed-data success.

## CPU and exchange scopes

The following medians make the two compute components and request path visible.
They are nested or related scopes and must not be summed into an additional wall
time. Exchange includes request serialization, HTTP waiting, receiver work and
reply handling. **It is not the time spent only sending bytes.**

| Case | Arm | Sender CPU ms | Reported receiver CPU ms | Exchange wall ms |
| --- | --- | ---: | ---: | ---: |
| Generated | Raw | 27.724 | 36.292 | 1,276.842 |
| Generated | Zstd | 31.326 | 36.754 | 369.532 |
| Generated | Adaptive | 118.777 | 35.633 | 331.009 |
| Generated | Runtime | 584.390 | 262.415 | 374.316 |
| One-bit edit | Raw | 26.697 | 49.137 | 1,292.513 |
| One-bit edit | Zstd | 31.364 | 41.259 | 415.916 |
| One-bit edit | Adaptive | 117.335 | 35.938 | 402.621 |
| One-bit edit | Runtime | 582.791 | 265.422 | 392.198 |
| Unrelated | Raw | 27.339 | 51.639 | 1,157.175 |
| Unrelated | Zstd | 36.448 | 31.109 | 1,442.349 |
| Unrelated | Adaptive | 64.377 | 53.636 | 1,364.084 |
| Unrelated | Runtime | 533.516 | 51.168 | 1,344.364 |

All 13 metrics, including exchange CPU, receiver wall, verification CPU/wall,
both body directions and batch counts, retain median, p95, minimum, maximum,
sum and sample count in [results.json](results.json). Remote `verification_*`
times refer to client post-ack identity work. The later exact receiver byte
comparison is separately qualified and is outside these transfer timers.

## Seed costs, equal setup and practical cold start

Every arm receives a fresh 3,382-byte checkpoint, costing 3,883 body bytes. Raw
and zstd do not need that seed. Accordingly, practical cold-start comparisons
use **runtime seed plus target versus raw/zstd target only**. There is one target
per fresh scope; no seed cost is silently amortized.

| Case | Runtime seed + target bytes | CPU subtotal median ms | CPU subtotal p95 ms | Wall median ms | Wall p95 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Generated | 4,630 | 851.216 | 852.532 | 1,061.108 | 1,065.053 |
| One-bit edit | 4,649 | 861.661 | 915.503 | 1,078.554 | 1,211.010 |
| Unrelated | 14,887,821 | 597.976 | 600.933 | 1,960.758 | 2,017.699 |

Cold favorable body reductions versus raw target only remain **99.968893% and
99.968765%**. Median paired cold wall speedups versus raw are **1.215× and
1.206×**, with three of three wins; versus zstd they are **0.384× and 0.421×**,
with zero wins. Unrelated cold traffic has **0.026089% extra body bytes**, zero
wall wins against raw/zstd and a larger CPU subtotal.

[paired-comparisons.csv](paired-comparisons.csv) and the JSON retain every
comparison: nine warm target-only cells, nine equal-setup seed-plus-target
cells, and six practical cold-start cells. Equal-setup sums are formed inside
each pair before statistics. The conventional adaptive arm does use the
checkpoint base, so its required seed remains in its cold total.

## Separate workload generation

Fixture generation occurs once before each pair's four arms. Its median CPU /
wall milliseconds are **472.775 / 472.827** generated, **471.300 / 471.337**
edited, and **499.727 / 499.803** unrelated. It is workload creation, including
prediction construction even for the unrelated control. It must not be assigned
as an unavoidable raw/zstd cost for transferring an already existing file.
The full p95 and other generation statistics are retained separately. Combining
generation with a transfer interval describes only an application that actually
creates data this way and is a composition of intervals, not another directly
measured end-to-end clock.

## Analysis qualification and limits

[analyze.rs](analyze.rs) is independent receipt-validation and arithmetic code,
adapted from the qualified local analyzer. It does not invoke the measured
program. Clean input passes; **16 sender-data mutations and 11 receiver-join
mutations** are rejected. The checks cover omissions, duplicates, failures,
wrong source/plan/executable/scope/object/length/hash, false exact-read claims,
candidate costs, order, metric scope and completion. Mutations operate on memory
copies and do not alter any measurement input.

[crosscheck.jq](crosscheck.jq) independently reconstructs all **36 metric cells
× 13 metrics** and **24 paired comparisons** directly from sender observations.
All cell statistics match exactly; paired floating ratios match within relative
or absolute `1e-12`. There are no mismatches. See
[statistics-crosscheck.json](statistics-crosscheck.json).

An analyzer-only initial build failed because a generated edit contained a
literal backslash-n between JSON fields. The failure output and exit code are
retained. Replacing it with a real newline fixed the build; the first analyzer
execution then passed all checks. No runtime code or raw observations were
modified. [analysis-receipt.json](analysis-receipt.json) records exact commands,
successful and failed exits, source and executable custody. The analyzer binary
is retained in a task-owned local tooling directory, outside Git.

The prior sealed local-analysis packet is unchanged. The WAN dose is three
trials per case with deterministic rotating arm order, synthetic public inputs,
one relay route and one pair of hosts. It does not satisfy the proposed
30-randomized-trial, 80-ms/10-Mbps shaped WAN gate or measure loss/jitter matrices,
full-path byte overhead, energy, MVM integration, every zstd setting, rsync,
VCDIFF, or a conventional prior-full-trace cache baseline. The byte win is
conditional on the shared model; CPU reduction remains unresolved.
