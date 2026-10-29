# Finer cache-scan attribution — completed diagnostic

All 72 registered observations reconstructed the exact target and passed the recorded
inventory, readiness, frame, clock and counter checks. The diagnostic reinforces that
namespace layout affects scan cost, but the fine clocks exceeded the registered overhead
ceiling in the primary `spread768/sparse` cell. **No production candidate is accepted or
selected from that cell's expression ranking.** There were no retries, replacements,
excluded rows or additional performance observations.

This is an isolated diagnostic of source `d03fe1630bba787a28cc3fa3ae201c4a1a75388b`
with hardware SHA enabled. Registration is `9c9650dc156cd2511ba74932537f691a68fdada4`.
The production checkout was not modified by this experiment. Frozen `DESIGN.md`,
`README.md` and qualification receipts preserve their preparation-time status; this
result records the subsequent execution without rewriting those premeasurement inputs.

The immutable measured executable SHA256 is
`26942f5f371df05e6d63dac16c639273eaac5313e7e5ca2f8412444200006c37`;
the diagnostic source fingerprint is
`640c5de7312ec20d32bad8fe7a9e7e4980c7580e5bdbcf10598a6a7d7fe4798d`.
The Rust analysis is `profile-analysis.json`, SHA256
`1c3428ea7a0fb6ab9bc21c7caa08e08a5386ac2bc24fb1ff7356d2f52e0c64c6`.
Its source is `src/bin/profile-analysis.rs`. The nine raw files, sequence, process exits
and per-process host observations remain in `measured/`.

## Qualification and complete accounting

The three intended missing-counter tests failed at runtime before recording was fixed.
Two analyzer safety tests failed before bounded reads and checked arithmetic were added.
Final qualification passed 15 helper/analysis tests and 136 snapshot tests, with one
existing release-only 16 MiB local HTTP test explicitly ignored in the default command.
Both Clippy checks passed. Architecture and transport reviewers approved the source.
Actual invalid-UTF8 filesystem entry creation is **NOT_RUN**: this host rejected the
filename with OS92. Pure invalid-UTF8 path classification was tested separately.

The four templates were populated through Endpoint, exact-checked and synced before
measurement. All 1,612 template file hashes matched before and after. Each observation
used a private synced clone with the target absent, and checked returned bodies and
exact recovered bytes after reopening. Graph retention was disabled in both arms;
normal node limits remained fixed. Raw publication retains graph readiness while sparse
publication does not. This is an explicit endpoint behavior difference, not a comparison
of equivalent graph-preparation work.

The fixed run lasted from 2026-10-29 19:18:18Z to 19:21:51Z across nine processes;
the surrounding manifest checks are outside that interval. All process exits were zero.
Validated totals: **72 exact observations, 828 handled frames, 144 quota scans,
57,744 metadata API calls and 37,860,264 modeled application-frame bytes**.
Per-operation modeled bytes were unchanged: raw 1,051,138; sparse 536. These are local
modeled frame sizes, not captured network bytes. No WAN or energy measurement occurred.

## Counter-only observations

Each value below is the median of all three observations. Scan fraction is the median
of per-observation scan/operation CPU, not a ratio of the two displayed medians.
Counter-only still includes fixed counters and coarse clocks; it is not an uninstrumented
production benchmark. Operation CPU includes local frame/recipe preparation, Endpoint
handling, reply validation and buffer teardown. Scan CPU is a nested subset.

| Layout / arm | Operation CPU ms | Operation elapsed ms | Scan CPU ms | Scan fraction of operation CPU |
|---|---:|---:|---:|---:|
| empty / raw | 32.633 | 41.391458 | 0.316 | 0.7731% |
| empty / sparse | 4.775 | 17.296625 | 0.219 | 4.5864% |
| compact64 / raw | 47.582 | 54.434916 | 1.186 | 3.1251% |
| compact64 / sparse | 7.903 | 18.859916 | 1.561 | 13.5857% |
| compact768 / raw | 35.681 | 42.567208 | 7.968 | 22.3312% |
| compact768 / sparse | 14.047 | 21.672291 | 8.381 | 58.6392% |
| spread768 / raw | 75.921 | 94.471917 | 26.781 | 41.0845% |
| spread768 / sparse | 28.230 | 35.945958 | 23.854 | 84.4988% |

