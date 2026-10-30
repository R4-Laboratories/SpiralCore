# SC-OPT-004 evidence packet

This packet records exactly two blocks and 32 local diagnostic observations against a
snapshot of `f0966ebedcb8994f0985b54e9a2b961eeb8be914`, hardware SHA enabled. The
[report](../../cache-profile-results-2026-10-29.md) explains all results and boundaries.
Production source was not changed. There is no old/new performance comparison here.

| Artifact | Contents |
|---|---|
| `raw/block0-diagnostic.jsonl`, `raw/block1-diagnostic.jsonl` | Complete metadata and observations, every frame/counter, exact reopen status, inventories and stage clocks |
| `raw/execution.jsonl` | Two start/end event pairs with exit 0, no retry |
| `raw/templates-*.sha256`, `templates-established.sha256` | Identical complete source-template manifests before/after |
| `profile-analysis.json` | Rust-validated 32 observations, all 16 cells, both raw values per metric, midpoint medians, minima/maxima, fractions and input hashes |
| `source/profile-analysis.rs` | Complete native Rust analyzer and qualification binder; no Python |
| `source/probe-lib.rs`, `source/probe-main.rs` | Reused receiver probe plus diagnostic reset/read and counter validation |
| `source/diagnostic-endpoint.rs`, `source/diagnostic-transfer-lib.rs`, `source/scan_profile.rs` | Complete changed/new snapshot runtime source and tests |
| `patches/` | Exact endpoint/module/probe differences from retained originals |
| `snapshot-origin.tsv`, `snapshot-established.sha256` | Original Git blob comparisons and all 52 snapshot file SHA-256 values |
| `build/` | Compiler/native target, commands, effective feature tree/metadata, build tools/environment and executable/source/helper hashes |
| `qualification.json`, `qualification/` | Unchanged historical premeasurement binding, source manifests and exit receipts; removed log hashes/paths remain historical |
| `structured-qualification.json` | Test names/counts, exact commands, RED assertion/lint failure details, original diagnostic hashes and source-manifest references, preserved before log deletion |
| `diagnostic-cleanup.json`, `cleanup-method.md`, `source/cleanup-receipts.rs` | Exact bounded cleanup selection, before/after custody, Rust source and commands |
| `independent-profile-review.rs`, `.json`, `-method.md` | Independent Rust-only review of all 256 author metric records, raw structural/accounting checks, provenance and template equality |
| `schedule.json` | Fixed two-block matrix, order, fixture IDs and template fingerprint |
| `SHA256SUMS`, `checksums.sha256` | Identical current relative-checksum manifests for all other retained packet files |

The frozen premeasurement qualification says `NOT_RUN` and is intentionally unchanged;
later execution and raw observation files establish that both blocks ran. Structured
qualification retains each test outcome, summaries, exact commands, failure details,
original output hashes and available source-manifest identities. The two expected
runtime RED cases failed with missing profiles, then passed with the reviewed
recording/wiring. Lint failures are setup findings, not failed transfers.

After independent review, the owned `.log`/`.stdout`/`.stderr` diagnostics were removed
only after durable structured receipts were written. Original premeasurement bindings
remain byte-for-byte unchanged: their removed log paths and hashes are historical.
Old test-log checksum commands and `--qualify` cannot be rerun against this cleaned
directory without a newly generated qualification. Do not edit the old binding to make
it appear live. The current packet checksum manifests cover the cleaned retained files.

The full owned local experiment is
`/Users/casey.allard/spiralcore/local/cache-profile-20261029`. It retains immutable native
executables, complete snapshot, prior source used for RED tests and the exact path layout
used by the build/run drivers. Large binaries and Cargo build outputs are not copied
into Git. The original templates remain under
`/Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/experiment/templates`;
only fresh clones under the new experiment's own working directory were changed.

To reconstruct the source, extract both crate directories from the pinned commit into
`snapshot/`; overlay the three diagnostic transfer files from `source/`, place the probe
library/main/analysis into `src/`, and use the retained Cargo manifest/lock. File mappings
are explicit above; preserved manifests retain original build paths. The source helpers
for build/run use the already reviewed Bash orchestration and native Rust executables.
Running the frozen driver again is not part of this campaign; a new measurement would
need a new output directory, reviewed source/paths and explicit registration.

CPU and elapsed times are nanoseconds in raw JSON. Scan clocks are subsets of handler
clocks; handler clocks are subsets of whole-operation clocks. Added counter/timer work
is retained. Counts identify Rust API call sites, not OS syscalls. Frame-byte totals are
modeled in-process protocol bytes, not captured traffic. No energy was measured.
