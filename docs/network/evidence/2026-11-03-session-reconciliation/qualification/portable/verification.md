# Final hosted source and artifact custody

2026-11-03. **PASS** for source head
`b27fbba9bdc636575266d6da04ec8f1ad5c7c2ea`, portable run
[41677859307](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41677859307)
and Linux run
[41677859354](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41677859354).
Both completed successfully at attempt one. Original run/job API JSON, original
Linux job log, seven original portable ZIPs and independent verification scripts
are retained. This collector launched no workflow, build, test, measurement or
guest and canceled no workflow.

The seven original API downloads total **195,352 bytes**. Every archive's
SHA-256 and length match its artifact API digest/size; all **236 inner listed
checksums** pass. Member checks accept only flat regular files. Original ZIPs
remain in `archives/`; only this verifier's duplicate extracted members were
removed after checksum/source/count verification. `artifact-api.tsv`,
`custody.tsv`, `archives.sha256` and `extracted/*.checksums.txt` retain custody.
Windows CRLF was accommodated only when parsing checksum manifests and logs;
stored bytes were never normalized. Windows LF/CRLF projections of the tested
manifest and lockfile match the same Git blobs. Both canonical MVM packages
resolve to the manifest-pinned `ae12e9deb41cc9a3264196ce22ebc72d4042883d` source.

Every native row identity and the Linux checkout log identify actual test merge
`7c10e0f063d1b513881b3ee3d4c9975504d90e11`, with parents
`e984d2d64cdf0483d50d6d59319276c85e7df644` and the head above. Its complete tree
`2dc76d48038762670314ac929c4dda171f042a80` equals the reviewed head tree;
the full merge-to-head diff is empty. The exact object was fetched without
changing checkout, index or FETCH_HEAD. See `source-tree.txt`,
`tested-manifest.json`, `source-projections.tsv` and `rows.json`.

Each row's 13 target listings, JSON counts and actual test-log summaries agree:
**95 passed per row**, including `service_tcp` 14, `call_send` 6 and `receipt` 21.
Failure/ignored/measured/filtered counts are zero. Native Rust 1.97.1 host,
expected target, process architecture and OS architecture match each tested
manifest row; these identities are observed, not inferred from cross-compilation.

| Row | Actual OS description | Native Rust host | Passed | Separate Unix storage |
| --- | --- | --- | ---: | ---: |
| ubuntu24-x64 | Ubuntu 24.04.5 LTS | x86_64-unknown-linux-gnu | 95 | 2 |
| ubuntu24-arm64 | Ubuntu 24.04.5 LTS | aarch64-unknown-linux-gnu | 95 | 2 |
| macos26-arm64 | macOS 26.6.2 | aarch64-apple-darwin | 95 | 2 |
| macos26-intel | macOS 26.6.1 | x86_64-apple-darwin | 95 | 2 |
| windows2022-x64 | Microsoft Windows 10.0.20348 | x86_64-pc-windows-msvc | 95 | Not executed |
| windows2025-x64 | Microsoft Windows 10.0.26100 | x86_64-pc-windows-msvc | 95 | Not executed |
| windows11-arm64 | Microsoft Windows 10.0.26200 | aarch64-pc-windows-msvc | 95 | Not executed |

That is **665 native adapter-client/component TCP-loopback executions**, plus
**eight separate Unix storage executions**. The matrix-generation job is not
an eighth native row. Windows durable storage and the new Windows live
reconciliation consumer remain **NOT_RUN** under the recorded profile/storage
gate. These rows establish no native Windows MVM backend, WAN, fleet, energy
or performance result.

The original Linux log independently derives **111 live release tests** in
11 result scopes (two zero-test scopes), **130 shared adapter tests** in
22 scopes (two zero-test scopes) and **eight object-v1 tests** in one scope.
Shared total is **138 = 130 + 8**. All failure/ignored/measured/filtered counts
are zero; the four zero-test scopes are retained rather than counted as nonzero
gates. Strict warnings-denied Clippy, default guest compilation, published MVM
source verification and both synthetic analyzer/negative suites pass. The
step conclusions, original commands and independent counts remain in `../linux/`.
These stage/platform test scopes overlap and must not be summed as independent
coverage or runtime qualification.

`../initial-failure/` preserves the original Windows Server 2022 fixture failure,
its API/job log and verified original ZIP, plus the superseded successful Linux
run. The approved fixture-only correction starts its short write cap after
canonical frame authorization; no product change, larger timeout or weakened
partial-byte assertion is credited. Final qualification comes from the new
head and runs above, not the canceled/partial initial portable run. Source PR
merge, publication review and full C8/parent-roadmap acceptance remain separate;
closed-stream resume, general ACK-loss recovery and wider platform/WAN gates
are not completed by this custody PASS.
