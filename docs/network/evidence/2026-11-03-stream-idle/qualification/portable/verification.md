# Hosted idle-source qualification custody

Verified 2026-11-03. **PASS for original artifact custody, native client row
execution, Linux build/component gates and exact tested source tree.** No new
CI, tests, builds, measurements or guests were launched by this verification.

[Portable run 41638793063](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41638793063)
and [Linux run 41638793269](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41638793269)
completed successfully, attempt 1, for reviewed PR head
`960e73c59d7eab62f15a16667f95e594e1cabc8b`. API run/job/artifact records and the
original `gh run view --log` Linux output are retained. Eight portable jobs are
successful: one matrix-generation job and seven execution rows; this is not eight
native rows. Every Linux job/step succeeded; portable nonexecuted conditional
steps are explicitly skipped, not counted as tests.

## Original archive custody

`../download.sh` retains the exact download commands. Exactly seven original ZIPs
were obtained using `gh api repos/R4-Laboratories/SpiralCore/actions/artifacts/ID/zip`.
All **seven API SHA-256 digests and byte lengths match**, totaling **192,021 ZIP
bytes**. Each archive contains only flat regular files and was extracted to its
own subdirectory. **All 236 listed inner checksums pass**: 32 per Windows row and
35 per Unix row; the original archive digest covers SHA256SUMS itself. The archive
member count equals the listed checksums plus SHA256SUMS in every row.
`custody.tsv`, `archives.sha256` and `extracted/*.checksums.txt` retain these checks.

Original ZIPs and extracted files were not rewritten. Windows CRLF was handled
only in checksum/log parsing. Windows manifest/lock digests differ from Unix
checkout bytes; `source-projections.tsv` confirms both LF and CRLF projections
of the same tested Git blobs, computed without rewriting an archive or checkout:

```sh
git show TESTED_MERGE:PATH | shasum -a 256
git show TESTED_MERGE:PATH | awk '{printf "%s\r\n", $0}' | shasum -a 256
```

The lock path is `crates/spiralcore-mvm-adapter/Cargo.lock`. An initial derivation
used nonexistent root Cargo.lock and stopped; the corrected retained script
succeeded. No source/archive changed as a result.

## Exact checkout identity

Every row's identity.json records the same checkout and GITHUB_SHA. The original
Linux checkout log independently records it. The merge object was fetched by
exact SHA with `git fetch --no-tags origin SHA`; no checkout, branch or index was
changed. `verify-source.sh` and `source-tree.txt` retain the comparisons.

```text
tested merge  1da75b1aa4a6aacda5f47706caeeeaa213f0e99a
base parent   62692428cd1e9effc51c7930f7b1f34cc1519277
head parent   960e73c59d7eab62f15a16667f95e594e1cabc8b
merge tree    7d4d1568fd6f2143992b145abf68a790b1b1d4ae
head tree     7d4d1568fd6f2143992b145abf68a790b1b1d4ae
```

The **entire tested merge tree equals the reviewed head tree**; Git tree IDs and
empty `git diff --exit-code MERGE HEAD` both agree. The saved tested-manifest.json
is read from that merge, including service_tcp minimum 11 and receipt minimum 21.
Historical manifest status fields are not treated as this run's observations.
This identity applies to the pinned source, not a later evidence-only commit or
future merge snapshot.

## Seven native client rows

All actual runner OS/architecture, process and OS architecture, Rust 1.97.1 host
and expected target match their declared row. Both canonical MVM dependencies
resolve to exact pinned ae12e9deb41cc9a3264196ce22ebc72d4042883d. Every row's 13
target listings, JSON counts and actual log summaries agree: **88 passed per row,
including TCP 11 and receipt 21**, with zero failed/ignored/measured/filtered.

| Row | Actual OS description | Process/OS arch | Native Rust host | Portable passed | Separate Unix storage |
|---|---|---|---|---:|---:|
| ubuntu24-x64 | Ubuntu 24.04.5 LTS | X64 | x86_64-unknown-linux-gnu | 88 | 2 |
| ubuntu24-arm64 | Ubuntu 24.04.5 LTS | Arm64 | aarch64-unknown-linux-gnu | 88 | 2 |
| macos26-arm64 | macOS 26.6.2 | Arm64 | aarch64-apple-darwin | 88 | 2 |
| macos26-intel | macOS 26.6.1 | X64 | x86_64-apple-darwin | 88 | 2 |
| windows2022-x64 | Microsoft Windows 10.0.20348 | X64 | x86_64-pc-windows-msvc | 88 | Not executed |
| windows2025-x64 | Microsoft Windows 10.0.26100 | X64 | x86_64-pc-windows-msvc | 88 | Not executed |
| windows11-arm64 | Microsoft Windows 10.0.26200 | Arm64 | aarch64-pc-windows-msvc | 88 | Not executed |

There are **616 portable test executions**, plus **eight separate Unix storage
dispatch/reopen executions across four rows**. Windows storage is absent and not
credited. These are native client/process, ephemeral canonical mailbox/component
and TCP-loopback results. They establish no Windows MVM backend, booted guest,
WAN, performance, energy or fleet capacity result.

## Linux scope and overlap

The original log and retained `derive-linux.awk`/`linux-test-counts.tsv` show:

| Scope | Passed | Result summaries | Zero-test summaries |
|---|---:|---:|---:|
| Release live crate, linux-host, all targets | 92 | 10 | 2 |
| Shared object-v1 contract | 8 | 1 | 0 |
| Shared adapter, live/conformance/hardware-sha256, all targets | 123 | 22 | 2 |

Shared total is **131 = 8 object-v1 + 123 adapter**. Every summary has zero
failed/ignored/measured/filtered. Both release and adapter zero-test targets remain
visible; none is credited as a nonzero test gate. Relative to the preceding
session-events packet, live release remains 92 and adapter increases 116 to 123
solely through seven new service_tcp idle checks (4 to 11). These new checks run
in the shared stage; they are not silently credited to the live release stage.

The successful strict Clippy step uses `--all-targets -- -D warnings`; default
guest-entry `cargo check --all-targets` also succeeds. Actual log PASS records
confirm the synthetic legacy/v1 analyzer's altered-frame/identity/duplicate-cell
negatives and the combined synthetic HVF/Linux analyzer's known paired ratios,
exact readback and mutation negatives. These are build/component/synthetic
analyzer checks, not booted MVM or transfer-performance evidence.

Counts overlap across stages, targets and platforms; **92, 131 and 616 are not
additive independent test coverage**. `rows.json`, `verification.json` and
`target-counts.tsv` retain compact observations. The copied/adapted previous
verification scripts validate only this new packet and leave the previous packet
untouched. A preliminary assertion expected seven jobs; metadata showed the
additional successful matrix-generation job, and the corrected verifier requires
eight jobs and seven artifact rows separately. No execution failure is implied.
The complete local qualification packet remains within a few MiB. No private
bootstrap files, global process arguments or shell history are included.
