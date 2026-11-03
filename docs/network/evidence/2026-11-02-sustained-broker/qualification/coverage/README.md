# Focused Linux source-line coverage

**PASS for the adapter product-source scope:** **4,483 / 4,828 executable
production lines (92.8542%)**. Added adapter production lines relative to
`95fc3c45` measure **167 / 168 (99.4048%)**.

Compiled source is pinned to `d94020382a1c838a0794af14989d687985c26b69`, on Wick
Linux x86-64 with Rust 1.97.1 and its matching LLVM 22.1.6 tools. The
[source hashes](source-files.sha256), [complete owned crate input hashes](build-inputs.sha256),
[instrumented artifact inventories](adapter-artifacts.json),
[live target inventory](live-artifacts.json) and
[test executable hashes](test-binaries.sha256) identify the collection.

The adapter metric includes **all twelve compiled files under its `src/`**,
including existing production paths and the conformance feature. Only the three
explicit trailing inline test blocks listed in [test-cutoffs.tsv](test-cutoffs.tsv)
are excluded from the production metric. Integration tests and examples remain
outside that source denominator. No compiled adapter production module is omitted.

| Adapter production module | Covered / executable | Added production covered / executable |
|---|---:|---:|
| Broker | 28 / 28 | No added lines |
| Receipt tracker/carrier | 781 / 815 | 18 / 19 |
| Mailbox service and diagnostics | 1,916 / 2,086 | 149 / 149 |
| All compiled adapter modules | 4,483 / 4,828 | 167 / 168 |

The separate live scope includes **only `config.rs` and `campaign.rs`**, exercised
by the four focused `broker_objects` tests. Their added production coverage is
**3 / 4 (75%)**; the updated invalid-arm error-string line is not covered in this
focused collection. Their combined existing production coverage is 581 / 770.
This is explicitly **not whole-live coverage**, and the four-line result is not
rounded up or substituted for the adapter gate.

Only source-line coverage was collected. Branch, function, MC/DC, native guest
and WAN coverage are not inferred from these figures.

## Executed collection

`collect.sh` builds adapter all-feature/all-target test harnesses and the one
focused live test target with `--no-run --message-format=json`. Both owned
release packages were cleaned first; all inventoried executables report
`fresh: false`. The wrapper instruments only owned adapter/live compilation
units, including integration callers of generics. Upstream MVM and other
dependencies remain uninstrumented.

All **22 exact emitted test executables** were then run once, separately:
**104 adapter tests plus four live tests passed**, with zero failures, ignored
or filtered tests. Two zero-test example harnesses add nothing to these counts.
[per-binary-tests.txt](per-binary-tests.txt) retains the actual summaries.
Instrumented runtimes are not transfer-performance measurements.

Each executable received its own profile directory, sparse merge and LLVM
export against that executable only. All **22 exports had zero warnings or
errors**, avoiding incompatible function-hash mixtures. The line union retains
the maximum hit count per file/line across isolated exports.

[source.lcov](source.lcov), [lines.tsv](lines.tsv) and [summary.json](summary.json)
retain the resulting line proof. [derive.sh](derive.sh) documents the explicit
scope and aggregation. Added executable lines are the intersection with
[changed-lines.tsv](changed-lines.tsv), derived from new-side added source lines
in the pinned Git diff. Comments, removed lines and nonexecutable additions do
not enter that denominator. The retained AWK summarizer independently reproduced
every line-table value from the compact LCOV; sorted tables compared identical.

After export, only the two owned release packages were cleaned, reclaiming
87.4 MiB, and the normal `service_sustained` release example rebuilt successfully
with wrappers and all Rustflags explicitly unset. Existing private normal native
binaries were preserved. No instrumented binary is used for transfer timing.

The owned raw profiles/unfiltered exports were removed only after the compact
packet and its checksums were verified. No payload dataset, VM disk, duplicate
build tree or Mac build was created by this coverage task.

```sh
shasum -a 256 -c SHA256SUMS
awk -f summarize.awk changed-lines.tsv test-cutoffs.tsv source.lcov
```
