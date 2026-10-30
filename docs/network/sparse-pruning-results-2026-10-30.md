# SC-OPT-007: sparse pruning preserved bytes and reduced buffers, but failed CPU acceptance

**Disposition: candidate not adopted; active encoder restored to the published
baseline.** Exact-output and allocation gates passed. Two of three primary CPU
cells passed, but middle insertion missed its 1.10 ratio gate. Repeat and periodic
controls exceeded the allowed 5% CPU regression. All observations remain in the
packet; no pair was discarded, replaced or rerun to change acceptance.

This is a local Rust encoder experiment, not a new transfer or WAN result. The
previous [WAN campaign](broad-wan-results-2026-10-29.md) and
[adaptive selector measurements](codec-selection-results-2026-10-29.md) retain
their own source identities and limitations.

## Source and measured work

| Identity | Value |
|---|---|
| Published baseline and immutable oracle | `cc0199ffd255b9e2e002afe8c96f10462a3e474f` |
| Measured candidate and harness | `317c7f75b8b8355fbb2c74b4bad1944e4f0cb1e8` |
| Committed registration before acceptance collection | `5bd7c51832710176cca30bfd17ce071cfc80b6af` |
| Candidate encoder SHA-256 | `8d24c10c96de770d40d8c1ea24d5ae5427c95576d99936637c886ecf07d12735` |
| Restored encoder SHA-256 | `af31026c806c9046d62735b3ebdd03472e667103011a06b6f49ff76edb19e18e` |
| Normal timed executable SHA-256 | `e30407365f46bad4e6271382fc22961d984449f5176eef9322defeb4f9393592` |
| Separate diagnostic executable SHA-256 | `f1c523cfc575a759a311013f7b5881a404dbd09e40fc0af6af0592a04730604c` |

The candidate adds a checked lower bound before constructing sparse frame buffers.
For total literal bytes `B`, COPY count `C` and LITERAL count `L`, the bound is
`B + 13*C + 5*L + 137*max(ceil(B/61440), ceil((C+L)/256))`.
It rejects only candidates that cannot strictly beat Raw or the existing winner.
It preserves the original tie rule. The same linear input scans and operation
generation remain; it does not skip whole-base zstd prefix compression or change
the adaptive sender's other work. See the [reviewed design](../specs/2026-10-30-sparse-pruning-design.md).

All targets are **16,777,216 bytes (16 MiB)**. Insertions use bases shortened by
4 KiB so their target stays within the current client limit. The frozen workload
contains three primaries and four controls, with ten fresh-process old/new pairs
per case, five AB and five BA in alternating order. Pair-dependent seeded inputs
match between arms. The allocation campaign ran first, qualifying all fixtures
and intended rejection before the normal timed campaign.

## CPU and elapsed results

Times below are midpoint medians in **milliseconds**, measured from encoder entry
through return. The ratio is the **geometric mean of the ten paired old/new CPU
ratios**, not the quotient of the displayed medians. Values above one favor the
candidate. The primary gate required ratio at least 1.10 and at least seven strict
CPU wins. Controls required candidate/old geometric mean at most 1.05.

| Workload | Old CPU | Candidate CPU | Old elapsed | Candidate elapsed | Paired CPU ratio | Candidate CPU wins | CPU gate |
|---|---:|---:|---:|---:|---:|---:|---|
| 4 KiB prefix insertion | 17.7005 | 15.4940 | 18.3260 | 15.5860 | 1.116035 | 8/10 | Pass |
| 4 KiB middle insertion | 13.9840 | 12.9760 | 14.0800 | 13.0260 | 1.092043 | 10/10 | **Fail: below 1.10** |
| Unrelated replacement | 12.6180 | 7.3935 | 15.8370 | 8.3680 | 1.514101 | 9/10 | Pass |
| Exact repeat control | 11.1970 | 11.3475 | 11.2855 | 11.4470 | 0.936189 | 3/10 | **Fail: +6.816% paired CPU** |
| Localized 32-byte control | 13.2220 | 12.8210 | 13.3375 | 12.9310 | 1.044649 | 7/10 | Pass |
| Scattered-edit control | 7.2470 | 7.2580 | 7.2650 | 7.2655 | 1.022050 | 6/10 | Pass |
| Periodic-content control | 13.8075 | 13.8125 | 13.8675 | 13.8340 | 0.939837 | 4/10 | **Fail: +6.401% paired CPU** |

