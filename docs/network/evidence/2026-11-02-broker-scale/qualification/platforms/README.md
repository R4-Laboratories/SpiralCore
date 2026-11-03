# Final broker portable contracts and Linux workflow

**Both final workflows passed at the exact EF814b source.** The
[seven-platform portable run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41572232084)
executed **63 tests on each native row: 441 passing executions**, plus **eight
Unix storage tests**, for **449 recorded passes**. The separate
[Linux workflow](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41572232073)
passed its **43-test release host/adapter/campaign stage**, **103-test shared
conformance stage**, strict warnings check, default guest compilation and
synthetic analyzers.

These scopes overlap and their counts are not pooled into a scientific trial
count. They qualify native ephemeral-process/loopback client and Linux component
contracts, not WAN, booted-MVM fleet capacity, Windows durable storage or transfer
performance. The [load measurements](../../README.md) retain their separate
source, profile and timing contract.

## Exact executed source

The original identities from all seven ZIPs name executed PR merge
`007c9c00c027b8478acb2d308716cd934b8f79cd`. The Linux checkout log records the same
merge. Its tree is `97717505d278bf8ad79cef2433ec088352d6f90a`, verified equal to
PR head `ef814b995c3b51bdf7ad17ab216c5c279115bdaa`.

Every portable row records Rust 1.97.1 and its native compiler target. Both
portable canonical dependencies resolve to the published MVM contribution fork
`ae12e9deb41cc9a3264196ce22ebc72d4042883d`. The Linux source-verification stage
records eleven MVM dependency packages from the same exact source; its retained
[dependency list](linux-dependencies.json) is parsed from actual job output.
This contributor-fork qualification does not imply upstream MVM acceptance.

| Native row | Recorded actual OS | Runner image version | Compiler host / target | Portable passes | Unix storage passes |
|---|---|---|---|---:|---:|
| Ubuntu 24.04 x64 | Ubuntu 24.04.5 LTS | 20261026.354.1 | x86_64-unknown-linux-gnu | 63 | 2 |
| Ubuntu 24.04 ARM64 | Ubuntu 24.04.5 LTS | 20261026.166.1 | aarch64-unknown-linux-gnu | 63 | 2 |
| macOS 26 ARM64 | macOS 26.6.2 | 20261026.0455.2 | aarch64-apple-darwin | 63 | 2 |
| macOS 26 Intel | macOS 26.6.1 | 20260824.0517.1 | x86_64-apple-darwin | 63 | 2 |
| Windows Server 2022 x64 | Windows build 10.0.20348 | 20261026.347.1 | x86_64-pc-windows-msvc | 63 | Not run |
| Windows Server 2025 x64 | Windows build 10.0.26100 | 20261030.294.1 | x86_64-pc-windows-msvc | 63 | Not run |
| Windows 11 ARM64 | Windows build 10.0.26200 | 20261026.197.1 | aarch64-pc-windows-msvc | 63 | Not run |

All eleven portable targets have passing counts equal to listed counts and zero
failed, ignored or filtered cases. Their target-by-target counts, native runner
metadata, exact source, toolchain and dependency identities are preserved in
[summary.json](summary.json) and the original ZIPs. The two storage tests per
Unix row are separate durable dispatch/reopen checks; Windows storage remains
unqualified.

## Linux execution evidence

The [job/step status](linux-status.json) and
[execution excerpt](linux-execution-excerpt.txt) preserve the source checkout,
actual successful test summaries and warnings/build/analyzer completion. The
43 release tests comprise library 19, adapter-live 10, bandwidth profile one,
campaign nine, recovery one and host packing three. The binary harness ran zero
tests and adds nothing to that total. The shared stage separately passed 103
tests. Test-run durations are not transfer-performance measurements.

The portable smoke uses its named 30-second deadline profile on debug CI builds.
The measured CLI load profile remains at its original one-second deadline. No
production or measured-profile deadline was relaxed to obtain these passes.
Earlier Intel and Windows failures remain separate records in the parent
qualification directory; this final run does not erase or relabel them.

## Permanent original records and integrity

The [seven original artifact ZIPs](original-zips/) total **161,260 bytes** and
retain **215 original files**, including their original per-row manifests.
Every downloaded ZIP matches GitHub's published artifact SHA-256 digest in
[artifact-api.json](artifact-api.json). All **208 inner manifest entries** also
passed content-hash verification. Original Windows CRLF bytes and UTF-8 markers
remain untouched inside the ZIPs; normalization was confined to the checksum
parser.

The original ZIPs are not repacked, and no extracted duplicate stores, binaries,
payload datasets, private bootstraps or full environment dumps are included.
The Linux excerpt is a derived selection with display-color markup removed;
it is not presented as an unmodified full job log.

```sh
shasum -a 256 -c SHA256SUMS
```

This verifies every retained packet file, including the original ZIPs. The
[portable status](portable-status.json) preserves final runner job identities,
timestamps and successful conclusions. The complete packet stays below 1 MiB.
