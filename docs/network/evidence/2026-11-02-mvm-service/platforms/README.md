# Current native mailbox clients and Linux contract workflow passed

The latest [seven-platform run 41546165676](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41546165676)
passed **350 portable tests plus eight Unix storage tests: 358 passing executions**
with published MVM dependency `ae12e9deb41cc9a3264196ce22ebc72d4042883d`.
The separate [Linux MVM contracts run 41546165642](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41546165642)
also passed, including **42 release-mode host/adapter/campaign tests**, shared
conformance, strict warnings and the default guest entry compilation/analyzer.

This qualifies the pinned native ephemeral mailbox client/service and loopback
contract scope. These jobs do not establish a booted MVM, real WAN transfer,
fleet capacity, Windows durable storage or transfer-performance result.

## Exact executed source and platform identities

All seven artifact identities record executed PR merge source
[`f06af8e5aa7e73c71af84edb0fd2f1ee5543d1b9`](https://github.com/R4-Laboratories/SpiralCore/commit/f06af8e5aa7e73c71af84edb0fd2f1ee5543d1b9).
The Linux contract checkout log records the same merge. Its tree is
`046d3d4c646ac29d65057daddf64d4332691308d`, verified equal to PR head
[`25234ddb44589f752d28870190af985852ddc1a7`](https://github.com/R4-Laboratories/SpiralCore/commit/25234ddb44589f752d28870190af985852ddc1a7).
The later documentation-only checkpoint does not introduce a new protocol
implementation or replace these exact execution identities.

All platform rows used Rust 1.97.1 with the recorded native compiler host triple,
locked dependencies and the `live` feature without hardware SHA assembly. Both
canonical dependency records identify published contributor-fork revision
`ae12e9deb41cc9a3264196ce22ebc72d4042883d`, including its explicit Firecracker state
directory correction. Contributor-fork qualification is not upstream acceptance.

| Native row | Actual OS/build | Runner image version | Portable passes | Unix storage passes |
|---|---|---|---:|---:|
| Ubuntu 24.04 x64 | Ubuntu 24.04.5 LTS; kernel 7.0.0-1017-azure | 20261026.354.1 | 50 | 2 |
| Ubuntu 24.04 ARM64 | Ubuntu 24.04.5 LTS; kernel 7.0.0-1017-azure | 20261026.166.1 | 50 | 2 |
| macOS 26 ARM64 | macOS 26.6.2, build 25G83 | 20261021.0445.8 | 50 | 2 |
| macOS 26 Intel | macOS 26.6.1, build 25G76 | 20260824.0517.1 | 50 | 2 |
| Windows Server 2022 x64 | Windows build 10.0.20348.0 | 20261026.347.1 | 50 | Not run |
| Windows Server 2025 x64 | Windows build 10.0.26100.0 | 20261030.294.1 | 50 | Not run |
| Windows 11 ARM64 | Windows build 10.0.26200.0 | 20261026.197.1 | 50 | Not run |

Every row executed `canonical` (9), `service` (16), `receipt` (15),
`service_epoch` (3), `service_sender` (2), `service_tcp` (1) and `service_clock`
(4), with listed counts equal to passing counts and zero failed, ignored,
measured or filtered cases. The separate `canonical_storage` target passed two
Unix dispatch/reopen cases on each Linux/macOS row. Windows durable storage
remains explicitly unqualified.

## Linux release workflow and retained failures

The [Linux success excerpt](linux-contracts-41546165642-success-excerpt.txt) and
[complete job/step status](ci-status-41546165642.json) retain the actual source
checkout and executed outcomes. The first release test stage passed 42 tests:
19 library, 10 adapter-live, one bandwidth-profile, eight campaign, one recovery
and three host-pack tests. Its binary harness ran zero tests and contributes
nothing to that count. The eight-test campaign includes the exact-readback
16 MiB whole-mixed case.

The shared conformance stage separately passed 90 tests, including object-v1
and canonical adapter suites. The legacy/v1 synthetic comparison validator,
strict Clippy, default guest compilation and synthetic measurement analyzer
also passed. These are distinct, partly overlapping test scopes; their counts
are not pooled with the seven-platform matrix to imply independent experiments.
A successful release-mode loopback campaign is not an actual 16 MiB MVM or WAN
witness, and its test-run wall time is not transfer-performance evidence.

The previous [Linux run 41539620010](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41539620010)
failed its debug 16 MiB campaign with `Expired` / `object endpoint refused operation`.
Its [failure excerpt](linux-contracts-41539620010-failure.txt) and original JSON
checkpoint remain unchanged. The new successful release-mode run does not erase
or relabel that failed debug execution. The still earlier
[run 41535679640](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41535679640)
failed with the obsolete preparation configuration; it remains a separate
historical failure.

## Permanent records and archive integrity

- [Current machine-readable summary](summary-41546165676.json)
- [Current original platform artifacts](official/41546165676/)
- [Current archive checksum inventory](official-checksums-41546165676.sha256)
- [Current native job/runner IDs and timestamps](ci-status-41546165676.json)
- [Previous 358-execution checkpoint](checkpoint-41539620012.md),
  [summary](summary-41539620012.json), [artifacts](official/41539620012/) and
  [checksum inventory](official-checksums-41539620012.sha256)
- [Original 330-execution checkpoint](checkpoint-41535679868.md),
  [summary](summary.json), [artifacts](official/41535679868/) and
  [checksum inventory](official-checksums.sha256)

The prior clock-expanded checkpoint used MVM dependency `5766754` at merge
`104187b956190c01351b10e0530ce928a66620df` and passed 350 portable plus eight Unix
storage tests. The original checkpoint used merge
`7506b3f3a40ee7dc190d665031750d336fd65aeb` and passed 322 portable plus eight Unix
storage tests. Neither historical run is rewritten into evidence for `ae12`.
Raw timings from these checkpoints are not pooled.

All **152 files listed by the current seven original manifests** pass SHA-256
verification. The additional inventory pins all **159 original files**, including
the seven checksum manifests. Current artifact content totals 259,998 bytes;
there are no payload stores, binaries, build outputs, credentials or full
environment dumps in this archive.

Original Windows CRLF bytes remain untouched. Verification normalizes only the
checksum parser's line endings. The scoped [.gitattributes](.gitattributes)
preserves text bytes and intentional log whitespace under `official/**`.
The additional inventory uses LF so it verifies every unchanged file directly:

```sh
shasum -a 256 -c official-checksums-41546165676.sha256
```

Windows and Unix raw checkout hashes of text manifests/lockfiles can differ;
those original identities remain recorded. Locked builds and clean tracked-lock
checks succeeded on each native platform.

Native MVM backend evidence, real WAN pairs, Windows durable storage, FlowMux,
owner access and additional OS versions retain separate acceptance gates. Fleet
capacity, aggregate budgets, long-lived state, fairness, backpressure, recovery,
saturation and tail latency remain the [scale release gate #49](https://github.com/R4-Laboratories/SpiralCore/issues/49).
Two guests or simulated endpoints do not establish fleet capacity or authorize
larger host budgets. No energy, throughput, send-only time or security
certification claim follows from these CI logs.
