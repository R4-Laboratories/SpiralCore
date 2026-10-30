# SC-WAN-005: warm-object reuse still wins against matched whole-object zstd

**The existing verified sparse path remains faster and transfers far fewer bytes
on the tested warm-object workload.** Thirty paired real-WAN trials per size
completed at 1 MiB and 16 MiB. All 120 base seeds and 120 changed objects were
reconstructed and compared byte-for-byte after stopping and reopening the
receivers. No transfer failed, was retried, or was removed from the result.

This resolves the scope question raised by SC-OPT-014: its schema/codec-context
comparison sent distinct new objects without offering a previous object as a
base. That comparison did not test the sparse warm-object path. This recheck
changes no production encoder or decoder; it measures that existing path against
the newer, equally tuned whole-object zstd control. It does not establish that
every warm workload or automatic selection policy is efficient.

The [registered design](../specs/2026-10-30-warm-object-recheck-design.md),
[measurement contract](evidence/2026-10-30-warm-object-recheck/measurement-contract.md),
[raw evidence](evidence/2026-10-30-warm-object-recheck/README.md) and
[complete summary](evidence/2026-10-30-warm-object-recheck/wan/summary.json)
retain the dose, identities, distributions and limitations.

The WAN source is `605ee6f1b2f5012c497e65fd56a0499f2de94253`, with compiled
source identical to local qualification commit
`5a4f141ac64aa09533a6561d6ae72b3f04856e95`. The first-attempt
[GitHub run 41140288919](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41140288919)
transferred from 22:48:41 to 22:52:18 UTC on October 30, 2026. A hosted Linux
sender connected to the Mac receiver through Cloudflare Quick Tunnels. Three
receiver shards each admitted 80 seed/update transfers. Trial order is a frozen
seeded shuffle, with 15 Sparse-first and 15 Whole-first pairs per size.

Each base contains exactly 1,048,576 or 16,777,216 pseudorandom bytes; every target
changes exactly 32 midpoint bytes. Both arms actually pay for the same Raw base
in separate fresh authorized namespaces. The target is absent, and exact-repeat
probing is disabled. Optional DAG construction is disabled for both arms. Whole
attempts prepared level-3, window-25, long-distance-matching, single-thread zstd
with content size and checksum, then chooses Raw when compression does not save
bytes. All 60 Whole updates selected Raw; the compression attempt remains timed.
Sparse uses the existing COPY/LITERAL delta API without trying schema, runtime
or other losing codecs.

All following transfer values count both HTTP-body directions. CPU subtotal is
sender-process CPU plus the disjoint receiver-handler CPU. Completion starts
before encoding/comparison and hashing and ends after the durable verified ACK.
Context preparation is separately measured and added once to the paid lifecycle.
Medians of separate columns need not add to the median of their sum.

| Object and method | Update body bytes | Sender CPU ms | Receiver-handler CPU ms | CPU subtotal ms | Exchange elapsed ms | Full completion ms | Completion p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1 MiB Sparse | 600 | 1.8950 | 7.4495 | 9.3680 | 141.0010 | 142.2855 | 177.964 |
| 1 MiB Whole/Raw | 1,051,338 | 4.0515 | 7.5275 | 11.5390 | 244.4210 | 246.5945 | 618.575 |
| 16 MiB Sparse | 600 | 20.7850 | 33.8115 | 54.5955 | 181.6815 | 201.7710 | 322.725 |
| 16 MiB Whole/Raw | 16,814,026 | 48.2305 | 38.6940 | 85.9755 | 1,592.4320 | 1,626.6170 | 2,171.258 |

The update byte reduction is **99.9429% at 1 MiB** and **99.9964% at 16 MiB**.
The ratios of the displayed completion medians are **1.73× and 8.06×** in favor
of Sparse. Those are ratios of cohort medians, not medians of paired speedups.
The 1 MiB median comparison remains below the proposed 2× time target. This
single relay route does not satisfy the full WAN acceptance matrix.

Actual paired Sparse/Whole ratios below use each trial's own matched control;
less than one favors Sparse. CPU and wall outcomes remain separate.

| Size and phase | Median paired sender CPU ratio | Median paired CPU subtotal ratio | CPU wins | Median paired completion ratio | Completion wins |
|---|---:|---:|---:|---:|---:|
| 1 MiB update | 0.468054 | 0.846206 | 22/30 | 0.545163 | 30/30 |
| 16 MiB update | 0.431981 | 0.600888 | 30/30 | 0.127982 | 29/30 |
| 1 MiB seed + update + setup | 0.747752 | 0.952790 | 21/30 | 0.816878 | 25/30 |
| 16 MiB seed + update + setup | 0.698051 | 0.783580 | 29/30 | 0.610258 | 29/30 |

