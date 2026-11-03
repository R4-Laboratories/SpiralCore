# Hosted qualification artifact verification

Verified on 2026-11-03. **PASS for artifact custody, native portable row
execution, Linux component/build gates and the exact tested merge tree.**
No tests, products, CI jobs, performance measurements or native guests were
started by this verification task.

Runs: [portable 41629079515](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41629079515)
and [Linux 41629079360](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41629079360),
both successful for PR head `5c41d6ac2d9b40e6eead62e201d5e9b020f96019`.
Original API/run metadata are retained in `artifacts.json`, `run.json` and the
parent `linux-run.json`; the original Linux log is `../linux-run.log`.

## Custody and source identity

Downloaded exactly the seven original ZIPs through:

```sh
gh api repos/R4-Laboratories/SpiralCore/actions/artifacts/ARTIFACT_ID/zip > zips/ARTIFACT_NAME.zip
```

All seven original byte lengths and SHA-256 digests match their saved API
records: **188,522 ZIP bytes** in total. Every archive was extracted to its own
owned subdirectory after confirming flat regular-file members. **All 236 listed
inner file checksums pass** (32 each for three Windows artifacts, 35 each for
four Unix artifacts); the archive digest covers each checksum manifest itself.
`custody.tsv`, `archives.sha256` and `extracted/*.checksums.txt` retain the checks.

ZIPs and extracted evidence files were not rewritten. Windows CRLF was handled
only in checksum/log parsing. Windows checkout manifest/lock hashes differ from
Unix because their checked-out text uses CRLF: both Windows hashes were verified
against CRLF stream projections of the same Git blobs, without modifying any
archive or source file. Exact source-file digest pairs are retained in `rows.json`.
The source projections were computed with `git show TESTED_MERGE:PATH` (Unix)
and `git show TESTED_MERGE:PATH | awk '{printf "%s\r\n", $0}'` (Windows).

Every row's `identity.json` records checkout commit and GITHUB_SHA
`3e7c94b23cfc4a8b1bdf0bc09cb7d8e832619527`, the PR merge snapshot. The Linux
checkout log independently records the same merge. It was fetched by exact SHA
with `git fetch --no-tags origin SHA`, without checkout/index/branch changes.

```text
tested merge:  3e7c94b23cfc4a8b1bdf0bc09cb7d8e832619527
merge parents: 231ab32103f58e52c206d6e5917d883de4e3a928
               5c41d6ac2d9b40e6eead62e201d5e9b020f96019
merge tree:    ea05391bd4f077339a6338a102b68e62d83f8815
head tree:     ea05391bd4f077339a6338a102b68e62d83f8815
```

The **entire merge tree equals the reviewed head tree**, confirmed both by Git
tree IDs and an empty `git diff --exit-code MERGE HEAD`. `source-tree.txt` retains
these object identities. This claim applies to the pinned tested head, not a
later evidence-only commit or a future merge snapshot.

## Native row observations

For every row, actual runner OS/architecture, process architecture, OS
architecture and Rust 1.97.1 host match the declared native target. Both canonical
MVM dependencies resolve to the manifest's exact pinned `ae12e9de` source.
Each row's 13 target listings, JSON counts and actual test-log summaries agree:
**81 passed per row, including all 21 receipt tests**. Failed, ignored, measured
and filtered counts are all zero. Receipt minimum is 21. The row total is checked
against the actual target manifest rather than its historical status fields.

| Row | Recorded actual OS description | Process/OS arch | Native Rust host | Portable passed | Separate Unix storage |
|---|---|---|---|---:|---:|
| ubuntu24-x64 | Ubuntu 24.04.5 LTS | X64 | x86_64-unknown-linux-gnu | 81 | 2 |
| ubuntu24-arm64 | Ubuntu 24.04.5 LTS | Arm64 | aarch64-unknown-linux-gnu | 81 | 2 |
| macos26-arm64 | macOS 26.6.2 | Arm64 | aarch64-apple-darwin | 81 | 2 |
| macos26-intel | macOS 26.6.1 | X64 | x86_64-apple-darwin | 81 | 2 |
| windows2022-x64 | Microsoft Windows 10.0.20348 | X64 | x86_64-pc-windows-msvc | 81 | Not executed |
| windows2025-x64 | Microsoft Windows 10.0.26100 | X64 | x86_64-pc-windows-msvc | 81 | Not executed |
| windows11-arm64 | Microsoft Windows 10.0.26200 | Arm64 | aarch64-pc-windows-msvc | 81 | Not executed |

This is **567 portable test executions across seven native client rows**, plus
**eight separate Unix storage dispatch/reopen executions across four rows**.
Windows storage execution is absent, not silently credited. Portable results are
native-process, ephemeral canonical mailbox/component and TCP-loopback evidence.
They do not establish a Windows MVM backend, any booted guest, WAN or scale.

## Linux workflow scopes and overlap

The saved log independently confirms these scopes, including zero-test targets:

| Scope | Passed | Result summaries | Zero-test summaries |
|---|---:|---:|---:|
| Release live crate, linux-host, all targets | 92 | 10 | 2 |
| Shared object-v1 contract | 8 | 1 | 0 |
| Shared adapter, live/conformance/hardware-sha256, all targets | 116 | 22 | 2 |

The shared stage therefore has **124 executions = 8 object-v1 + 116 adapter**.
All failures, ignored, measured and filtered counts in these summaries are zero.
`linux-test-counts.tsv` retains every target, including release main/example zero
targets and the two zero-test adapter examples. No zero summary is counted as a
nonzero acceptance gate. Counts overlap across workflow stages and portable
rows; 92, 124 and 567 are not additive independent test coverage.

The run's step records and actual log also confirm strict Clippy with
`--all-targets -- -D warnings`, the default guest-entry `cargo check --all-targets`,
the synthetic legacy/v1 comparison analyzer (rejects altered frame bytes,
identities and duplicate cells), and the combined synthetic HVF/Linux analyzer
(known paired ratios, exact-readback and mutation negatives). These are build,
component and synthetic-analyzer checks, not booted MVM or transfer-performance
measurements.

`verify-artifacts.sh`, `verify-rows.sh` and `derive-linux.awk` retain the custody
and count derivations. `verification.json`, `rows.json` and `target-counts.tsv`
provide compact machine-readable observations; original ZIPs and evidence logs
remain retained. Total local packet size is approximately 1.4 MiB, within the
requested few-megabyte bound.
