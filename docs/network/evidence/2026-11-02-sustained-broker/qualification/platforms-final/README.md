# Final source native-platform CI

Both workflows passed for PR #52 head
`6257930015c68d663958e208f42143d045aa1507`:

- [Portable native clients: run 41579824897](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41579824897):
  **72 executed tests on each of seven rows = 504 passes**, plus **eight Unix
  storage passes**. Every selected test was listed, executed and passed, with
  zero failed, ignored or filtered tests.
- [Linux host contracts: run 41579824894](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41579824894):
  **47 release host/component/example tests and 112 shared conformance tests**
  passed, along with strict warnings, default Linux guest compilation and the
  analyzers. The 112 include eight UOR controls plus 104 adapter tests. Scopes
  overlap other suites; do not count all passes as unique independent experiments.

The actual executed checkout was merge
`3866c1eb3c559835a750e93fadc8620552d3f9f7`, with parents `95fc3c45` and `6257930`.
Its tree is `c5cc520b63d74f4b4dbd2f4b6eb51b920904d6f6`, exactly the head tree.
Product files under `crates/` are byte-identical between measured/covered source
`d940203` and this head; the intervening commits contain documentation/evidence.

## Original artifacts and verification

The seven unrepacked ZIPs retain **229 original members**, including **222
inner SHA-256 manifest entries**. All seven API ZIP digests and all inner hashes
passed [verification](verified-artifacts.jsonl). No duplicate extracted artifact
trees are retained. [The verifier](verify-original-zips.sh) checks paths, source,
run, native architecture, Rust 1.97.1, test counts and pinned MVM dependencies.

Rows executed Ubuntu 24.04 x64/ARM64, macOS 26 Intel/ARM64, Windows Server
2022/2025 x64 and Windows 11 ARM64. Actual runner images, rustc native hosts,
process/OS architectures, Git source, manifest and lock hashes are in each
original `identity.json`, preserved inside its ZIP. Rustc hosts and both
architecture fields match the selected native targets. These are native client
processes, not WSL or cross-compilation evidence.

Windows checkout text has CRLF line endings. Its actual manifest/lock hashes
are `503d2d0aeee8ed5643e848126d105f7ea3776600a68a49c951e8246fedc45164` /
`617c3e54c5217ee896ecca8d26fd4fc93abc06ede61fedd1554e8fd264223a7d`.
Other rows record LF hashes
`d4f219fc906c01b075e5ce78f84156884f49f96d2d0d9f1dae2a25c228a52997` /
`b611a2dcabad117ddd0a410d71eae8299b0f4c4e1c8f490fbc6beaf076ee1622`.
Both variants were reproduced from the pinned Git source. Original bytes remain
unchanged; only manifest/log parsing normalizes CRLF and an optional BOM.

The Linux job publishes no workflow artifact. Its
[original job/step status](linux-jobs.json), [run status](linux-status.json) and
[derived execution excerpt](linux-execution-excerpt.txt) retain the executed
source, actual test-result counts, eleven MVM dependency pins and successful
build/check summaries. The excerpt is explicitly a filtered log, not a complete
raw job log. Portable core/contract metadata and Linux's eleven-package closure
all pin `ae12e9deb41cc9a3264196ce22ebc72d4042883d` from the public contribution fork.

These CI runs qualify the declared ephemeral client and host-contract scopes.
Actual guest boots are in the separate native pilot packet; direct/relay WAN,
Windows durable storage, fleet budgets and certification remain open.
