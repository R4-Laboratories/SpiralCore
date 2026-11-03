# Native ephemeral mailbox CI

Tracking: [platform qualification #46](https://github.com/R4-Laboratories/SpiralCore/issues/46),
under [MVM delivery #42](https://github.com/R4-Laboratories/SpiralCore/issues/42).

The [workflow](../../.github/workflows/mvm-mailbox-portable.yml) reads its matrix
and minimum test counts from the [support manifest](mvm-mailbox-portable.json).
It runs on pull requests affecting the implementation or workflow, and supports
manual dispatch. Its scope is the native remote mailbox client and ephemeral
canonical service process, using loopback TCP for the portable transport test.
A successful row does not qualify a durable object gateway or native MVM host.

The seven selected versioned runner labels were checked against GitHub's
[hosted-runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
and [official image inventory](https://github.com/actions/runner-images) on
2026-11-02. These are standard runner labels rather than paid `large`/`xlarge`
labels. Release labels are explicit; their images still change, so every job
records the actual image version and OS/kernel/build. If a runner is unavailable,
that row remains an unmet gate; no cross-compiled or emulated fallback replaces it.

The scripts use PowerShell Core consistently on all rows. The official
[Ubuntu ARM64 image inventory](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Arm64-Readme.md)
and [macOS Intel image inventory](https://github.com/actions/runner-images/blob/main/images/macos/macos-26-Readme.md)
list PowerShell as installed; its absence is a failing job rather than a shell
fallback. The selected tests contain no environment-controlled skip path.

Each row installs Rust 1.97.1 with the declared native host triple and checks the
actual runner OS/architecture and compiler host. The Windows profile uses MSVC.
All rows use `--locked --no-default-features --features live` against the
published MVM fork revision pinned in the manifest. Dependency metadata must
match that revision for both `mvm-core` and `mvm-contract`. No local prepared-source
patch or hardware SHA assembly feature is enabled.

| Portable test target | Minimum executed tests |
|---|---:|
| `canonical` | 9 |
| `service` | 16 |
| `receipt` | 15 |
| `service_epoch` | 3 |
| `service_sender` | 2 |
| `service_tcp` | 1 |
| `service_clock` | 4 |

Every target is first enumerated and then executed without a filter. Passing
counts must equal the enumerated count and meet the minimum. Failed, ignored,
measured or filtered cases fail the row; a Unix-only target silently running
zero tests cannot pass. The current portable targets require at least **50 passes on every row**,
including four new clock-policy regressions. Their all-platform execution status
is **PASS at the pinned current `ae12` dependency source** in run 41546165676:
all seven native rows executed the expanded coverage. Earlier 46/50-test
checkpoints remain separate historical evidence at their original dependencies. The two existing
durable storage dispatch tests were moved unchanged from `canonical.rs` into
`canonical_storage.rs`, retaining the shared fixture module. A separate explicit
Linux/macOS-only step enumerates and executes both, records separate counts and
rejects zero/ignored/filtered coverage. Unix `--all-targets` also retains both
storage tests. Windows does not run this target and remains unqualified for
durable object storage. Other Unix endpoint/storage suites remain separate. Raise minimum counts when expanding required coverage;
do not lower them to accommodate a platform failure.

Each job retains compact logs, listings, machine-readable counts, source/MVM
revision, lockfile and manifest hashes, compiler identity and actual runner
identity. A checksum manifest accompanies each artifact, retained for 14 days.
Permanent release evidence should copy the compact result summaries into the
repository with the final commit/run links before artifact expiry. No payload
stores, build outputs, credentials or full environment dumps are uploaded.
Jobs have read-only repository permissions, pinned action SHAs, bounded timeouts
and at most three concurrent test rows.

Latest executed status is **PASS for 50 tests on all seven native rows** in
[run 41546165676](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41546165676):
350 portable tests plus eight Unix storage tests, using published MVM revision
`ae12e9deb41cc9a3264196ce22ebc72d4042883d`. Actual merge source
`f06af8e5aa7e73c71af84edb0fd2f1ee5543d1b9` has the same tree as PR head
`25234ddb44589f752d28870190af985852ddc1a7`. The
[permanent report and original artifacts](../network/evidence/2026-11-02-mvm-service/platforms/README.md)
preserve exact identities and the earlier 330/358-execution checkpoints. The
later documentation-only checkpoint does not create a new protocol implementation
or change these pinned execution identities.

The separate [Linux MVM workflow 41546165642](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41546165642)
passed at the same merge source: 42 release-mode host/adapter/campaign tests,
90 shared conformance tests, strict warnings, default guest compilation and
synthetic analyzers. Its exact-readback 16 MiB loopback campaign passed. The older
debug 16 MiB `Expired` failure remains retained; release-mode success does not erase
it or establish a booted MVM, real WAN or transfer-performance result.

Remaining explicit gates include native backend/16 MiB qualification, real WAN
in both directions, Windows durable storage, NIC attachment, FlowMux and owner
access, additional OS/version/ABI combinations, security and performance.
[Scale #49](https://github.com/R4-Laboratories/SpiralCore/issues/49) separately
requires measured aggregate limits, long-lived bounds, fairness, saturation and
tail latency; two guests or simulated endpoints do not prove fleet capacity.
See the [platform contract](../specs/2026-11-02-mvm-platforms.md) for the complete
boundary.
