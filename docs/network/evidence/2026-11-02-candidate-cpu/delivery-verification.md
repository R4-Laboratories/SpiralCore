# Results delivery verification

Implementation PR31 merged as `531acdded6c1dedcd844e5ee7b26ce4650c52793`
after both executed workflows passed on measured source
`343248065628176f76841e98608a4a4badf747dd`.
Baseline source is pinned `4b8cb79fe5fca9e9527a2923924c887d9e7ddbf8`.

- Candidate run41399314329: success,18sessions264operations216exact reopened
  deliveries, all before/after receipt-parity gates passed. Linux15core/44gateway
  tests, strict Clippy and both release builds passed before timed traffic.
- TUN regression41399314515: success,12sessions48exactflows, complete captures.
- Original candidate ZIP matches GitHub API SHA-256
  `f8750a9a1441145b03a6c41723a3abe886a4f0420303bf5800d3cb11e140e821`.
  Root verified904artifact entries,165candidate source entries,151baseline source
  entries and4frozen binaries. Baseline source hashes were compared with the
  pinned baseline Git blobs, not the intentionally changed current checkout.
- Executed merge `eeaf4b766d7e4fced83c2be47bafb851b9a5e04a` has candidate
  workflow blob `1385d2add3175ad42f8f157e6f96eba098a7f835`, identical to measured
  source. The quick workflow blob also matches source;862artifact entries,
  44source entries and2binaries verify against its API-digest-matching original ZIP.
- Both original network ZIPs plus the frozen local baseline profiler were
  archived to the existing private dataset. Independent fresh forced immutable
  downloads with Xet disabled matched size, SHA-256 and bytes. Archive receipts
  distinguish root manifest/source verification from independent storage custody.
- Frozen Rust analyzer exited0 on full original data. The compact Git-bound
  campaign reproduces byte-identical JSON SHA-256
  `69b6efd388c8acfd418e528c05a52f079ca1aab26ef8b5631c22a00a933efef2`.
  Independent arithmetic reproduced session, operation and cumulative joins.

Results modify documentation/evidence only. Original raw output whitespace is
retained deliberately; source/document checks exclude raw receipts. Full captures
and binaries remain archived rather than copied into Git. No other lab or user
saved-session files are changed or removed. Full local owned copies are eligible
for cleanup only after all these custody and reproduction checks.
