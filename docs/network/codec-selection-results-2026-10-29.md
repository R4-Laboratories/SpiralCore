# Byte-first codec selection: local results, October 29, 2026

The opt-in Rust selector chose the smallest measured configured encoding in all
**240 paired inputs**. All **1,800 seed/target operations** reopened exactly,
comparing **16,043,212,800 bytes**. This closes the fixed sparse sender's cold
compression gap, while retaining its small warm updates. It also exposes a real
cost: evaluating losing codecs increases CPU, particularly for warm sparse edits.

This is a **local serial encoding and durable-publication experiment**, not a new
WAN speed result. The earlier [real WAN campaign](broad-wan-results-2026-10-29.md)
remains tied to its original explicit-codec source and measurements. No network-only
send time or energy result is inferred here.

## Source, scope and acceptance

- Runtime and harness: `850272f01d580d1cf7564d25fb628fd50f58d956`.
- Registration committed before timing: `791ec790eca25bd3f79cc21bdb38cfa27013b32a`.
- Native release executable SHA-256:
  `c165a97205f858578f3c25d02464a725503660e21cf7c318811201f548c4d3fd`.
- Rust 1.97.1, `aarch64-apple-darwin`, opt-in `hardware-sha256`, pinned Cargo lockfile.
- Apple M1, eight logical CPUs, 16 GiB RAM, macOS 27.0. Shared developer host;
  other labs were not stopped or reconfigured. No owned builds/tests/profiles
  ran during collection. This was not a dedicated or thermally controlled host.
- Thirty paired trials in each of eight case/size strata; five methods per input:
  Raw, chunked zstd, sparse, zstd prefix and adaptive. Deterministic cyclic arm
  order balances positions, not randomization or every carryover effect.
- Receipts for all 1,200 targets and 600 separately paid Raw seeds are retained. A separate
  three-trial smoke run completed 180 exact operations and is excluded below.
- Each arm uses a fresh scoped cache, equal durability and disabled optional DAG
  warming. The receiver closes/reopens between seed and update, so these local
  results do not reuse the previous campaign's retained verified in-memory base.

Body counts are actual serialized request/reply lengths plus the defined 16-byte
reply timing fields: **HTTP-body-equivalent bytes**. No HTTP socket, TLS or WAN
traffic occurred in this timing harness. The integration suite separately verifies
the cost through the real loopback HTTP client/server. Operation wall time includes
all candidate encoders, rejected candidates, hashing, serialization, receiver work
and durable publication. Process CPU covers both sides serially; receiver and
exchange counters are nested subsets. Never add them again to the total.
Fixture generation, endpoint open/close, later reopened comparison, result writes
and cleanup are excluded equally. See the [measurement contract](codec-selection-measurement.md).

## Selection results

All values below are medians of 30 observations. CPU and wall times are milliseconds.
Raw is an unchanged configured control in the same run. These are ratios/comparisons
of cell medians; the raw records also retain per-pair ratio distributions.

| Size | Case | Selected mode in all 30 trials | Raw body bytes | Adaptive body bytes | Fewer body bytes |
|---|---|---|---:|---:|---:|
| 1 MiB | Cold random | Raw | 1,051,338 | 1,051,338 | 0% |
| 1 MiB | Cold structured | Chunked zstd | 1,051,338 | 77,765 | 92.6032% |
| 1 MiB | Warm 32-byte midpoint change | Sparse | 1,051,338 | 600 | 99.9429% |
| 1 MiB | Unrelated structured replacement | Zstd prefix | 1,051,338 | 4,835 | 99.5401% |
| 16 MiB | Cold random | Raw | 16,814,026 | 16,814,026 | 0% |
| 16 MiB | Cold structured | Chunked zstd | 16,814,026 | 1,179,589 | 92.9845% |
| 16 MiB | Warm 32-byte midpoint change | Sparse | 16,814,026 | 600 | 99.9964% |
| 16 MiB | Unrelated structured replacement | Zstd prefix | 16,814,026 | 6,395 | 99.9620% |

The unrelated-structured result is **not evidence of useful base reuse**. Its
prefix engine uses one whole-target level-three frame, whereas the configured
ordinary control resets level-one compression at 60 KiB chunks. A whole-object
no-base zstd comparison remains unrun. The prefix wire mode still requires an
authorized receiver base, even when this particular input does not benefit from
that base. No claim of best possible compression or novel mathematical gain follows.

| Size | Case | Raw CPU ms | Adaptive CPU ms | Raw wall ms | Adaptive wall ms | Raw p95 wall ms | Adaptive p95 wall ms |
|---|---|---:|---:|---:|---:|---:|---:|
| 1 MiB | Cold random | 3.4485 | 3.4015 | 17.0270 | 17.9025 | 20.940 | 23.898 |
| 1 MiB | Cold structured | 6.8820 | 5.8110 | 18.9220 | 17.4750 | 20.867 | 19.005 |
| 1 MiB | Warm 32-byte change | 4.0525 | 8.2580 | 12.6180 | 17.5595 | 17.795 | 22.967 |
| 1 MiB | Unrelated structured | 4.7385 | 10.5525 | 11.9095 | 17.8515 | 18.378 | 20.879 |
| 16 MiB | Cold random | 37.0430 | 41.1760 | 55.0840 | 58.0985 | 68.324 | 63.308 |
| 16 MiB | Cold structured | 37.2955 | 27.6585 | 54.0565 | 45.0380 | 61.152 | 53.090 |
| 16 MiB | Warm 32-byte change | 35.2865 | 86.4015 | 48.9755 | 99.9090 | 72.280 | 119.802 |
| 16 MiB | Unrelated structured | 35.4830 | 78.8440 | 48.5245 | 92.1220 | 54.939 | 110.441 |

