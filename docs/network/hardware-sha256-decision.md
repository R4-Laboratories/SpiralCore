# SC-OPT-003: optional hardware SHA-256 backend

Status: proposed opt-in backend and registered local experiment; no full-operation result yet.
Decision baseline: `7975b9d`, after the SC-OPT-002 candidate was reverted by `cb376b8`.
The failed SC-OPT-002 experiment and its candidate remain preserved in the
[receiver-cost report](receiver-cost-results-2026-10-29.md) and
[optimization ledger](optimization-ledger.md).

## Decision and allowed change

Add one default-off feature to `spiralcore-transfer`:

```toml
[features]
hardware-sha256 = ["sha2/asm"]
```

Keep the existing `sha2 = "0.10.9"` declaration and resolve exactly `sha2 0.10.9`
through the reviewed lockfiles. The allowed lockfile change is the new
`sha2` dependency edge to `sha2-asm` and the `sha2-asm 0.6.4` package entry, which
uses the already resolved `cc` dependency. No existing package version or checksum
may change as part of this experiment. If Cargo proposes another dependency change,
review it before building the comparison. Do not run a broad dependency update.

The expected registry package checksums are:

| Package | Registry checksum |
|---|---|
| sha2 0.10.9 | `a7507d819769d01a365ab707794a4084392c824f54a7a6a7862f8c3d0892b283` |
| sha2-asm 0.6.4 | `b845214d6175804686b2bd482bcffe96651bb2d1200742b712003504a2dac1ab` |

The initial execution and qualification scope is native `aarch64-apple-darwin`.
Do not enable the feature by default. Do not introduce bespoke cryptographic code,
global `+sha2` target flags, `target-cpu=native`, a new digest, or a protocol version.
The production implementation difference is limited to Cargo feature selection and
its necessary lock entries. Test, harness, evidence and documentation changes must
be identified separately and must not alter the measured production Rust code.

## Source-checked backend behavior and portability

The following statements were checked against the installed Cargo registry sources
for the exact versions named here; paths are relative to each package's source root:

| Source | Checked behavior |
|---|---|
| `sha2 0.10.9`, `src/sha256.rs:3-29` | Default aarch64 selects software. The `asm` feature enables the aarch64 dispatcher. `force-soft` and `force-soft-compact` take precedence. x86/x86-64 already have a hardware dispatcher; `asm` changes their software fallback. |
| `sha2 0.10.9`, `src/sha256/aarch64.rs:11-24` | The dispatcher selects SHA2 instructions when the capability check succeeds and otherwise calls the software implementation. The selected aarch64 path is implemented inside `sha2`, using its SHA256 instruction wrappers; this is not evidence that the separately compiled `sha2-asm` object is the active aarch64 compression path. |
| `sha2 0.10.9`, `Cargo.toml`, features and target dependencies | `asm` enables optional `sha2-asm`; `asm-aarch64` is an alias for `asm`, not an aarch64-only portability restriction. |
| `cpufeatures 0.2.17`, `src/aarch64.rs:94-120` | Apple ARM64's SHA2 capability check is true by platform guarantee. On this M1, the runtime hardware-absent branch is not exercised. |
| `sha2-asm 0.6.4`, `src/lib.rs:12-17` and `build.rs:3-28` | The package supports x86, x86-64 and aarch64 assembly builds and explicitly rejects Windows targets. It introduces a native assembly build through `cc`, including Apple-specific aarch64 source selection. |

Retain these package identities, the resolved feature trees and the relevant source
references in the experiment packet. The prior package lock and feature receipts
are preserved with the [earlier A/B metadata](evidence/2026-10-29-compute/ab-metadata.json),
[software feature tree](evidence/2026-10-29-compute/ab-software-features.txt) and
[assembly feature tree](evidence/2026-10-29-compute/ab-hardware-features.txt).

The proposed feature is an opt-in assembly/backend feature, not an enforced
aarch64-only Cargo feature. Enabling it on another architecture can change that
architecture's backend or fail to build. In particular, the Windows assembly
feature is unsupported. Default-off behavior remains unchanged; no new platform
support claim follows. Linux aarch64, x86, Windows, hardware-absent runtime fallback
and the declared minimum Rust version require their own executed qualification
before any corresponding support claim.

## Invariants and correctness gates

All object, node and manifest identities, domain separators, byte encodings, graph
construction rules and wire frames must remain byte-for-byte identical. Complete
length/digest validation, namespace and authorization checks, resource reservations,
cache trust, staging, atomic rename, file/directory synchronization and durable
publication acknowledgment remain unchanged. Hardware instructions are a different
implementation of the same SHA-256 operation; they do not permit skipping a check.

Before timing either variant:

1. Run fixed independent SHA-256 known-answer vectors under both feature settings,
   including empty, short, padding-boundary and multiblock input. Retain the existing
   independent object/node/manifest vectors in
   [`tests/profile.rs`](../../crates/spiralcore-transfer/tests/profile.rs).
