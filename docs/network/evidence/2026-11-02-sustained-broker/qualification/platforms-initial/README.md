# Initial broker-source native CI checkpoint

The initial PR #52 source **d53cb335** passed all seven native ephemeral client
rows: **72 tests per row, 504 portable passes**, plus **eight Unix durable
storage checks**, for **512 recorded passes**. The separate Linux job passed
**43 release host/component tests** and **112 shared conformance tests**, strict
warnings, default guest compilation and the synthetic analyzers.

This checkpoint precedes RawMixed and its failure recorder. It must not be
relabeled as the final later source, native guest execution, WAN qualification
or performance evidence. The scopes overlap; their counts are not pooled into
scientific trials.

- [Portable run 41577442243](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41577442243)
- [Linux run 41577442224](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41577442224)
- [Derived summary](summary.json), [native row records](verified-artifacts.jsonl),
  [portable job/step status](portable-status.json) and [Linux status](linux-status.json)

## Exact source and platform scope

Every original identity records executed synthetic PR merge
`18e7a6c42c73ed1ec7ad71731240710f5ed0621a`. Its tree
`92bc316796bb8e7194d08c98e96df26da34abd79` equals head
`d53cb335ee3884644bfe1a26296923c1288fa540`. Linux checkout logs record that
executed merge. Rust is pinned at 1.97.1; the canonical MVM dependency remains
the contribution fork at `ae12e9deb41cc9a3264196ce22ebc72d4042883d`.
The eleven actual Linux MVM dependency packages are in
[linux-dependencies.json](linux-dependencies.json).

| Native row | Recorded OS | Native compiler target | Portable / storage passes |
|---|---|---|---:|
| Ubuntu 24 x64 | Ubuntu 24.04.5 LTS | x86_64-unknown-linux-gnu | 72 / 2 |
| Ubuntu 24 ARM64 | Ubuntu 24.04.5 LTS | aarch64-unknown-linux-gnu | 72 / 2 |
| macOS 26 ARM64 | macOS 26.6.2 | aarch64-apple-darwin | 72 / 2 |
| macOS 26 Intel | macOS 26.6.1 | x86_64-apple-darwin | 72 / 2 |
| Windows Server 2022 x64 | Windows build 10.0.20348 | x86_64-pc-windows-msvc | 72 / not run |
| Windows Server 2025 x64 | Windows build 10.0.26100 | x86_64-pc-windows-msvc | 72 / not run |
| Windows 11 ARM64 | Windows build 10.0.26200 | aarch64-pc-windows-msvc | 72 / not run |

All twelve portable targets have passing counts equal to listed counts and
zero failed, ignored or filtered cases. The five sustained-probe tests are
functional short-profile tests; they are separate from the registered real-time
load and recovery measurements. Windows qualifies ephemeral process/client
contracts here, not a durable object gateway or native MVM backend.

The [Linux excerpt](linux-execution-excerpt.txt) is derived from actual job output,
with display-color markup removed and only source/build/test summaries selected.
It is not an unmodified full job log. The Linux workflow uploads no artifact;
job status, dependency stdout and the excerpt are retained instead.

## Original artifact integrity

The [seven original, unrepacked ZIPs](original-zips/) retain **229 original
files**. Each ZIP matches its GitHub API SHA-256 digest in
[artifact-api.json](artifact-api.json), and all **222 inner manifest entries**
match their original content hashes. No extracted duplicate stores, binaries or
payload datasets are retained. Original Windows line endings and UTF-8 markers
remain intact inside the ZIPs.

The verification parser normalizes only manifest CRLF/BOM and the single-object
PowerShell storage count representation. It rejects unexpected file paths and
failed, ignored or filtered test-count gates.

```sh
bash verify-original-zips.sh
shasum -a 256 -c SHA256SUMS
```

The first command rederives the retained JSONL records from the original ZIPs.
The second verifies the compact packet itself.