The control medians alone hide why the registered gates failed. Repeat pair 3
used 11.782 versus 24.818 ms CPU; periodic pairs 6, 7 and 9 also favored the old
encoder substantially. These remain part of the paired geometric means.
They are not removed as warmup or presumed noise. The
[performance disposition](evidence/2026-10-30-sparse-pruning/performance-disposition.json)
records every pair in the failed cells and distinguishes the statistics.

| Workload | Old CPU p95 | Candidate CPU p95 | Old elapsed p95 | Candidate elapsed p95 |
|---|---:|---:|---:|---:|
| Prefix insertion | 35.044 | 31.021 | 53.672 | 42.179 |
| Middle insertion | 17.915 | 15.965 | 20.692 | 16.301 |
| Unrelated replacement | 17.381 | 16.646 | 23.458 | 25.415 |
| Exact repeat | 12.569 | 24.818 | 12.854 | 38.420 |
| Localized edit | 32.226 | 24.321 | 35.557 | 24.997 |
| Scattered edit | 8.056 | 7.514 | 8.083 | 7.546 |
| Periodic content | 14.717 | 19.933 | 14.768 | 23.380 |

p95 uses nearest rank; with ten samples per arm it equals the maximum observed
value. This small experiment does not estimate a stable deployment tail.

Returned-recipe destruction is measured separately. Median drop CPU/wall values
are 0–3.5 / 0–4 microseconds across these cells; zeros reflect clock resolution.
Those tiny intervals have no standalone speedup claim. Full distributions,
encode-plus-drop sums of disjoint intervals, verification time and continuous
entry-to-release time are retained in the
[independent timing result](evidence/2026-10-30-sparse-pruning/independent-review/timing-result.json).
The continuous interval includes output verification and is not used as the
encoder-performance gate. No network-only duration is inferred by subtracting CPU.

## Frame allocation and unchanged encoded bytes

The diagnostic binary deliberately implements tracked reallocation as
allocate-copy-free. This measures actual requested frame-buffer capacities and
old/new overlap under a shared diagnostic policy. The timed binary uses the normal
allocator and excludes the tracer. These are **not process RSS or production
allocator peak-memory measurements**; input buffers, operation vectors and other
sender/receiver state are excluded.

| Workload | Old peak frame-buffer bytes | Candidate peak bytes | Peak reduction | Serialized recipe bytes, unchanged |
|---|---:|---:|---:|---:|
| Prefix insertion | 16,789,290 | 12,330 | 99.9266% | 4,123 |
| Middle insertion | 8,400,851 | 12,369 | 99.8528% | 4,136 |
| Unrelated replacement | 33,562,122 | 0 | 100% | Raw fallback; original 16 MiB |
| Exact repeat | 90 | 54 | 40% | 22 |
| Localized edit | 180 | 108 | 40% | 72 |
| Scattered edit | 16,822,601 | 16,822,601 | 0% | 25,658 in five frames |
| Periodic content | 20,550 | 12,330 | 40% | 4,123 |

These peak values are identical across all ten pairs within each workload.
**All 30 primary memory pairs pass the 50% reduction gate.** No primary rejected
candidate constructs frame buffers. The winning recipe's retained capacity is
unchanged, and every tracked allocation is freed after drop. The scattered
control still builds both candidates: this change does not reorder candidates or
predict a winner. Full cumulative, retained, growth and free accounting is in the
[allocation result](evidence/2026-10-30-sparse-pruning/independent-review/allocation-result.json).

Recipe bytes in the table exclude transfer framing and replies; they are not new
WAN byte measurements. The candidate changes no selected encoding bytes.

