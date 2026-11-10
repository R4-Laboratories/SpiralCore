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
copied test executable, listing profiles, executed-test profiles, merged profile,
build diagnostics and LLVM exports. No binary or raw profile is placed in the
checkout or uploaded by the ordinary evidence artifact. The directory remains
on the disposable runner through collection; runner disposal removes it. No
separate raw artifact upload is introduced by this change.

The collector checks Cargo/LLVM/test exits, exactly two listed and passed tests,
no ignored or filtered tests, a nonempty bounded profile set, exactly one mapped
production `src/windows/quota.rs`, and a positive executable-line denominator.
The whole new production file is the gate: covered executable lines must reach
at least 80%, using integer comparison. There are no source-line exclusions.
LLVM JSON summary counts must agree with the independently parsed LCOV line map.
Branch coverage is reported as unavailable if LLVM supplies no branch mappings;
line coverage is not presented as branch coverage.

`portable-evidence/windows-quota-coverage-*` retains only source hashes, test
enumeration/output, the compact summary and uncovered executable-line numbers,
with a 32 KiB per-file and combined 256 KiB limit. An oversized text is replaced
by a short refusal marker; its full content stays only in runner temporary
storage, and the step fails. The summary pins the original and copied binary,
Rust/LLVM versions and LLVM executable hashes, raw/merged profile and export
hashes, source commit, native role and collection flags. All tracked storage
crate inputs are hashed before/after, including the lockfile, and the binary and
LLVM tool hashes must remain unchanged. The existing final evidence checksum
step covers these compact outputs, including a failed below-threshold receipt.

Implementation status: workflow/source inspection only; no coverage build or
native execution has been run for this collector. Pinned LLVM-tool availability
on all three hosted Windows roles is unverified until execution. The previous
plain primitive passes and historical Linux f26 coverage do not satisfy this
gate. A native refusal or a result below 80% remains a failed gate requiring
investigation and focused tests; the threshold must not silently change.

References: [Rust instrumentation](https://doc.rust-lang.org/rustc/instrument-coverage.html)
and [LLVM coverage exports](https://llvm.org/docs/CommandGuide/llvm-cov.html).
