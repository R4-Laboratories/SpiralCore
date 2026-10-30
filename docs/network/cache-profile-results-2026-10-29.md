# Direct cache-scan attribution: SC-OPT-004

The two quota scans account for most of the measured CPU in the populated-cache
sparse-update operation. With 768 background publications, the normal-budget sparse
case used a midpoint median of **30.174 ms operation CPU**, of which **25.4735 ms**
was inside the instrumented `disk_usage` scans. The median of the two per-observation
scan shares was **84.3912%**. With no background publications, operation CPU was
**4.489 ms**, and scans accounted for **0.1215 ms**, or **2.7083%**.

This is direct attribution within an instrumented local Rust process. It does not
measure an optimization, a WAN transfer, operating-system syscall counts or energy.
It also does not separate path/formatting/allocation work from directory enumeration
and metadata work inside the scan. No production candidate is accepted by this result.

## Recorded execution

- Registration: [cache profile and larger WAN plan](../plans/2026-10-29-cache-profile-and-large-wan.md),
  initially committed as `867ad3338b6e8c0ba708259cb8103fd4dac0f5d0`.
- Baseline snapshot: `f0966ebedcb8994f0985b54e9a2b961eeb8be914`; hardware SHA enabled;
  native `aarch64-apple-darwin`, Rust 1.97.1. The production checkout was not modified.
- Diagnostic source fingerprint:
  `93e6cfb9ee5a88d5eb724d6c290296d22c444c6a9a50b3b0ce7bbff5dbba5539`.
  The snapshot comparison found 49 original files unchanged and two changed files:
  the endpoint instrumentation/tests and the module export. `scan_profile.rs` is new.
- Frozen probe SHA-256:
  `ee2e57c73d5ecd28d692877ff17edf89ff678b15eda66b41edabd7b303f06e6d`.
- Two blocks ran from 17:19:05 to 17:20:18 UTC on October 29, 2026. Both exited 0.
  The duration includes cloning, synchronization and other untimed setup; it is not
  the sum of operation timings. No retry, filtering or additional profiling followed.
- **32/32 exact observations**, **368 frames**, **64 scans**, and **17,472 metadata
  API call sites**. Every target and base was compared byte-for-byte after closing
  and reopening the endpoint. There were no failed observations or counter mismatches.
- Modeled request-plus-reply frame bytes remained **1,051,138 per raw operation** and
  **536 per sparse operation**, totaling **16,826,784** across the diagnostic. These
  are application frame sizes; the in-process probe did not transmit network packets.

The [evidence packet](evidence/2026-10-29-cache-profile/README.md) contains both raw
blocks, complete Rust analysis, source patches and diagnostic source, structured test
outcomes/commands/failure details with original output hashes, effective features,
compiler/build metadata, template hashes and checksums.
The premeasurement qualification receipt intentionally retains its original
`performance_measurement: NOT_RUN`; the later raw execution receipts establish the run.

## Measurement boundaries

The unchanged fixture is a 1 MiB per-byte xorshift64 sequence with seed42, with 32
bytes XORed at offset 524288. This is the earlier receiver-probe generator, not the
WAN generator that emits eight bytes per state. Base ObjectId is
`c7fceaf53550e0ab1393604c2bc88eabd267676766ae90ad52459ece381822c0`; target ObjectId is
`227bbf10e522740c837d03da96838331a00fe3dac2ca77c095ba1044467b0ebf`.

Each observation starts with a fresh private synchronized clone of the same closed
template. A cache contains N unrelated 32-byte publications, distributed two per
background namespace, plus one 1 MiB base in the active namespace. Before the transfer,
there are N+1 publications and N/2+1 namespaces. Afterward there are N+2 publications.
The two scans both observe the N+1 pre-publication inventory. Source template hashes
were identical before and after all observations.

Instrumentation wraps only the original `disk_usage` body in the isolated snapshot.
The filesystem calls, predicate order, checked sums, publication limit and original
success/error return remain unchanged. Counters are stack-local, then merged into a
constant-size thread-local aggregate. No per-entry allocation, logging, clocks or
extra filesystem calls were added. The probe resets and reads that aggregate outside
the preexisting handler timer. Each successful Begin and Finish must contain one scan;
Data must contain none. Counts and returned totals must match the exact inventory.

Scan CPU/wall clocks include counter overhead inside the loop; handler clocks also
include the record operation and timer overhead. Whole-operation clocks include frame
or recipe preparation, handler execution, profile reset/read, reply/counter validation
and request-buffer teardown through Published. These are nested clocks: **do not add
scan, handler and operation CPU together**. Fixture generation, clone/sync, endpoint
open and close/reopen verification are separate. Instrumentation can change code
generation; its overhead is not subtracted, and these timings cannot establish a
production speed change against an earlier uninstrumented run.

Both graph-retention limits are zero. The normal node budget is 262144 and the tiny
budget is 1. The tiny budget also affects admission/reservation and manifest readiness;
it is not a pure warming toggle. Raw normal-budget publications retain an optional
manifest; sparse and tiny-budget targets do not. This readiness difference is preserved.

## All registered cells

Times are milliseconds. Each time is the midpoint median of **two** observations.
The percentage is the midpoint median of the two per-observation scan/operation CPU
fractions; it is not computed by dividing the displayed medians. All raw values and
per-frame samples are retained in the JSON evidence. With two samples these are
descriptive observations, not estimates of a stable population distribution.

