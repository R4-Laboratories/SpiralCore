# Combined runtime and rolling-cache gateway results

The shared Rust object gateway now combines the existing ternary/Kuramoto/CA
prediction with acknowledged rolling-cache reuse in one persistent session.
On the calibrated **80 ms RTT / 10 Mbps** link, its complete mixed session used
**58.26% fewer captured IP bytes, 38.08% less summed endpoint process CPU and
52.65% less elapsed time than prepared whole-object zstd**. Compared with the
separate Runtime and Warm profiles, it used about half the captured bytes.

These are three descriptive matched trials on one Linux runner with real QUIC
traffic through isolated network namespaces and calibrated impairment. They are
not a new Internet-WAN measurement or automatic compression of arbitrary TUN
traffic. All 288 delivered objects passed exact reopened byte comparisons.

Implementation [PR #29](https://github.com/R4-Laboratories/SpiralCore/pull/29),
measured source `7e698aefe88f3729bc1b6e4f7ca72d9f036c110d`,
[run 41394672925](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41394672925).
The [evidence index](evidence/2026-11-02-combined-gateway/README.md) links the
registration, raw receipts, full analysis, independent review and immutable archive.

## What changed

`CombinedSession` retains one prepared prediction, one acknowledged rolling
target and reusable codec contexts. Generated content can use RuntimeV1 directly,
without Warm's forced raw first-object seed. Every validated target becomes the
rolling base, so a later unrelated object's edits can use SparseV1. Exact byte
repeats take the current-receiver cache probe before engine residual/hash work.
The gateway's independent input hash remains charged equally to every arm.

Runtime offers reference the fixed checkpoint; Sparse offers reference the last
acknowledged object. Schema is eligible only after its explicit publication.
Only a validated Published acknowledgment promotes the rolling base. Missing
state is an error; invalid acknowledgments clear readiness; namespace changes
discard the prepared session. No new wire mode, decoder or mathematical kernel
was substituted. The bounded single-session admission remains 512 MiB, with
multi-session resource admission still belonging to the caller.

The [service guide](runtime-object-gateway.md#combined-runtime-and-rolling-cache)
shows `mode:"combined"`, checkpoint/recipe/schema inputs and normal send actions.
This is a cooperating object service using the shared gateway transport. The
separate packet/TUN path remains available and unchanged.

## Workload and accounting

The campaign completed **24 persistent sessions, 336 operations and 288 object
deliveries**, totaling **2,289,396,384 requested object bytes**. Each session
delivered six objects at each of two sizes: **1,047,202 bytes** and
**14,851,384 bytes** (about 14.16 MiB). Sequence: generated runtime output,
512-byte residual change, repeat, unrelated opaque bytes, localized opaque edit,
and its repeat. The same deterministic trial fixtures served every arm.

Controls were prepared whole-object zstd with raw fallback, separate Runtime,
and separate rolling Warm. All arms used the same authenticated reliable QUIC
channel, durable publication, verified-cache probe and receiver readback.
Combined paid checkpoint and schema publication; Runtime paid its checkpoint;
Warm paid its schema and counted its raw generated seed as the first delivery.
Preparation, replay, input reads and hashing were charged. Raw-only is retained
in the historical campaign, not rerun in this four-arm comparison.

Directional calibration measured **80.281–80.300 ms RTT** and
**9.612–9.795 Mbps**; the unshaped profile measured 0.040–0.048 ms RTT.
All captures completed without capture drops. Finite shaped queues dropped
**1,544 packets**, retained in retransmission/time costs. Host routes and frozen
source/binaries remained unchanged. Three trials are descriptive; there is no
outlier removal, confidence interval, p95 or physical-energy claim.

## Complete mixed sessions

Medians of three sessions, each containing both size tiers and twelve deliveries.
Captured IP totals count both directions and transport overhead through closure.
CPU is measured process time, summed per trial before its median. The client
elapsed interval includes setup, all sends, finish and receiver reopened readback.
It starts inside `run_inner` after CLI config validation/output reservation and
ends before final report writing and endpoint shutdown; separate harness elapsed
includes launch, capture drain and cleanup. CPU and elapsed overlap and are not
added together. The analysis does not invent send-only time by subtraction.

| Link | Profile | Captured IP bytes | Sender CPU s | Receiver CPU s | Summed CPU s | Client elapsed s |
|---|---|---:|---:|---:|---:|---:|
| Unshaped | Whole-zstd | 40,266,059 | 0.822929 | 0.682217 | 1.505146 | 1.126738 |
| Unshaped | Runtime | 33,395,197 | 0.789906 | 0.669853 | 1.459759 | 1.142403 |
| Unshaped | Warm | 33,395,072 | 0.683080 | 0.625970 | 1.309050 | 0.990653 |
| Unshaped | Combined | 16,719,671 | 0.654904 | 0.541659 | 1.197458 | 1.040818 |
| 80 ms / 10 Mbps | Whole-zstd | 41,295,031 | 1.358837 | 1.775044 | 3.133881 | 35.520383 |
| 80 ms / 10 Mbps | Runtime | 34,229,935 | 1.222105 | 1.581789 | 2.792965 | 30.221147 |
| 80 ms / 10 Mbps | Warm | 34,235,354 | 1.136781 | 1.521772 | 2.662683 | 29.867256 |
| 80 ms / 10 Mbps | Combined | 17,237,193 | 0.888773 | 1.017792 | 1.911030 | 16.821350 |

Matched reductions are computed per trial before taking the median, rather than
dividing the table's independent medians:

| Combined relative to | Link | IP reduction | CPU reduction | Elapsed reduction |
|---|---|---:|---:|---:|
| Whole-zstd | Shaped | 58.26% | 38.08% | 52.65% |
| Runtime | Shaped | 49.64% | 31.58% | 44.31% |
| Warm | Shaped | 49.65% | 27.84% | 43.68% |
| Whole-zstd | Unshaped | 58.48% | 20.44% | 7.87% |
| Runtime | Unshaped | 49.93% | 17.97% | 8.89% |
| Warm | Unshaped | 49.93% | 8.31% | **−4.78%** |

Warm alone remains faster on the unshaped link for this mixed sequence. Combined
is therefore a demonstrated byte and shaped-link improvement, not the fastest
profile under every condition. Current same-run controls matter: CPU values from
the earlier campaign are not pooled with these new runner measurements.

## Where the savings occur

These are shaped-link operations on **14,851,384-byte objects**, after their
group setup. Logical bytes include both object-channel directions and batch
prefixes, but exclude QUIC/IP overhead. CPU columns measure sender operation
work and receiver Endpoint-handler work; concurrent activity within those
process windows is included. Elapsed includes input loading/hash, selection,
network wait, reconstruction and validated publication.

| Operation | Profile | Logical bytes | Sender CPU ms | Receiver handler CPU ms | Elapsed ms |
|---|---|---:|---:|---:|---:|
| Generated | Whole-zstd | 3,194,293 | 195.273 | 30.522 | 2,989.189 |
| Generated | Runtime | 771 | 23.574 | 56.382 | 245.231 |
| Generated | Warm seed | 14,883,930 | 388.409 | 32.712 | 12,786.241 |
| Generated | Combined | 771 | 100.379 | 57.431 | 322.033 |
| Residual edit | Whole-zstd | 3,194,762 | 160.027 | 28.094 | 2,978.066 |
| Residual edit | Runtime | 1,571 | 113.797 | 55.468 | 333.832 |
| Residual edit | Combined | 1,374 | 116.029 | 41.138 | 322.119 |
| Opaque edit | Whole-zstd | 14,883,962 | 379.104 | 27.043 | 12,863.228 |
| Opaque edit | Runtime | 14,883,962 | 390.151 | 26.596 | 12,860.579 |
| Opaque edit | Warm | 1,374 | 73.843 | 26.140 | 265.411 |
| Opaque edit | Combined | 1,374 | 82.261 | 26.360 | 274.108 |
| Residual repeat | Whole-zstd | 230 | 93.277 | 17.184 | 191.106 |
| Residual repeat | Runtime | 262 | 105.760 | 17.250 | 203.535 |
| Residual repeat | Combined | 230 | 16.071 | 17.176 | 113.771 |
| Unrelated | Whole-zstd | 14,883,962 | 400.006 | 26.493 | 12,876.701 |
| Unrelated | Combined | 14,883,962 | 439.345 | 26.468 | 12,927.017 |

Combined executed RuntimeV1 for each fresh generated object, SparseV1 for the
residual and opaque edit, and Raw Begin cache probes for both repeats. The
runtime mathematics is active, not merely stored as an artifact. The residual
winner was measured rather than forced; choosing the rolling delta reduced it
from Runtime's 1,571 to 1,374 logical bytes.

At **1,047,202 bytes**, Combined generated output used 771 bytes / 174.805 ms
versus zstd's 88,481 / 423.364 ms; its residual used 1,374 / 174.849 ms versus
89,002 / 283.103 ms. The opaque edit used 1,374 / 172.407 ms versus
1,049,988 / 1,103.428 ms. All exact-repeat Combined operations used **230 logical
channel bytes**. This is a different framing/accounting layer from the historical
600-byte HTTP-body warm result; it is not a claimed 600-to-230 wire optimization.

## Setup, amortization and remaining costs

The following group intervals include codec/prediction preparation and both
actual auxiliary publications. They exclude the separate connection setup,
preceding group and final readback; full session costs above include those.

| Size | Paid interval | Combined logical bytes | Zstd logical bytes | Combined elapsed s | Zstd elapsed s |
|---|---|---:|---:|---:|---:|
| 1,047,202 B | Setup only | 4,416 | 0 | 0.169297 | 0.000020 |
| 1,047,202 B | Setup + first delivery | 5,187 | 88,481 | 0.344102 | 0.423385 |
| 1,047,202 B | Setup + all six | 1,058,383 | 2,277,907 | 2.108987 | 3.053093 |
| 14,851,384 B | Setup only | 4,140 | 0 | 0.197768 | 0.000013 |
| 14,851,384 B | Setup + first delivery | 4,911 | 3,194,293 | 0.519660 | 2.989204 |
| 14,851,384 B | Setup + all six | 14,892,081 | 36,157,439 | 14.270994 | 32.033068 |

Setup is not free. Small setup-plus-first delivery uses **33.84% more endpoint
stage CPU** than zstd despite 18.63% lower elapsed; large setup-plus-first saves
13.63% stage CPU and 82.62% elapsed. The second group's smaller setup traffic
reflects the actual already-present schema/checkpoint handling, not an assumed
full upload or omitted charge. Every cumulative prefix remains in the analysis.

There is more sender optimization available. Large generated Combined uses
**4.28× Runtime's sender CPU** at identical 771-byte transfer cost. This comparison
does not isolate the cause within candidate preparation. Large unrelated input
sends the same bytes as zstd, with **13.79% more sender CPU** and 0.39% higher
elapsed; small unrelated input has 8.95% higher sender CPU and 16.75% higher
elapsed. These losses are retained. The next bounded optimization should prune
provably losing candidates and reuse exact prepared predictions without removing
verification or changing the winning bytes.

## Delivery and scope

Local final checks passed 15 gateway-core and 36 gateway tests, plus strict
Clippy; Linux passed 15 core and 38 gateway tests, release build and strict Clippy.
Five new transfer tests and 63 affected existing tests passed, with one
preexisting profiling test ignored. The independent TUN regression also passed
12 sessions and 48 exact application flows. New correctness tests cover missing
bases/checkpoints, eviction, invalid acknowledgments, namespace poisoning,
capacity rejection and same-offer repeat recovery.

The original 38,061,842-byte campaign ZIP, including full captures and frozen
executables, is in the existing private Hugging Face dataset. Its SHA-256 is
`258ffa5d58fd8b9957285e020a5d6128179725a996bbd11348c2508adfc065a3`.
All 1,029 artifact entries, 162 source entries and three executables verified;
the executed workflow blob matches the measured source. A fresh immutable
download matched the original ZIP. Compact original receipts and all trials
remain in Git, with independent arithmetic review.

This milestone combines the existing mechanisms at the cooperating object
gateway. A real two-host Internet run, bounded flow batching for ordinary TUN
traffic, adapting this exact Combined profile into MVM's authorized mailbox
boundary, wider workload qualification and physical-energy measurement remain
separate work. No universal compression ratio or exactly shared physical clock
is implied by these synthetic deterministic-generation results.
