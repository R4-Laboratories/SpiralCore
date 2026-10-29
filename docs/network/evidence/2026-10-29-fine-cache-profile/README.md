# SC-OPT-005 evidence packet

The [result report](../../fine-cache-profile-results-2026-10-29.md) describes all
findings and limits. This packet preserves 72 local observations from three blocks
against an instrumented snapshot of `d03fe1630bba787a28cc3fa3ae201c4a1a75388b`.
No production optimization was selected and no new WAN result is established.

| Path | Contents |
|---|---|
| `raw/block*.jsonl` | Nine complete metadata/observation files, including every frame, scan, nested clock and exact reopen status |
| `raw/execution.jsonl`, `raw/execution-order.txt` | Fixed order, start/end timestamps and all nine zero exits |
| `raw/templates-*.sha256`, `templates-established.sha256`, `setup.jsonl` | Template creation and unchanged before/after identity |
| `profile-analysis.json`, `RESULTS.md` | Author's complete 24-cell distributions, paired controls, 48 screens and interpretation |
| `independent-review.json`, `independent-review/` | Independent Rust recomputation, checks, source, reproduction instructions and method hashes; final result is `review-final.json` |
| `source/` | Complete changed diagnostic endpoint/module/counter source and Rust probe/analyzer |
| `build/` | Frozen helper source, feature/compiler/tool/platform/source and executable identities; native executables remain local |
| `qualification.json`, `qualification/` | Original premeasurement qualification, source histories, setup findings and exit receipts |
| `structured-qualification.json`, `diagnostic-cleanup.json`, `cleanup-method.md` | Test names/counts/failures, exact available commands, original diagnostic hashes and scoped retirement of 37 text files |
| `DESIGN.md`, `schedule.json`, `local-reproduction.md`, Cargo/build/run files | Original registration and reproduction procedure |
| `SHA256SUMS` | Current relative checksums for all other packet files |

The original qualification intentionally says NOT_RUN: it was frozen before timing.
The later execution and raw records establish that the run occurred. Original
manifests retain historical paths and hashes of retired diagnostic text; do not
rewrite them to make old log checks appear live. Scientific JSONL, source, analysis,
template data and executable bindings were preserved.

The full local directory is
`/Users/casey.allard/spiralcore/local/broad-wan-20261029/cache-profile`. It retains
the native executables, complete source snapshot and templates. Reconstruct the
snapshot by extracting both crate directories from the pinned Git revision, then
overlay `source/diagnostic-endpoint.rs`, `diagnostic-transfer-lib.rs` and
`scan_profile.rs` at their corresponding transfer-crate paths. Place `probe-lib.rs`,
`probe-main.rs` and `profile-analysis.rs` at `src/lib.rs`, `src/main.rs` and
`src/bin/profile-analysis.rs`; use the retained Cargo manifest/lock and build files.
Original source/build manifests and paths remain authoritative for this experiment.

Times are nanoseconds in raw records. Nested scan, handler and operation clocks
must not be added together. Counts identify Rust API call sites, not actual system
calls. Empty spans are controls, not an exact subtractable overhead correction.

The native Bash 3.2 standalone-conditional finding discovered after this run is
recorded in the result report. Actual positive identities and data were independently
verified; rejection-path claims require a repaired successor driver. Do not modify
the frozen driver and present it as the measured one.
