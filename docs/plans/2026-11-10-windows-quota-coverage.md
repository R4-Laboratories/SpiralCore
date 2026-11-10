# Native Windows quota coverage gate

The portable mailbox workflow collects native Windows coverage after the
uninstrumented primitive tests, canonical portable component tests and private
storage/endpoint suites. Thus a collector defect cannot skip those earlier
positive witnesses. Coverage remains a required failing step, with no
`continue-on-error`. The same two library tests run serially in a separate
instrumented build. Their timing is not quota performance evidence. The original
uninstrumented output remains the performance record.

Each Windows row installs `llvm-tools-preview` for the exact existing
`1.97.1-<native-target>` toolchain. It locates `llvm-cov` and `llvm-profdata` in
that toolchain's sysroot and checks their LLVM versions against `rustc -vV`.
Missing tools, native host mismatch or incompatible versions fail the step.
No cross-compilation or Linux mapping receives native Windows credit.

A fresh GUID directory beneath `RUNNER_TEMP` contains the isolated Cargo target,
copied test executable, executed-test profiles, merged profile,
build diagnostics and LLVM exports. No binary or raw profile is placed in the
checkout or uploaded by the ordinary evidence artifact. The directory remains
on the disposable runner through collection; runner disposal removes it. No
separate raw artifact upload is introduced by this change.

Compile and execution use one absolute profile filename pattern. The binary
runs with its current directory under the owned runner-temporary directory,
so a runtime default-file fallback also stays outside the checkout. Recursive
owned-directory profile discovery records all observed parent directories,
counts and bytes plus at most 16 full file-detail rows after enumeration and
after execution. Only the enumerated owned
`.profraw` files are removed between enumeration and execution; the collector
requires zero remaining profiles before running the two tests. It merges only
fresh post-test profiles and never silently accepts enumeration coverage.

The collector checks Cargo/LLVM/test exits, exactly two listed and passed tests,
no ignored or filtered tests, a nonempty bounded profile set, exactly one mapped
production `src/windows/quota.rs`, and a positive executable-line denominator.
The new production file is the gate: covered executable lines must reach
at least 80%, using integer comparison. Preserve whole-file raw LLVM counts,
then exclude only the exact `#[cfg(test)]` blocks at lines 200–205, 207–208 and
303–304 from the production line map. The exact LF Git source SHA-256 is pinned to
`f7a1433d083cb9b16a2e4316cba797655b5f257b5e4ae970bfcf9e971ffb074c`;
the observed raw checkout hash is retained separately. Windows checkout CRLF
normalizes to LF solely for this Git content comparison (the observed f9 CRLF
hash is `178dea30ebf931e5675316f154d0fb5fd2d1ae93cd3631d409e872b12a101df9`);
every excluded snippet is checked against those exact source lines. Source or
snippet drift refuses collection until the exclusions receive review. No entire
production function or unmapped gap receives an exclusion. Retain both raw and
production denominators, the complete compact line-hit map and exclusions.
LLVM JSON raw line counts must agree with the selected LCOV `LF`/`LH` totals.
The production gate uses distinct physical `DA` source lines, removing only the
approved test statements. JSON summary totals and distinct `DA` lines are
different LLVM metrics and are retained separately, rather than forced equal.
Both formats, the complete bounded line-hit map, normalized JSON/LCOV paths and
all comparison counts are saved before any numeric/path consistency guard.
Exactly one quota-file record must match in each export; malformed or out-of-file
`DA` lines refuse collection. The 80% physical-production-line formula remains
unchanged.
Branch coverage is reported as unavailable if LLVM supplies no branch mappings;
line coverage is not presented as branch coverage.

`portable-evidence/windows-quota-coverage-*` retains only source hashes, test
enumeration/output, profile-discovery records, the compact summary and uncovered
production executable-line numbers,
with a 32 KiB per-file and combined 256 KiB limit. An oversized text is replaced
by a short refusal marker; its full content stays only in runner temporary
storage, and the step fails. The summary pins the original and copied binary,
Rust/LLVM versions and LLVM executable hashes, raw/merged profile and export
hashes, source commit, native role and collection flags. All tracked storage
crate inputs are hashed before/after, including the lockfile, and the binary and
LLVM tool hashes must remain unchanged. The existing final evidence checksum
step covers these compact outputs, including a failed below-threshold receipt.

The initial f9 collector installed matching LLVM tools and passed its two
instrumented tests on the native Windows roles, then refused its profile set.
The [Server 2025 job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42915575247/job/130177088484)
retains the refusal. Its compact artifact recorded neither actual profile
locations nor the failing count/size, so it cannot establish a filename-location
cause. The successor uses consistent paths and captures that missing evidence;
this is a collector correction awaiting execution, not a demonstrated diagnosis.

The `5fc29bd4` native run then recorded zero profiles at enumeration,
before tests and after both tests passed. This rules out an oversized profile
set in that run. It still did not retain actual compiler invocations.