2. Require identical object IDs, encoded manifests, encoded-node fingerprints and
   exact reconstructed bytes across the retained boundary, trailing-zero, structured
   and randomized fixtures. Include the actual receiver-probe base and target.
3. Run the existing hostile-input, sparse reconstruction, scoped-cache, authorization,
   quota, corruption and durable-publication checks under both feature settings.
4. A separately identified forced-software correctness build may test upstream
   `force-soft` precedence. Do not describe it as an executed runtime test on a CPU
   lacking SHA2. Preserve unrun platform/fallback qualifications explicitly.

No FIPS validation, CUI deployment approval, cryptographic-module certification or
new security level follows from this feature, the known-answer tests, or hardware
instructions. It does not replace the separately scoped validated-cryptography path.

## Same-source paired experiment

Reuse the reviewed SC-OPT-002 receiver probe and its exact fixture and timing
contracts. Preserve SC-OPT-002 artifacts unchanged and create a separate SC-OPT-003
packet. Both software and hardware executables must be built from the same
post-revert production source, including the same new feature declaration, and the
same expanded lockfile. The intentional difference is whether `hardware-sha256`
is selected. Do not compare an older pre-declaration build against a newly resolved
candidate and attribute all differences to the backend.

Use locked builds with the same compiler, native target, profile, environment and
helper source. Explicitly pin Cargo's target directory and native target, preserve
both executables, and retain the build arguments and file hashes. Record
`cargo tree -e features -i sha2` for both resolved builds. Cargo feature unification
affects all users of the same `sha2` version in the executable, including the probe
and imported object-store dependency. Confirm that the software variant has no
`asm` feature and the hardware variant has it, with neither variant selecting
`force-soft` or `force-soft-compact` during performance runs.

The fixture remains `xorshift64-low-byte-per-step-v1`, seed 42, a 1 MiB base with a
32-byte XOR-0xa5 edit at offset 524288. It is distinct from the earlier WAN fixture.
Keep all four raw/sparse by node-budget arms, populations 0/64/256/768, fresh private
synced template clones, target absence, actual inventory checks and reopened exact
verification. Both node-budget profiles keep graph retention disabled as in the
reviewed probe. The one-node profile changes graph admission/reservation and target
readiness; it is not a pure warming toggle or a recommended general policy.

Use eight paired blocks with four software-then-hardware and four hardware-then-
software invocations. Preserve the existing internal population rotation and
four-arm counterbalancing. Freeze the updated schedule and both executables before
timing; do not build, test or run another benchmark concurrently. Retain all 16 block
outputs and all 256 observations, including failures and regressions. No outlier
deletion, selective population reporting, or retry of a slow successful observation.

The primary metric remains `local_operation_total.cpu_ns`: process CPU from before
recipe/frame preparation through validated publication and request-buffer teardown.
It includes sender-like preparation and bounded instrumentation, so it is not
receiver-only CPU. Handler stage CPU is a subset and must never be added again.
Record elapsed time, Begin/Data/Finish costs, setup/clone/open/close/reopen costs,
modeled request/reply frame bytes, manifest readiness and actual cache inventories.
The packet must retain exact timing exclusions and OS-cache/background-load limits.

## Required joint acceptance gates

For each cell, form each block's ratio as software CPU divided by hardware CPU,
then take the geometric mean of those eight paired ratios. A hardware win requires
strictly lower hardware CPU in that pair. All three rows must pass:

| Population | Transfer mode | Node budget | Minimum paired geometric mean | Minimum hardware wins |
|---|---|---:|---:|---:|
| 0 | Raw | 262144 | 1.10 | 6 of 8 |
| 0 | Sparse | 262144 | 1.50 | 6 of 8 |
| 768 | Sparse | 262144 | 1.20 | 6 of 8 |

These are ratio thresholds, not equivalent percentage CPU reductions: 1.10, 1.50
and 1.20 correspond to approximately 9.09%, 33.33% and 16.67% lower hardware CPU.
Report every remaining population/profile cell and all elapsed-time results.
Mechanistic stage or primitive results cannot replace a missed full-operation gate.
Missing/failed observations or identity mismatches invalidate qualification; retain
them and report the failure rather than calculating a favorable subset. If any
registered performance gate misses, record the negative and retain the unchanged
default behavior; do not lower the gate after seeing the results.

The previous [hardware-SHA diagnostic](compute-efficiency-2026-10-29.md#separate-hardware-sha-feature-builds)
showed faster primitive hashing but slower full graph folding in a sequential,
unisolated run. It motivates this experiment and is not full-operation evidence.
Passing this new local gate would qualify only this native fixture/profile scope.
It would not establish a WAN elapsed improvement, a physical-wire saving, energy
savings, a default-enable decision, or support for other platforms.