## Correctness, qualification and custody

Both campaigns contain 140 valid observations and 70 matching old/new pairs.
Every diagnostic row also matches its normal-run input and recipe receipt.
Each campaign reconstructs **120 selected recipes**, comparing **2,013,265,920
bytes** with the original target, and records **20 Raw fallbacks** retaining
335,544,320 original bytes. Raw fallbacks are not counted as decoded or transferred
recipes. The implementation tests additionally compare full ordered frame vectors
and operation counts directly with immutable legacy code, then apply both decoders.

Before collection, the candidate passed 202 release all-target tests, one doctest,
71 independent imported-store tests, 10 serial diagnostic tests, format checks,
normal and diagnostic Check/Clippy, and independent code/Rust/security review.
Focused coverage passed 37 tests and covered 228/233 lines and 14/14 LLVM functions
in `delta.rs`. Branch/MC/DC and statements were not separately collected; this is
not an asserted score for those metrics. The expected materialization RED and
setup failures remain separate from successful qualification.

The timing checker rejects eight deliberately corrupted inputs; the allocation
checker rejects another eight. Their valid synthetic negative cases confirm that
failed performance gates remain valid results rather than disappearing as invalid
records. Synthetic fixtures are explicitly labeled and are never campaign data.

The pre-edit baseline contains 21 additional observations and the initial
allocation smoke contains 14; neither is included in acceptance. A provenance
correction records that baseline harness tests preceded a driver-only
failure-recording patch and successful rebuild. Original history/output records
and the exact measured baseline harness are preserved, without manufacturing a
second historical test run. See the
[custody receipt](evidence/2026-10-30-sparse-pruning/recovered-qualification-custody.json).

| Acceptance raw file | SHA-256 |
|---|---|
| `measurements.jsonl` | `a8ef89bc65705abf76f19555592858567ba4e8d6ac0ec64aa7e23d74c395266d` |
| `allocations.jsonl` | `5e1c2fb43c4e6e54a678687824fd4054299857e71a4182d973a8179d2a990820` |

The host was an Apple M1 with eight logical CPUs and 16 GiB RAM, macOS 27.0 build
26A5378n, Rust 1.97.1/LLVM 22.1.6, native release with `hardware-sha256`. The timed
campaign ran 04:27:03–04:27:17 UTC. One-minute host load was 4.59 before and 4.34
after. No owned build/test/profile ran concurrently; other applications and labs
were untouched. There was no controlled thermal/core experiment or causal profile
of the negative tails. Shared-host variation is a limitation, **not proof that the
host caused the failed controls**. The local oracle module versus public-library
code-generation context is also not isolated by this experiment.

## Delivery and next experiment

The active `delta.rs` is restored byte-for-byte to the baseline. The rejected
candidate and its pruning-specific tests remain in Git at `317c7f7` and as
[archived source snapshots](evidence/2026-10-30-sparse-pruning/candidate-source/README.md).
They are not active production or test code after rollback. The harness and
immutable oracle remain available, but historical candidate reproduction requires
the measured commit: on the restored tree the `new` arm calls the restored encoder
and compares baseline against baseline. A supplied revision label cannot select code.

Fresh post-rollback qualification passes 188 tests across all active release targets, including
the normally ignored large HTTP tests. The 14 pruning-specific tests belong to
the archived candidate; their successful candidate qualification is retained
separately rather than represented as active delivery tests.

The main result is a useful boundary: avoiding discarded buffers can dramatically
reduce that allocation category, while unchanged scans and other codec work keep
CPU benefits workload-dependent. This experiment does not qualify this candidate
as the general optimization to ship.

The next candidate should address whole losing-codec evaluations. An
[unimplemented follow-up note](prefix-candidate-bound-note.md) derives a possible
prefix-codec lower bound from Zstandard block framing. It requires independent
proof, truthful skipped-candidate diagnostics, fresh correctness tests and its own
registered measurements. No new WAN, energy, larger-client, FIPS or MVM result is
claimed here; the later MVM bridge still includes the small-orchestrator case.
