# Default-feature simultaneous and alternating direction control

This development control investigates the Windows deadline failures holding
PR #260. The retained ARM failure at source `515b87bb` records a sender waiting
for a kind-3 reply while both scopes remain valid. Its receiver thread also
handles incoming storage work synchronously, so delayed local reply routing is
a hypothesis; the log does not prove that the remote endpoint withheld a reply.

At executed source `13ff238d`, the ordered-stream fixture accepts the test-only environment selector
`SPIRALCORE_ORDERED_LAYOUT=simultaneous` or `alternating`. Absence retains the
simultaneous workload. Unknown values refuse before endpoint setup. Production
library code, encoding, authority and wire meanings are unchanged.

Both modes retain 4,096 distinct 16-KiB blocks in each direction (64 MiB each),
the same exact byte comparisons, two-second operation deadlines, 240-second
workload deadline, 270-second enrollment and retention bounds. Alternating mode
uses two capacity-one integer-token channels to allow one direction's block at
a time. Exact token ordinals are checked. No extra worker is spawned, token
receive uses the original workload deadline, and token publication refuses a
full or disconnected channel instead of waiting. Dropping a failed writer drops
its original channel ends so its peer cannot wait forever. This control state
belongs to the fixture, outside the product's SmallV1 accounting.

The manually requested `duplex_control` timing workflow builds one default-
feature executable per native Windows runner, executes both modes with separate
fresh endpoint stores, records both outcomes even when one fails, and checks the
same executable digest before and after. The existing instrumented timing jobs
are skipped for this explicit control request. The paired order is determined
by run identity and matrix row and recorded in the results. Source, tree, lock,
OS/architecture, raw result lines and checksums are retained in compact artifacts.

This is a diagnostic comparison, not an optimization or acceptance claim. If
only simultaneous traffic fails, that discriminates concurrency dependence but
does not by itself prove a specific storage stage or reply-routing cause. No
root cause, speedup, WAN result, native MVM backend or supported capacity is
claimed before execution and analysis. PR #260 remains held.

## Executed Linux control

Both modes pass at source `13ff238d2ff013bd33012a8140003f058c48f20b` with the
same executable SHA-256 `b662c83ed00962c3cc6035895813c5b76b6e05be741df0c4fe734d7da9b7c476`.
Strict release Clippy passes. Each mode verifies 67,108,864 exact bytes in each
direction, 64 retained publications per endpoint and 4,032 evictions per endpoint.

| Linux layout | Lifecycle wall | Process CPU | Reported TLS A sent / received |
|---|---:|---:|---:|
| Simultaneous | 45.826980 s | 4.821633 s | 5,395,050 / 5,395,015 bytes |
| Alternating | 58.385668 s | 5.165151 s | 5,395,050 / 5,395,015 bytes |

These are one paired diagnostic run with both endpoints on Wick. The original
fixture measures lifecycle through cleanup and process CPU across both endpoint
threads; these are not send-only times or WAN measurements. Payloads are highly
compressible structured fixtures. No matched raw/zstd comparison or compression
advantage is established by this control. The alternating layout is a test
control, not an implementation optimization.

[Structured results and raw-log hashes](linux-results.json). Raw data and the
shared build target stay on Wick; only this compact report is copied locally.

## Verified native Windows results

[Paired run 43612000640](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43612000640)
completed. All three ZIPs match GitHub's archive digests, and all six manifest-
listed members in each ZIP verify. Source and tree match `13ff238d`; both modes
on each runner retained the same executable digest and length. The original
mode order remains recorded in the results.

| Native process role | Layout | Outcome | Wall | Process CPU |
|---|---|---|---:|---:|
| Server 2022 | Simultaneous | Exact 64 MiB each direction | 204.151819 s | 99.281250 s |
| Server 2022 | Alternating | Exact 64 MiB each direction | 225.826709 s | 94.203125 s |
| Server 2025 | Alternating | Exact 64 MiB each direction | 206.119197 s | 109.328125 s |
| Server 2025 | Simultaneous | Exact 64 MiB each direction | 225.098870 s | 122.968750 s |
| Windows ARM | Simultaneous | Exact 64 MiB each direction | 209.055928 s | 116.484375 s |
| Windows ARM | Alternating | Original workload deadline; incomplete | 241.988984 s snapshot | 92.406250 s snapshot |

Successful rows report complete fixture lifecycle time. The failed ARM row is
a failure snapshot, not completion time. Its write/read progress was
`[3990, 3989, 3989, 3990]`, short of the required 4,096 in every counter. It
therefore fails the original gate. No full-object fidelity or throughput result
is claimed for that row.

Successful rows report the original 5,395,050 / 5,395,015 TLS bytes, excluding
lower-layer headers. These are process-local TCP/TLS fixtures with highly
compressible input, not MVM, WAN or matched raw/zstd acceptance. Memory RSS was
unavailable on these Windows runs. The two instrumented job groups were
intentionally skipped; they are not counted as passing execution.

This result does **not** reproduce the earlier simultaneous sender-reply timeout
or establish synchronous reply routing as its cause. Alternating did not provide
a consistent lifecycle improvement and failed the ARM workload deadline. No
production change or deadline increase is adopted from this control. The older
failures remain retained and PR #260 stays held.

[Verified structured outcomes](windows-results.json) · retained original ZIPs:
[Server 2022](windows/server2022.zip), [Server 2025](windows/server2025.zip),
[Windows ARM](windows/windows11-arm.zip). Their combined archive size is only
11,291 bytes; no executable, VM disk, key or raw application corpus is included.

The executed test and manual workflow are published at
[source `13ff238d`](https://github.com/R4-Laboratories/SpiralCore/commit/13ff238d2ff013bd33012a8140003f058c48f20b),
based on held source `515b87bb`. The report qualifies those source-scoped
experiments, not the default product dependency graph.