The compact and spread 768 layouts contain the same number of background objects and
publication bytes, placed in one versus 384 background namespaces. The higher spread
cost is consistent with more directory enumeration work. This is a descriptive layout
comparison with three observations, not a measured improvement from changing production.
Raw measurements are visibly variable: compact64 CPU was 23.493, 47.582 and 49.418 ms;
spread768 raw CPU was 43.369, 80.663 and 75.921 ms. These remain in the result.

## Instrumentation controls and expression evidence

Ratios below are medians of block-paired operation-CPU ratios. They are not ratios of
unpaired medians. Values below one are retained as noise/code-generation/control effects,
not interpreted as clocks accelerating the underlying work.

| Layout / arm | Fine / counter CPU | Empty-clock / counter CPU |
|---|---:|---:|
| empty / raw | 0.998899 | 0.685441 |
| empty / sparse | 1.063061 | 0.871832 |
| compact64 / raw | 1.043315 | 0.937392 |
| compact64 / sparse | 1.351386 | 0.753351 |
| compact768 / raw | 1.326048 | 1.330764 |
| compact768 / sparse | 1.137752 | 1.345269 |
| spread768 / raw | 1.285965 | 0.777150 |
| spread768 / sparse | 1.533316 | 1.533971 |

In the primary spread768/sparse cell, operation CPU was 28.230, 23.022 and 38.410 ms
for counters; 39.068, 35.300 and 62.525 ms for fine clocks. All three paired fine
inflations exceeded the 1.25 ceiling (1.383918, 1.533316, 1.627831). Consequently none
of the six primary expression categories passed the complete registered descriptive
screen. No empty-span subtraction was performed.

| Primary expression group | Fine CPU median ms | Empty-span CPU median ms |
|---|---:|---:|
| Enumeration | 21.509 | 1.371 |
| Entry type | 0.804 | 0.812 |
| Canonical namespace name | 0.359 | 0.241 |
| Extension/path | 0.812 | 0.522 |
| Metadata length | 4.234 | 0.524 |
| Namespace formatting/comparison | 2.623 | 0.594 |

Enumeration includes ReadDir creation/advance and the namespace `d.path()` argument;
it is not an OS-syscall measurement. Extension/path includes original temporary path
work, but allocations are not counted. Entry-type and canonical-name spans are not
resolved clearly above their empty controls. The coarse unspanned residual includes
bookkeeping and iterator/entry destruction, and is not labeled pure I/O.

The secondary compact768/sparse cell has two descriptive leads: metadata and namespace
comparison passed its screen, with fine medians 3.798 and 2.498 ms, respectively. Its
paired fine inflation median was 1.137752, but one of three ratios was 1.958137 and is
retained. These secondary leads do not replace the registered primary, establish
attainable savings, or overturn SC-OPT-002's rejected formatting-hoist experiment.
Across all eight layout/arm cells, these are the only two passing category screens.

## Consequence for the next change

The broad cache-scan bottleneck remains supported. Fine-grained attribution in the
primary cell is instrumentation-sensitive; there is no adequate result here for a
path-allocation-only production optimization. Namespace layout and enumeration deserve
further investigation, and metadata cost is a secondary lead. A prospective backend
that obtains directory entries and metadata together would require explicit platform,
error-equivalence and authority review before implementation, not a silent cached quota
catalog or fewer filesystem observations.

Any production candidate must retain fresh Begin/Finish observations and external
growth/addition/deletion behavior, then meet separately registered uninstrumented gates
on exact source/binaries. No candidate, extra timing or adoption was performed here.
The present finding is useful even though its primary fine-attribution screen failed.

The complete analysis retains all 24 cells and three samples per metric, every control
ratio and all screen booleans. Independent numerical review is recorded separately when
complete; this author report is not itself that review.
