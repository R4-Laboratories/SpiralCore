# SC-OPT-003: hardware SHA-256 result

**All three registered CPU gates passed.** All 256 observations recovered the exact target and base after reopening. Hardware was faster in all 128 paired operation-CPU comparisons, covering 16 configurations with eight pairs each.

Both binaries use candidate `80449939bd92348593cc88555cf7fde0847ceffd`, the same expanded lock, measured Rust, and recorded compiler/build settings. Runtime host state was uncontrolled. The sole active configuration difference is the default-off hardware-sha256 feature. Plan `f6a0144` and qualification packet `be2dfc3` were committed before measurement. Complete graph/manifest encodings matched across 36 qualification fixtures.

Ratios are geometric means of matched software/hardware observations. CPU covers the local operation from frame/recipe preparation through validated Published and request-buffer destruction. Handler CPU is a subset. Marginal wall medians are separate from paired ratios.

| Required cell | Software CPU median | Hardware CPU median | Paired CPU ratio | Required ratio | CPU wins | Software wall median | Hardware wall median | Paired wall ratio |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| N0 raw / nodes 262144 | 51.964 ms | 25.303 ms | 2.104× | 1.10× | 8/8 | 60.494667 ms | 33.378479 ms | 1.880× |
| N0 sparse / nodes 262144 | 16.360 ms | 4.402 ms | 3.669× | 1.50× | 8/8 | 24.649729 ms | 12.738000 ms | 1.866× |
| N768 sparse / nodes 262144 | 38.283 ms | 27.571 ms | 1.413× | 1.20× | 8/8 | 46.535708 ms | 36.285812 ms | 1.369× |

These paired CPU ratios correspond descriptively to 52.47%, 72.74% and 29.21% less CPU, respectively. They describe this local operation on this host, not WAN time or energy.

| Background objects | Arm / max nodes | Software CPU median ms | Hardware CPU median ms | Paired CPU ratio | CPU wins | Paired wall ratio | Wall wins |
|---:|---|---:|---:|---:|---:|---:|---:|
| 0 | raw / 262144 | 51.964 | 25.303 | 2.104× | 8/8 | 1.880× | 8/8 |
| 0 | sparse / 262144 | 16.360 | 4.402 | 3.669× | 8/8 | 1.866× | 8/8 |
| 0 | raw / 1 | 11.991 | 2.950 | 4.055× | 8/8 | 1.772× | 8/8 |
| 0 | sparse / 1 | 16.416 | 4.787 | 3.511× | 8/8 | 1.994× | 8/8 |
| 64 | raw / 262144 | 81.399 | 46.881 | 1.722× | 8/8 | 1.581× | 8/8 |
| 64 | sparse / 262144 | 35.107 | 13.560 | 2.507× | 8/8 | 1.939× | 8/8 |
| 64 | raw / 1 | 27.081 | 10.649 | 3.059× | 8/8 | 2.157× | 8/8 |
| 64 | sparse / 1 | 35.380 | 13.779 | 2.597× | 8/8 | 1.948× | 8/8 |
| 256 | raw / 262144 | 74.124 | 44.245 | 1.723× | 8/8 | 1.530× | 7/8 |
| 256 | sparse / 262144 | 35.831 | 19.044 | 1.886× | 8/8 | 1.692× | 8/8 |
| 256 | raw / 1 | 29.623 | 17.023 | 1.765× | 8/8 | 1.332× | 7/8 |
| 256 | sparse / 1 | 35.276 | 19.271 | 2.012× | 8/8 | 1.711× | 8/8 |
| 768 | raw / 262144 | 75.377 | 45.867 | 1.688× | 8/8 | 1.577× | 8/8 |
| 768 | sparse / 262144 | 38.283 | 27.571 | 1.413× | 8/8 | 1.369× | 8/8 |
| 768 | raw / 1 | 34.116 | 26.142 | 1.258× | 8/8 | 1.217× | 8/8 |
| 768 | sparse / 1 | 38.039 | 27.017 | 1.405× | 8/8 | 1.310× | 8/8 |

Wall time improved in 126/128 paired comparisons. Two slower observations remain: N256 raw normal block 4 (ratio 0.980723×) and N256 raw one-node block 2 (0.715277×). Every configuration's wall geometric mean favors hardware. Small components are mixed: N0 raw normal Begin CPU ratio is 0.937253×; N0 sparse normal Data CPU is 0.972932×. All stage and setup regressions remain in the machine report.

The stage measurements support the intended mechanism. For N0 sparse normal, median frame-preparation CPU falls from 8.4395 to 2.2755 ms, Begin from 3.8815 to 0.9255 ms, and Finish from 3.9595 to 1.208 ms. These paths contain full-object identity/verification hashes. The cryptographic function, integrity checks, identities and publication semantics remain unchanged.

Large-cache overhead remains: hardware sparse-normal CPU median is 4.4015 ms at N0 and 27.571 ms at N768. At N768, median Begin/Finish costs are still 13.077/12.033 ms. Source retains full quota scans; stages also include hashing, reads and durable publication, so this is not a per-function scan profile.

