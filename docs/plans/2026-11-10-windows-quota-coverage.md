# Native Windows quota coverage gate

The portable mailbox workflow collects native Windows coverage after the two
uninstrumented storage primitive tests and their strict Clippy check, before
the larger endpoint suites. The same two library tests run serially in a separate
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
LLVM JSON raw counts must agree with the independently parsed raw LCOV line map.
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

Implementation status: static inspection only for this environment correction;
no new native coverage collection has run. The earlier plain/instrumented primitive passes and
historical Linux coverage do not satisfy the production gate. A native refusal
or a result below 80% remains a failed gate requiring investigation and focused
tests; the threshold must not silently change.

References: [Rust instrumentation](https://doc.rust-lang.org/rustc/instrument-coverage.html)
and [LLVM coverage exports](https://llvm.org/docs/CommandGuide/llvm-cov.html).
