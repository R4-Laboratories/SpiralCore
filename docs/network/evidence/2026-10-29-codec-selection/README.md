# SC-OPT-006 evidence index

Start with the [result and limitations](../../codec-selection-results-2026-10-29.md),
[design](../../codec-selection-decision.md) and
[measurement contract](../../codec-selection-measurement.md).

- [registration.json](registration.json): pinned runtime `850272f`, executable,
  platform, eight strata, 30 paired trials, ordering, timing and no-replacement rule.
  Registration was committed as `791ec79` before smoke and registered timing.
- [source-files.sha256](source-files.sha256): exact tracked crate inputs, relative
  to the repository root, at the runtime revision.
- [measurements.jsonl](measurements.jsonl): all 3,043 records: registration,
  1,800 exact operations, 1,200 successful cache-cleanup records, paired acceptance,
  40 complete summaries and completion. All observations, including regressions,
  are retained. The native harness is [here](../../../../crates/spiralcore-transfer/examples/codec_selection.rs).
- [smoke.jsonl](smoke.jsonl): separate 180-operation qualification, excluded from
  the registered 30-trial sample.
- [qualification.json](qualification.json): fresh 185 release tests (including
  large ignored-by-default cases), 1 doctest, 71 imported-store tests, build and format.
  Associated command output and exit files are retained as formal evidence.
- [adaptive-test-disposition.json](adaptive-test-disposition.json): originalRED,
  authority-type test error, disproved sparse-always-wins test assumption and
  corrected 13-test qualification. Nothing rewrites the historical failures.
- [coverage/receipt.json](coverage/receipt.json): separate instrumented focused
  coverage with source/tool identities, full JSON and its own verified checksums.
- [review.json](review.json): independently executed Check/Clippy and scoped
  code/Rust/security source review. Final audit and post-audit review receipts
  supplement this before publication.
- [independent-review/](independent-review/): independent recomputation of every
  summary, paired minimum, fixture/seed identity, counts/order and cleanup, with
  negative checker qualification and custody.
- `audit.json` and `final-review.json`: final workflow/evidence gate and source,
  report and security confirmation. These are separate from performance results.
- `cleanup.json`: only this task's transient output and synthetic-cache cleanup.

`SHA256SUMS` binds every other regular file in this packet and is checked before
publication. Historical WAN reports, raw archives and captures are unchanged.
The local review-tool roster applies only to the current Codex worktree, not other
labs. No paid host, new relay or MVM runtime was provisioned for this milestone.

These files intentionally preserve scientific and qualification records. Routine
empty command-output files outside the packet are removed after their sizes and
hashes are recorded. Neither nested CPU counters nor local elapsed time imply
network-only transfer time, whole-path traffic or energy consumption.