| Background objects | Mode / node budget | Operation CPU | Scan CPU | Scan share of CPU | Operation elapsed | Scan elapsed |
|---:|---|---:|---:|---:|---:|---:|
| 0 | Raw /262144 | 24.5065 | 0.1465 | 0.598% | 32.0945 | 0.1439 |
| 0 | Raw /1 | 2.9775 | 0.1160 | 3.899% | 10.7664 | 0.1146 |
| 0 | Sparse /262144 | 4.4890 | 0.1215 | 2.708% | 12.2166 | 0.1200 |
| 0 | Sparse /1 | 4.4660 | 0.1090 | 2.441% | 11.8748 | 0.1076 |
| 64 | Raw /262144 | 32.8530 | 2.5515 | 7.802% | 38.9219 | 2.6494 |
| 64 | Raw /1 | 6.4555 | 2.9255 | 44.227% | 29.5066 | 2.9890 |
| 64 | Sparse /262144 | 10.8840 | 3.7540 | 35.203% | 20.1525 | 3.9002 |
| 64 | Sparse /1 | 5.8810 | 2.0000 | 33.990% | 15.5034 | 1.9981 |
| 256 | Raw /262144 | 36.6770 | 9.5825 | 26.030% | 49.0100 | 9.5819 |
| 256 | Raw /1 | 16.9795 | 12.8380 | 75.674% | 31.6303 | 12.8874 |
| 256 | Sparse /262144 | 14.7825 | 9.6460 | 65.271% | 22.4884 | 9.6658 |
| 256 | Sparse /1 | 14.4165 | 9.4400 | 65.478% | 21.7546 | 9.5258 |
| 768 | Raw /262144 | 70.6400 | 27.4740 | 42.302% | 85.5754 | 28.3723 |
| 768 | Raw /1 | 31.1040 | 27.7420 | 89.239% | 48.1935 | 30.4987 |
| 768 | Sparse /262144 | 30.1740 | 25.4735 | 84.391% | 42.9990 | 27.2545 |
| 768 | Sparse /1 | 27.7850 | 23.4075 | 84.246% | 36.2237 | 23.4273 |

At N768, normal-budget sparse Begin scan CPU was 12.533/13.046 ms and Finish scan CPU
was 10.570/14.798 ms. Total scan CPU was 23.103/27.844 ms; operation CPU was 27.489/32.859 ms.
The per-observation scan CPU shares were 84.0445% and 84.7378%. Scans consumed 90.8483%
of handler CPU and 63.6904% of operation elapsed by the same midpoint-of-fractions
definition. These different fractions should not be read as pure network or pure
I/O wait fractions: this probe has no network, and process CPU/wall clocks overlap.

Variation is material. N768 raw normal-budget operation CPU was 42.434 ms in block 0
and 98.846 ms in block 1; both remain in the report. N64 normal-budget sparse also varied
substantially. Host one-minute load averages were 7.20 and 6.88 at block starts; OS page
caches, scheduling and thermal state were uncontrolled. These limitations forbid
selecting a production optimization from a favorable two-point time comparison.

## Qualification and disposition

Counter qualification first failed at runtime with recording disabled: three tests
covered successful counts, a publication-limit failure with partial counts, and a
filesystem failure with no yielded entries. Probe integration likewise first failed
at runtime when profiles were not attached to frame receipts. Both RED outcomes,
assertion details, commands, source-manifest references and original output hashes are
retained in the structured qualification receipt. Final qualification passed 8 probe
tests, 2 Rust-analysis tests and 8 snapshot
unit tests, plus formatting and Clippy with warnings denied. Aggregation tests cover
overflow marking and thread isolation. Counter/body/reopen checks are part of every
measured successful observation. Two lint-only setup repairs are retained: use the
standard midpoint parity predicate and move the test module after production items.

Independent source review approved the preserved scan semantics and clock boundaries;
independent driver review verified the actual frozen hashes and bounded two-block
execution before timing. An independent Rust-only post-run review checked all 256
metric records across all 16 cells, including their raw vectors, medians, minima and
maxima, plus source/executable provenance, order, IDs, inventories, readiness, frames,
stages, counters and containing clocks. All checks passed; the independent report
SHA-256 is `416536a6be5eb4e5ec54f19eb3028be87324ab832f9056037da23bb2f0ce2523`.
No production source change follows automatically from this run.

After review, a bounded Rust cleanup helper preserved 20 structured diagnostic records
and removed 20 owned local diagnostic files plus 9 identical packet copies. Raw scientific
JSONL, sources, analyses, template/build/binary identities and independent reviews remain.
The original premeasurement qualification and its hashes were not rewritten: removed
log paths in those historical bindings are intentionally no longer live. Their old
checksum/qualification commands require a newly generated qualification before rerun.
The [cleanup method](evidence/2026-10-29-cache-profile/cleanup-method.md) records the
read-only selection check, exact allowlist, commands and before/after identity evidence.

The result supports focusing subsequent work on quota scans for populated caches.
It does **not** yet identify whether path construction, namespace formatting, directory
enumeration or metadata retrieval is the dominant component. Object count and namespace
count also grew together, so their effects are not separated. Exact syscall and allocation
counts remain unmeasured. The [cache scan decision](cache-scan-decision.md) therefore
remains conditional: preserve fresh observation of all namespaces at Begin and Finish;
do not silently replace it with a stale usage counter, watcher cache or reused snapshot.

Any bounded production candidate needs its own source pin, static-filesystem equivalence
and external-mutation tests, and an uninstrumented paired acceptance run with hardware
SHA on in both arms. The failed SC-OPT-002 namespace-formatting hoist remains rejected.
Larger WAN qualification is a separate experiment and is not evidence that an unmeasured
scan candidate works. CPU reductions and scan attribution do not establish joules saved.