A source-level collector defect is now documented: PowerShell 7.5+ treats
`[Environment]::SetEnvironmentVariable(name, $null, 'Process')` as an empty
variable rather than removing it. Cargo gives a present
`CARGO_ENCODED_RUSTFLAGS` precedence over `RUSTFLAGS`, including an empty value.
The collector used that .NET call to clear encoded flags, so its intended
instrumentation could be suppressed. The observed Server 2025 image documents
PowerShell 7.6.9. This establishes the defective source mechanism; proving that
it caused the observed binary's missing instrumentation still requires the
actual native compiler invocation, which the failed packet lacks.
[PowerShell behavior](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_environment_variables?view=powershell-7.5#use-the-systemenvironment-methods),
[Cargo precedence](https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags),
[observed runner image](https://github.com/actions/runner-images/blob/win25-vs2026/20261103.298/images/windows/Windows2025-VS2026-Readme.md#powershell-tools).

The minimal correction explicitly removes the encoded environment variable and
asserts its absence before building. The compact prebuild receipt records only
flag presence, the collector's known flag string and PowerShell/.NET versions;
it dumps no inherited environment. Cargo `-vv` remains under owned temporary
storage, while the actual storage `--test` compiler invocation is retained in
compact evidence and must contain `-C instrument-coverage`. Missing invocations
or missing instrumentation fail before test execution. Finally, absent saved
variables are removed and existing values, including empty strings, restored;
each restoration checks the resulting presence/value without printing it.

No calibration executable is added in this patch. If actual compiler flags
contain instrumentation and profiles still remain absent, the next bounded
investigation is a minimal Rust calibration and binary coverage-section probe
under the same pinned compiler/environment. The known ARM64 MSVC
[issue #150123](https://github.com/rust-lang/rust/issues/150123) concerns malformed
profiles after generation; it does not explain zero profiles on both x64 roles.
All native production coverage gates remain unresolved until actual collection.

At `18fd806c`, actual native compiler invocations include coverage instrumentation
and fresh post-test profiles were produced after the enumeration profile was
removed. The [Server 2025 job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42919503756/job/130190580623)
retains one 42,248-byte enumeration profile, zero profiles before execution and
one fresh 42,248-byte profile after two passed tests. Its collector then rejected
JSON-summary versus physical-line counts without retaining either export.
Those exact Windows counts remain unavailable; no quota coverage pass follows.

LLVM 22.1.6 source explains the distinct metrics: file summaries aggregate
function/instantiation-group summaries, whereas detailed LCOV `DA` rows iterate
file-level mapped lines. LCOV `LF`/`LH` reuse the function-summary values.
[Summary construction](https://github.com/llvm/llvm-project/blob/llvmorg-22.1.6/llvm/tools/llvm-cov/CoverageReport.cpp#L445),
[physical line export](https://github.com/llvm/llvm-project/blob/llvmorg-22.1.6/llvm/tools/llvm-cov/CoverageExporterLcov.cpp#L87),
[summary export](https://github.com/llvm/llvm-project/blob/llvmorg-22.1.6/llvm/tools/llvm-cov/CoverageExporterLcov.cpp#L240).
A collector-only read of the retained f9 Linux exports reproduced the difference:
timing JSON/`LF`/`LH` is 138/138 versus distinct `DA` 132/132, and duplex is
651/603 versus 635/591. This reads existing exports; no product build or test
was rerun. The successor compares matching summary metrics and retains native
physical counts for the unchanged production gate. Actual Windows collection
still must execute and satisfy every guard.

At `661c74e7`, all seven native roles completed their portable functional
checks. Server 2022 and Server 2025 each passed quota production coverage at
216/228 mapped physical lines (94.7368%). Windows ARM64 produced profiles and
passed its functional/storage tests, but its
[profile merge step refused](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42922116307/job/130199422213).
That collector did not upload actual merge stdout/stderr, so the error cannot
yet be attributed to Rust issue #150123 or called a demonstrated tool limitation.
ARM64 functional results remain valid within their scope; its coverage gate
remains failed and receives no x64 substitution or skipped-pass classification.

The diagnostic successor captures merge exit status, stdout and stderr before
the existing refusal guard. Each stream retains up to 24 KiB of tool output; longer output
retains its first/last 12 KiB with an explicit truncation marker. The merge
receipt pins full stream sizes/hashes, raw profile names/sizes/hashes and the
exact tool/version, source, target and arguments. Full logs/profiles remain only
in owned runner temporary storage. Existing 32 KiB per-file, 256 KiB combined
compact-evidence caps and the 80% production gate remain enforced.

First inspect the genuine error from the next native attempt. If it reports a
malformed profile, separate a writer/runtime defect from merge-tool mismatch
using the already pinned compiler/tool versions and a bounded calibration only
when needed. Retain a profile/PE pair externally only if diagnosis requires it;
do not put those raw artifacts in Git or download them to the local HDD. If the
pinned toolchain is reproducibly unsupported, document that exact coverage
limitation while keeping ARM64's functional/end-to-end role and open coverage
gate distinct. No unsupported-tool conclusion is established by this patch.

Implementation status: static inspection and retained-export collector check
only for this summary-semantics correction; no new native collection has run.
The earlier plain/instrumented primitive passes and
historical Linux coverage do not satisfy the production gate. A native refusal
or a result below 80% remains a failed gate requiring investigation and focused
tests; the threshold must not silently change.

References: [Rust instrumentation](https://doc.rust-lang.org/rustc/instrument-coverage.html)
and [LLVM coverage exports](https://llvm.org/docs/CommandGuide/llvm-cov.html).
