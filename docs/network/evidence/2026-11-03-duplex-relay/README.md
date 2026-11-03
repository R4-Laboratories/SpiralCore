# Duplex canonical relay checkpoint

[Results and limits](../../duplex-relay-results-2026-11-03.md) ·
[PR #58](https://github.com/R4-Laboratories/SpiralCore/pull/58).
This is the C8 relay prerequisite; both-peer recovery remains unfinished.

- `registration.json`, committed before performance, pins four exploratory
  Linux/Firecracker cells. `after-source.txt` binds reviewed product `f964c034`.
- `final-registration.json`, `native-final.sh`, `final-source-sha256.txt` and
  `native-final/` bind two separately registered final confirmation cells at
  `62cb6042`, after the Darwin error-path fix. The original four-cell comparison
  remains unchanged. The final archive/member manifests bind 27 public files;
  `native-final-cleanup.txt` records only custody-verified owned disk removal.
- `source-sha256.txt` binds all 278 tracked crate files, verified before normal
  execution and before/after isolated coverage.
- `native-linux.sh` launches the existing Rust engine within the approved
  two-guest/two-vCPU/2-GiB/no-NIC profile. It checks normal binaries, static guests,
  source, snapshot ceilings, teardown and counters. It does not delete disks.
- `native/`, `native-receipts.tgz`, `native-archive.sha256` and
  `native-members.sha256` retain all 49 original public receipt members, custody
  hashes and four exact native comparisons. Private bootstrap/launch folders,
  payload stores and VM disks are excluded. `native-cleanup.txt` records verified
  successful owned disks removed only after local archive/member verification.
- `validation/` retains original semantic REDs and executed focused/full
  tests, normal builds and strict Clippy. The EOF RED test source is explicit.
- `coverage/` retains exact commands, four isolated binary/profile/export
  identities, line hits, scoped derivation, uncovered lines and all 61 test
  summaries. Raw profiles and LCOV remain on Wick. Instrumented times provide
  no performance claim.
- `qualification/initial-failure/` retains both failed macOS job logs and the
  native Darwin Rust witness, its output and host identity. Original TCP closure
  assertions remain; the product adds separate-half shutdown fallback.
- `qualification/final-attempt-failure/` retains the subsequent fixture-only
  macOS failures and executed-merge identity. The probe timeout is now configured
  before closure. `qualification/fixture-timeout-order.diff` binds that test
  change; product files remain identical to `62cb6042`.
- `coverage-final/` binds final source `fdfbd2f` and 62 passing instrumented tests;
  protected executable copies remain on Wick. The initial shared-target binary
  paths were overwritten by normal reuse; that limitation is explicit, with
  original observed exports/profile/hash records preserved.
- Hosted qualification receipts and final reviews are added before delivery.

Verify root `SHA256SUMS` from this directory once the publication packet is
sealed. Preserve original measurements and losses. No automatic retry, >2,048
Calls, larger pipeline, host-wide plaintext budget, WAN or final upstream claim
follows from this packet.
