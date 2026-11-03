# Native mailbox portability results — seven platform rows passed

[GitHub Actions run 41535679868, attempt 1](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41535679868)
executed **322 portable tests plus eight separately classified Unix storage
tests: 330 passing test executions**. Every row executed all six portable test
targets without failed, ignored or filtered tests. These are native process and
TCP-loopback correctness results, not real WAN or MVM guest/backend results.

The executed source was merge commit
[`7506b3f3a40ee7dc190d665031750d336fd65aeb`](https://github.com/R4-Laboratories/SpiralCore/commit/7506b3f3a40ee7dc190d665031750d336fd65aeb),
Git tree `ae5521d8ca0a02a774a25d9396b3d5d9fc4f855c`. Every artifact's recorded
source commit matches this identity; the tree was also checked locally through
Git. Dependency evidence records the published MVM fork revision
`5766754fbaae59dbee548f968716a514d3a796c1` for both canonical MVM crates. All seven
rows executed Rust 1.97.1 with the native compiler host triple, locked dependencies
and the `live` feature, without the hardware SHA assembly profile.

| Native row | Actual OS/build | Runner image version | Portable passes | Unix storage passes |
|---|---|---|---:|---:|
| Ubuntu 24.04 x64 | Ubuntu 24.04.5 LTS; kernel 7.0.0-1017-azure | 20261026.354.1 | 46 | 2 |
| Ubuntu 24.04 ARM64 | Ubuntu 24.04.5 LTS; kernel 7.0.0-1017-azure | 20261026.166.1 | 46 | 2 |
| macOS 26 ARM64 | macOS 26.6.2, build 25G83 | 20261026.0455.2 | 46 | 2 |
| macOS 26 Intel | macOS 26.6.1, build 25G76 | 20260824.0517.1 | 46 | 2 |
| Windows Server 2022 x64 | Windows build 10.0.20348.0 | 20261026.347.1 | 46 | Not run |
| Windows Server 2025 x64 | Windows build 10.0.26100.0 | 20261030.294.1 | 46 | Not run |
| Windows 11 ARM64 | Windows build 10.0.26200.0 | 20261026.197.1 | 46 | Not run |

The portable targets were `canonical` (9), `service` (16), `receipt` (15),
`service_epoch` (3), `service_sender` (2), and `service_tcp` (1). The separate
`canonical_storage` target executed two Unix dispatch/reopen tests on each
Linux/macOS row. Windows storage is deliberately not represented by these tests
and remains an open delivery gate.

The [machine-readable summary](summary.json) records actual OS/image/target
identities, counts, artifact paths, source/tree identity and limitations. The
[original per-row records](official/41535679868/) retain listings, output,
counts, dependency source, compiler identity and runner identity. Their content
totals 234,081 bytes (632 KiB allocated locally), including the original checksum
manifests. No payload stores or build outputs are included.

## Artifact integrity and line endings

All **138 files listed by the seven original checksum manifests** were checked
against SHA-256 and the copied files compared byte-for-byte with the downloaded
artifacts. The original manifests and file contents are preserved unchanged. An additional
[repository checksum inventory](official-checksums.sha256) pins all 145 original
files, including the seven checksum manifests themselves. Run
`shasum -a 256 -c official-checksums.sha256` from this report directory to verify
the complete archive without platform-specific manifest parsing.
The scoped [.gitattributes](.gitattributes) disables Git text conversion only for
`official/**` so Windows CRLF bytes remain intact on all future checkouts. The same scoped
attribute disables whitespace rewriting/checks for these immutable artifacts,
which include CRLF and trailing log blank lines. Source and authored reports
retain normal whitespace checks.

The Windows manifests use CRLF line endings. A Unix checksum reader that treats
the terminal carriage return as part of the filename reports a missing file;
this is a manifest parsing error, not an artifact corruption or product failure.
Normalize only the parser input, while hashing the untouched files:

```sh
cd official/41535679868/mvm-mailbox-portable-windows2022-x64-1
sed 's/\r$//' SHA256SUMS | shasum -a 256 -c -
```

Windows checkout-time text conversion also explains the distinct raw manifest
and Cargo.lock hashes recorded by Windows versus Unix runners. Both identities
remain in the original evidence; they are not silently rewritten to a common
hash. The workflow's locked build and clean tracked-lock check succeeded in each
native environment.

## Separate failed CI run and remaining acceptance

The older Linux MVM workflow's
[run 41535679640](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41535679640)
failed while still using its obsolete source/preparation configuration. That
failure is separate from the seven-row portability run and remains retained.
The corrective workflow commit is
`a194546c2cbb73f3a24df845611a87e466e364f8`; at this report checkpoint its push and
executed replacement Linux workflow result remain pending. Do not convert this
workflow edit into a passing execution claim.

No transfer throughput, byte-saving, CPU performance, send-only time or energy
claim follows from these test logs. Real WAN endpoint pairs, native
KVM/HVF/libkrun guest/host execution, Windows durable storage, FlowMux integration,
owner access, additional OS versions and production security qualification retain
their own acceptance gates. The seven rows establish the pinned native ephemeral
mailbox/client scope, including actual Windows execution rather than WSL or
cross-compilation.
