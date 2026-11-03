# Retained Call identity checkpoint

Start with the [results, costs and remaining gates](../../persistent-session-identity-results-2026-11-03.md).
Product `91baf28` merged in PR #56 at `0e02dd7`. The full goal remains active.

- `registration.json`: pre-performance sizes/order/resources and explicit scope;
  functional RED/GREEN checks preceded this performance registration.
- `source-sha256.txt`: all 276 tracked crate files exported to Wick, checked
  before normal execution and again around coverage.
- `host-pairs/`: 22 original normal Rust probe receipts, both directions, setup,
  role ThreadCPU, aggregate ProcessCPU, verified time, RSS and exact readback.
- `native/`, original `native-receipts.tgz` and `native-members.sha256`: three
  original public Firecracker receipt sets, 34 verified archive members. Private
  bootstrap, launch folders, cache payloads and disks are excluded.
- `native-cleanup.txt`: three verified successful stores removed after custody.
  File lengths are apparent bytes, not measured freed filesystem blocks.
- `validation/`: observed RED failures, fixture/lint failures and final executed
  test/build/Clippy output. `call-send-red-test.rs` preserves the missing-API test
  source (`f471b03d...`); the original field-name compiler output was not archived.
- `coverage/`: exact isolated commands, binary/source hashes, line hits, covered
  and uncovered changed production, and all 46 test summaries.
- `qualification/`: successful seven-native-client and Linux CI receipts,
  original ZIPs and digest/inner-file verification. These do not prove MVM/WAN
  execution on those client platforms.
- `reviews.md`: source, normal executable and review identities with limits.

Verify every packet member except the root manifest with
`shasum -a 256 -c SHA256SUMS` from this directory. Publication document hashes
refer to the documentation PR snapshot; check out that revision to verify them
after subsequent README updates. Preserve historical originals and losses.

The 2,048-Call witness cap and pipeline one remain. No generic post-send retry,
both-peer event recovery, full retention soak, host-wide plaintext/cache
aggregation, installation exclusion, canonical WAN or final upstream readiness
claim follows from this packet.
