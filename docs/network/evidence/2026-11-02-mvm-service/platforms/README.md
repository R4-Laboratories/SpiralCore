# Native mailbox portability — clock-expanded matrix passed

The latest [seven-platform run 41539620012, attempt 1](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41539620012)
passed **350 portable tests plus eight separately classified Unix storage tests:
358 passing executions**. This includes four clock-policy regressions on every
native row. No tests were failed, ignored, measured or filtered in the required
targets. These are native process and TCP-loopback correctness results; they do
not qualify real WAN transport, a native MVM guest/backend or performance.

## Exact executed source

The workflow executed PR merge commit
[`104187b956190c01351b10e0530ce928a66620df`](https://github.com/R4-Laboratories/SpiralCore/commit/104187b956190c01351b10e0530ce928a66620df),
recorded independently in every artifact. Its Git tree is
`14d3a61b4a4134435102562817e047c78ff5803f`. Fetching the PR merge ref and resolving
its tree confirmed equality with PR head
[`2a4a199f0bae224fbe82aa148feb20273bd64d84`](https://github.com/R4-Laboratories/SpiralCore/commit/2a4a199f0bae224fbe82aa148feb20273bd64d84).
Later source changes, including native disk-custody changes, are not silently
qualified by this run.

Every row used Rust 1.97.1 with the recorded native compiler host triple, locked
dependencies, the `live` feature and no hardware SHA assembly profile. Dependency
records identify the published MVM fork revision
`5766754fbaae59dbee548f968716a514d3a796c1` for the canonical MVM crates.

| Native row | Actual OS/build | Runner image version | Portable passes | Unix storage passes |
|---|---|---|---:|---:|
| Ubuntu 24.04 x64 | Ubuntu 24.04.5 LTS; kernel 7.0.0-1017-azure | 20261026.354.1 | 50 | 2 |
| Ubuntu 24.04 ARM64 | Ubuntu 24.04.5 LTS; kernel 7.0.0-1017-azure | 20261026.166.1 | 50 | 2 |
| macOS 26 ARM64 | macOS 26.6.2, build 25G83 | 20261026.0455.2 | 50 | 2 |
| macOS 26 Intel | macOS 26.6.1, build 25G76 | 20260824.0517.1 | 50 | 2 |
| Windows Server 2022 x64 | Windows build 10.0.20348.0 | 20261026.347.1 | 50 | Not run |
| Windows Server 2025 x64 | Windows build 10.0.26100.0 | 20261030.294.1 | 50 | Not run |
| Windows 11 ARM64 | Windows build 10.0.26200.0 | 20261026.197.1 | 50 | Not run |

Each row executed `canonical` (9), `service` (16), `receipt` (15),
`service_epoch` (3), `service_sender` (2), `service_tcp` (1) and `service_clock`
(4). The separate `canonical_storage` target executed two Unix dispatch/reopen
tests on each Linux/macOS row. Windows durable storage remains unqualified.
The clock cases exercise strict receive time, explicitly bounded issuance,
retry stability and refusing insufficient reverse clock windows before admission.

## Permanent records and original integrity

- [Latest machine-readable summary](summary-41539620012.json)
- [GitHub job status, runner IDs and execution timestamps](ci-status-41539620012.json)
- [Latest original per-row artifacts](official/41539620012/)
- [Latest checksum inventory covering all 159 original files](official-checksums-41539620012.sha256)
- [Original 330-execution checkpoint report](checkpoint-41535679868.md),
  [summary](summary.json), [artifacts](official/41535679868/) and
  [145-file checksum inventory](official-checksums.sha256)

The original checkpoint remains unchanged: run 41535679868 executed 46 portable
tests per row at merge source `7506b3f3a40ee7dc190d665031750d336fd65aeb`, tree
`ae5521d8ca0a02a774a25d9396b3d5d9fc4f855c`, yielding 322 portable plus eight Unix
storage passes. It did not execute `service_clock`.

All **152 files listed by the latest seven original checksum manifests** match
their SHA-256 hashes. The additional repository inventory also pins the seven
manifests themselves, for **159 verified archived files**. Latest artifact
contents total 259,998 bytes. The records contain logs, listings, counts,
dependency sources, compiler and runner identities; no payload stores, build
outputs, credentials or full environment dumps are included.

Original Windows manifests and records retain CRLF bytes. Verification strips
line-ending CR only from checksum parser input; it hashes untouched file bytes.
A Unix parser treating CR as part of a filename can report a missing file even
when the hash is valid. The scoped [.gitattributes](.gitattributes) disables text
conversion and whitespace checks only for immutable `official/**` artifacts,
including their original trailing log blank lines. Authored reports and source
retain normal checks. From this report directory, verify the entire latest
archive using its additional LF-delimited inventory:

```sh
shasum -a 256 -c official-checksums-41539620012.sha256
```

Raw checkout-time manifest/Cargo.lock hashes differ between Windows and Unix
because of text line endings; both original identities remain recorded. The
locked build and clean tracked-lock checks succeeded in each environment.

## Separate Linux MVM workflow failure remains open

The separate [Linux MVM contracts run 41539620010](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41539620010)
completed with **failure** at head `2a4a199f0bae224fbe82aa148feb20273bd64d84`.
Its `campaign` target passed seven tests and failed
`qualified_16m_whole_mixed_service_session_reopens_exactly`: the caller reported
`Expired` and the receiver reported `object endpoint refused operation`.
The [retained failure excerpt](linux-contracts-41539620010-failure.txt) records
those errors and the 466.14-second debug test-target wall time. That duration is
not transfer-performance evidence. Diagnosis and a passing replacement gate
remain required; the portability pass does not waive this failure.

The [older Linux run 41535679640](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41535679640)
failed with the obsolete source/preparation workflow. The correction
`a194546c2cbb73f3a24df845611a87e466e364f8` was incorporated before the new Linux
run. The new failure above is distinct from that historical configuration issue.

Real WAN pairs, actual KVM/HVF/libkrun guest/host witnesses, Windows durable
storage, FlowMux, owner access, additional OS versions and production security
retain their own acceptance gates. No throughput, byte-saving, CPU performance,
send-only time, energy or security-certification claim follows from these logs.
