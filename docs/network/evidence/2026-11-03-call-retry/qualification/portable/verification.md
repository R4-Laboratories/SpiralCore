# Registered Call retry: hosted qualification custody

2026-11-03. **PASS — exact source, original artifacts, native client rows and Linux
build/component qualification verified.** No new tests, builds, CI, performance
runs or guests were launched by this verification.

[Portable run 41649886300](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41649886300)
and [Linux run 41649886275](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41649886275)
completed successfully at attempt 1 for head
`d14271ddec5424326603572693c15aeb1a2f2e48`. Original run/job/API metadata and the
original Linux `gh run view --log` output are retained. Portable has seven native
execution rows plus its successful matrix-generation job; the latter is not an
extra native row. Successful/skipped conditional steps remain distinct.

## Original ZIP and source custody

`../download.sh portable` downloaded exactly seven original ZIPs through
`gh api repos/R4-Laboratories/SpiralCore/actions/artifacts/ID/zip`.
All seven API byte lengths and SHA-256 digests match: **193,853 ZIP bytes**.
Flat regular-file member checks pass. All **236 inner listed checksums** pass:
32 per Windows row, 35 per Unix row; member counts equal listed hashes plus the
original SHA256SUMS, which is covered by the archive digest. Original ZIP bytes
are preserved. `custody.tsv`, `archives.sha256` and `extracted/*.checksums.txt`
retain the checks. Only owned duplicate extracted copies were removed after
verification; each original file remains recoverable from its retained ZIP.
Running verify-artifacts.sh again recreates those temporary directories before
verify-rows.sh. Compact row/count observations are retained instead of a second
full mirror of every archived log.

Windows CRLF was handled only while parsing checksums/logs, without rewriting
archives or members. Row manifest and adapter Cargo.lock hashes match LF/CRLF
stream projections of the same Git blobs, retained in source-projections.tsv.
Both canonical MVM packages resolve to the exact manifest-pinned source
`ae12e9deb41cc9a3264196ce22ebc72d4042883d`; no alternative dependency is credited.

Every row identity.json and the original Linux checkout log record:

```text
tested merge  1746e57c1adaf57a32269ee2d7bde213d4fed5ce
base parent   a8a4a9a8daba67c00447a067b18632ed515d95b2
head parent   d14271ddec5424326603572693c15aeb1a2f2e48
merge tree    e5c32fa15b132ca1ad28cfdc5914787079b18df2
head tree     e5c32fa15b132ca1ad28cfdc5914787079b18df2
```

The object was fetched by exact SHA without changing checkout/index/branch.
Tree IDs match and `git diff --exit-code MERGE HEAD` is empty for the entire tree,
not merely crate files. The saved tested-manifest.json is read from that merge.
The reviewed source head also has an empty crate diff against locally qualified
4d34c6a7. This establishes the pinned tested source, not a future merge identity
or later evidence-only commit.

## Native client observations

All seven actual runner OS/architecture, process/OS architecture, Rust 1.97.1 host
and expected target agree with the tested manifest. Each row's 13 target listings,
JSON counts and actual log summaries agree: **92 passed**, including call_send 6,
service_tcp 11 and receipt 21. Failed/ignored/measured/filtered counts are zero.
The required minima are checked against actual tests, not historical status text.

| Row | Actual OS description | Process/OS arch | Native Rust host | Portable passed | Separate Unix storage |
|---|---|---|---|---:|---:|
| ubuntu24-x64 | Ubuntu 24.04.5 LTS | X64 | x86_64-unknown-linux-gnu | 92 | 2 |
| ubuntu24-arm64 | Ubuntu 24.04.5 LTS | Arm64 | aarch64-unknown-linux-gnu | 92 | 2 |
| macos26-arm64 | macOS 26.6.2 | Arm64 | aarch64-apple-darwin | 92 | 2 |
| macos26-intel | macOS 26.6.1 | X64 | x86_64-apple-darwin | 92 | 2 |
| windows2022-x64 | Microsoft Windows 10.0.20348 | X64 | x86_64-pc-windows-msvc | 92 | Not executed |
| windows2025-x64 | Microsoft Windows 10.0.26100 | X64 | x86_64-pc-windows-msvc | 92 | Not executed |
| windows11-arm64 | Microsoft Windows 10.0.26200 | Arm64 | aarch64-pc-windows-msvc | 92 | Not executed |

This is **644 native client/component TCP-loopback executions**, plus **eight
separate Unix storage dispatch/reopen executions** across four rows. Windows
storage is absent. There is no booted guest, native Windows MVM, WAN, fleet,
performance or energy qualification from these rows.

## Actual Linux scopes and limits

The retained original log, derive-linux.awk and linux-test-counts.tsv confirm:

| Scope | Passed | Result summaries | Zero-test summaries |
|---|---:|---:|---:|
| Release live crate, linux-host, all targets | 92 | 10 | 2 |
| Shared object-v1 contract | 8 | 1 | 0 |
| Shared adapter, live/conformance/hardware-sha256, all targets | 127 | 22 | 2 |

Shared total is **135 = 8 + 127**. All failure/ignored/measured/filtered counts are
zero; all four zero-test targets remain visible without being counted as nonzero
acceptance gates. Compared with the idle packet, four extra call_send tests
increase only the shared adapter stage, 123 to 127; live release stays 92.
Counts overlap across stages/targets/platforms: 92, 135, local 127 and portable
644 are not additive independent coverage.

Actual command records and successful job steps confirm strict all-target Clippy
with `-D warnings`, default guest-entry all-target cargo check, synthetic legacy/v1
altered-frame/identity/duplicate-cell negatives and synthetic combined HVF/Linux
known-ratio/exact-readback/mutation negatives. Those are build/component/analyzer
checks, not new normal transfer or native performance measurements. The API is
unused by the normal engine; no performance rerun or improvement is claimed.

`verify-artifacts.sh`, `verify-source.sh`, `verify-rows.sh` and verify-linux.sh
retain custody/count derivations. rows.json, verification.json and target-counts.tsv
retain compact machine-readable observations; original ZIPs contain the complete
row files. Protected merge and actual delivery remain root-owned. Full C8,
consumer/timer, uncertain-send reconciliation, completed-receiver recovery,
reconnect/resume, larger retention and wider roadmap gates remain open.