Modeled bidirectional application-frame bytes are unchanged: **1,051,138 bytes per raw observation and 536 per sparse observation**, totaling 67,307,136 per backend across the campaign. These are protocol-frame sizes in direct Endpoint calls, not captured traffic.

The 16 processes ran from 16:16:09Z to 16:25:05Z on 2026-10-29 (536 seconds including cloning/verification). All exited zero. All 2,944 frame samples, identities, inventories, readiness, stage sums and provenance pins passed validation. The 1,100-entry template manifest is unchanged. No retries, filtering, extra trials or post-hoc normalization occurred.

`paired-analysis.json` retains all 21 metric summaries per cell, eight paired values, midpoint medians and nearest-rank p95. With eight samples, p95 equals the observed maximum. Host one-minute loads ranged from 4.54 to 8.62; scheduling, OS caches and thermal/power state were uncontrolled.

Qualification scope is native Apple ARM; the feature remains default-off. This experiment measures no WAN throughput/latency, packets, power or joules. Do not multiply these ratios by earlier WAN ratios. Remote x86 hosts may already use hardware SHA by default. Unsupported targets and cryptographic-module validation remain outside this result.

## Fixture and timing interpretation

Each case updates a 1 MiB deterministic object by overwriting 32 bytes at offset 524288.
N counts unrelated 32-byte cached objects; N0 still has the verified 1 MiB base. The
generator is xorshift64-low-byte-per-step-v1, seed42, distinct from the earlier WAN
fixture generator. Every observation starts from a newly synchronized private copy
of a closed template. Template creation/cloning, endpoint open, synthetic fixture
generation and post-close/reopen byte comparison are outside the primary operation
and recorded separately. Receiver verification and durable publication are inside it.

Both graph-retention settings are zero. Raw with the normal node budget constructs
a target DAG, while sparse skips it. The one-node budget changes admission and target
readiness as well as graph construction; it is not a recommended default or a pure
warming toggle. The reported backend ratios compare identical modes and policies.

CPU and elapsed time overlap and must not be added. The measured operation executes
sender-like preparation and the Endpoint in one local process, without a network or
TLS connection. These results neither isolate network-only time nor establish another
WAN speedup. The [separate real-WAN report](efficiency-results-2026-10-29.md) remains
tied to its original source, fixtures, transport and 30-trial paired comparisons.
Earlier [16 MiB WAN checks](wan-results-2026-10-29.md#cache-seeding-and-larger-objects)
passed raw/zstd and whole-object-reuse tests with one trial per arm. They do not
qualify these new sparse/hardware paths at 16 MiB or the 64 MiB protocol ceiling.

## Available implementation and qualification

The new production change is the opt-in feature in `spiralcore-transfer`, forwarding
to the existing pinned `sha2 0.10.9` backend. No production Rust algorithm, wire encoding,
object identity, integrity check or publication rule changed. The lock adds only
`sha2-asm 0.6.4` and its dependency edge. On the tested Apple M1, feature metadata and
symbol-scoped disassembly confirm software versus SHA2-instruction implementations.

Each configuration passed 125 transfer tests, one compile-fail doctest, fmt and strict
clippy; the imported store passed 71 tests and its save/restart example with each
backend. The measured helper passed seven tests in each configuration. A separate
exporter and independent parser checked 36 fixtures and 113,748 node records per
configuration, including complete encoded-file equality and exact reconstruction.
After the campaign, the native hardware release binaries built successfully and passed
17 CLI/transport integration tests. Those tests use local transport and are not a new
WAN performance result. No new line-coverage figure is claimed.

On the qualified native Apple ARM host, the opt-in build command is:

```sh
cargo build --release --locked --target aarch64-apple-darwin \
  --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --features hardware-sha256 --bins
```

The local release binaries are under
`/Users/casey.allard/spiralcore/target/aarch64-apple-darwin/release`; their exact hashes
are in [release validation](evidence/2026-10-29-hardware-sha/release-validation.json).
The backend remains opt-in for portability. Upstream assembly does not support Windows;
unsupported-CPU fallback and other platforms require separate qualification.

## Review record and next optimization boundary

The [evidence packet](evidence/2026-10-29-hardware-sha/README.md) preserves every raw
observation, execution order, template hashes, source/build identities, fixed schedule,
qualification, independent review, all metric arrays and checksum manifests. It also
records the Cargo feature/lock and Bash3 setup failures, resolved before timing.
The [registered plan](../plans/2026-10-29-hardware-sha256.md) and
[architecture decision](hardware-sha256-decision.md) specify the original gates.

The observed growth from 4.4015 to 27.571 ms of hardware sparse CPU as unrelated cache
entries increase makes cache-admission/publication work the next profiling target.
Full filesystem quota scans remain in the source, but the current stage timers do not
attribute all of that difference to a specific function. A persistent quota index or
other scan reduction needs its own correctness contract, crash/external-change tests
and preregistered measurement. The failed smaller quota-scan refactor remains recorded
separately as SC-OPT-002; its result is not revised by this successful experiment.