Small differences on this shared host must not be attributed to an algorithmic
speedup. In particular the 1 MiB random adaptive CPU median is slightly below
Raw despite additional work; the 16 MiB random case instead shows **11.16% more
CPU** and **5.47% more median wall time**, with identical bytes. Local durable
publication has no WAN serialization delay, so byte savings do not automatically
make local elapsed time smaller.

## Extra selection cost versus already choosing the winning codec

The forced winning codec and adaptive encode the same selected representation.
This comparison exposes the additional candidate work, with normal host variation.

| Size | Case / fixed winner | Fixed CPU ms | Adaptive CPU ms | CPU change | Fixed wall ms | Adaptive wall ms |
|---|---|---:|---:|---:|---:|---:|
| 1 MiB | Cold structured / zstd | 5.3620 | 5.8110 | +8.37% | 17.8860 | 17.4750 |
| 1 MiB | Localized / sparse | 5.2940 | 8.2580 | +55.99% | 14.6535 | 17.5595 |
| 1 MiB | Unrelated structured / prefix | 9.3050 | 10.5525 | +13.41% | 16.8545 | 17.8515 |
| 16 MiB | Cold structured / zstd | 27.5235 | 27.6585 | +0.49% | 45.0520 | 45.0380 |
| 16 MiB | Localized / sparse | 51.4550 | 86.4015 | +67.92% | 64.8780 | 99.9090 |
| 16 MiB | Unrelated structured / prefix | 68.7025 | 78.8440 | +14.76% | 81.8385 | 92.1220 |

The byte-minimum feature gate passes. **A general CPU or elapsed-time improvement
does not.** The API stays opt-in; existing explicit senders and frozen WAN plans
are unchanged. The next optimization should avoid unnecessary candidate work,
using this selector as a byte-decision reference. Exact conservative pruning and
separately measured heuristics are different contracts and must be labeled so.

## Paid seeding

Warm rows above exclude the separately measured seed. The table below includes
one Raw seed plus one target; these are medians of each trial's sum, not sums of
separately calculated medians. Receiver reopen/verification remains outside timing.

| Size | Warm case | Raw lifecycle body bytes | Adaptive lifecycle body bytes | Raw lifecycle CPU ms | Adaptive lifecycle CPU ms | Raw lifecycle wall ms | Adaptive lifecycle wall ms |
|---|---|---:|---:|---:|---:|---:|---:|
| 1 MiB | Localized | 2,102,676 | 1,051,938 | 7.8710 | 11.7480 | 29.7230 | 36.5675 |
| 1 MiB | Unrelated structured | 2,102,676 | 1,056,173 | 9.5245 | 15.4675 | 28.7390 | 34.8575 |
| 16 MiB | Localized | 33,628,052 | 16,814,626 | 71.7845 | 123.6335 | 102.9920 | 154.8620 |
| 16 MiB | Unrelated structured | 33,628,052 | 16,820,421 | 71.9145 | 115.7885 | 102.6880 | 147.0355 |

## Correctness, review and preserved negatives

The release suite passes **185 tests**, including the two otherwise ignored large
HTTP cases; the doctest passes, and all **71 imported object-store tests** pass.
Check, Clippy with warnings denied, formatting and release example build pass.
The adaptive suite's 13 tests cover all four winners, bounds before exchange,
missing/scoped bases, revocation, forged replies, no reoffer, exact reconstruction,
repeat hits and evicted-repeat misses, and actual HTTP body accounting.

Focused optimized coverage for `selection.rs` is 95/100 lines, 7/7 functions and
166/184 regions. Branch/MC/DC coverage is not collected; statement coverage is not
separately reported. This focused run does not measure full-crate coverage. Full
existing-client coverage is deliberately not claimed from the adaptive-only tests.

The initial RED compiler failure, test-only authority type error, and three test
failures caused by assuming sparse always wins are retained with their correction.
The latter exposed a useful case: a 128 KiB fixture with 32 midpoint bytes replaced
by zero sends **569 prefix body bytes versus 600 sparse bytes**. The selector was
correct; the test assumption was repaired and the case retained as a regression.
The harness's test-first development and corrected range-check Clippy finding are
also recorded. No measured operation was repeated to replace an unfavorable result.

The architecture, code/security review, command receipts, independent numeric
verification and pipeline audit are linked from the [evidence index](evidence/2026-10-29-codec-selection/README.md).
Raw measurement JSONL SHA-256:
`f2e3f74af52b89da1b8197a588f0d2c5ebc20595df632a43098000b6e641387d`.

This milestone does not qualify a 64 MiB client, sender peak memory, production
deployment, CUI/FIPS, energy, direct/shaped WAN, or MVM. The eventual MVM adapter
still targets the canonical encrypted vsock mailbox, including a small-MVM
orchestrator's identity, lifecycle, isolation, backpressure and resource limits.