Sender CPU was lower in all 30 update pairs at each size. Receiver CPU was lower
in only 14/30 small and 22/30 large update pairs; shared-host and route variation
remain visible. One 16 MiB update, trial 0, took **2,372.567 ms Sparse versus
2,140.860 ms Whole/Raw**, despite using 600 body bytes and less measured CPU.
Its Sparse exchange took 2,352.525 ms while its receiver handler took 43.767 ms
elapsed. The measurements locate most waiting outside that handler but do not
identify its cause. The outlier remains included in every aggregate.

Shared state is not free. The following medians charge the actual first Raw
seed, one changed update, and the arm's context setup. They are measured
lifecycle sums, not projections of repeated free warm transfers.

| Size and method | Paid lifecycle body bytes | CPU subtotal ms | Full lifecycle ms |
|---|---:|---:|---:|
| 1 MiB Sparse | 1,051,938 | 21.4795 | 480.4305 |
| 1 MiB Whole/Raw | 2,102,676 | 23.4760 | 734.4965 |
| 16 MiB Sparse | 16,814,626 | 141.4330 | 2,042.0765 |
| 16 MiB Whole/Raw | 33,628,052 | 180.9890 | 3,319.8470 |

Paid lifecycle byte savings are **49.9715% and 49.9982%**. Identical first seeds
show route variability: their 16 MiB median completion times were 1,839.495 ms
in the Sparse arm and 1,693.774 ms in the Whole arm. No seed cost was discarded
or replaced with a modeled common value. Across all thirty 16 MiB lifecycles,
Sparse used 504,438,780 body bytes, 4.332371 s measured CPU and 66.288888 s summed
completion, versus 1,008,841,560 bytes, 5.487575 s CPU and 102.387588 s for Whole.
These are actual cohort totals, not a concurrent throughput measurement.

The preceding small local qualification used separate sender/receiver processes
and three pairs per size. All 24 objects reopened exactly. Local 1 MiB update
medians were 4.640 ms CPU / 21.327 ms completion for Sparse versus 6.160 / 23.543
for Whole; at 16 MiB they were 42.488 / 78.081 versus 68.448 / 103.116. Those
small local samples qualify accounting and correctness; the WAN sample above
provides the thirty-pair route result.

The Mac qualification passed 324 tests with zero failures and two preexisting
ignored tests. A subsequent behavior-preserving Clippy predicate collapse passed
all six affected tests, all-target Clippy, and the release build. The hosted
sender independently ran its all-target release tests, Clippy and build. The
original RED cases and initial Clippy failure are retained. There is no production
source change and no new whole-repository or branch coverage claim. Procedural,
independent code/Rust and security/infra reviews precede the launch; the final
review independently checks raw results and report arithmetic.

All 36 downloaded artifact members, 83 source hashes and three cross-platform
plans match. Exact stopped-store readback covers all 240 full byte strings.
The hosted sender executable is
`b269e46ad8dc057e6f8c05d43ed4e2f0308013a35c53fbaf7f90d827271f31f6`;
the Mac receiver is
`572667bda7eba40c269e92c58e7bc6747e96a026c5cc92d06532328e4d26620c`.
The detailed [custody record](evidence/2026-10-30-warm-object-recheck/wan/custody.json)
links binaries, artifact retention and verification hashes.

HTTP-body counts exclude headers, TLS, TCP/IP, retransmissions and additional
relay hops. Exchange elapsed includes remote compute and waiting: it is not pure
wire time, and CPU must not be subtracted from wall time to invent one. The CPU
subtotal excludes receiver socket/logging and relay work. Fixture generation,
process startup, receipt hashes and final readback are outside transfer timers.
No energy measurement, shaped 80 ms/10 Mbps test, arbitrary-data compression,
standard-delta superiority, MVM integration or security qualification is claimed.
This proves the existing sparse mechanism on a defined shared-base workload;
it does not attribute that result to cellular automata or ternary arithmetic.

The implementation implication is to preserve this direct verified-base path
when improving selection, and avoid paying for schema or other encoders without
an expected byte benefit. SC-OPT-014's separate byte selector does not currently
include a prior-object candidate. The new measurement provides a matched baseline
for that future integration; it does not silently replace existing automatic
selection or claim that integration is already delivered.
