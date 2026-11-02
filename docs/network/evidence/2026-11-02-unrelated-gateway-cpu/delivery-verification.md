# Delivery verification

Implementation PR33 merged as `63541fa8a1be11b3172644e4a8c45a933cdae87e`
after run41446807449 passed on source
`c7ee11fdde26c922da9ae6d9574a91f88b2a3573`. Baseline is exactly
`4a8ec79c9b185c85407e9767653dbcc4af97bf7e`.

- Linux15core/44gateway tests, strict Clippy and both release builds passed before
  traffic. Complete dose18sessions264operations216deliveries; exact reopened
  bytes, all before/after parity checks, complete captures and unchanged routes.
- No network retry or failed campaign. Historical Runtime/Combined/candidate-CPU
  analyses remain byte-identical under the new analyzer.
- Original GitHub artifact13142827886 is27,080,829bytes; SHA-256 matches API:
  `962927de8ba538149ef444b4d0cb31218fb9ca49708e4ac369dced0480a4bee4`.
  Root verifies908artifact entries,167candidate source entries,154baseline entries
  and4executables. Baseline sources compare with baseline Git blobs, not current
  intentionally changed delta code.
- Executed merge `6d53beb08c27f95078f05a15428d3e2094839d6b` workflow blob
  `54ad6d5c5f3f0ffd9de445ffdddb6de887395421` matches measured source exactly.
- Independent archive custody verifies original ZIP size/hash/bytes after a fresh
  forced immutable download with Xet disabled. Both local native profiler binaries
  and all stage results have separate verified immutable custody.
- The compact bundle contains887original public receipt files, excluding captures
  and binaries. Every per-file SHA verifies after extraction; its campaign yields
  byte-identical analysis SHA-256
  `a97182d496dac0173815ee2d5d67f00ebe282489a468e2f0830012c86fde9f02`.
- Independent review recomputes18sessions,264operation joins,252group cumulative
  and252connection-prefix rows, including all96matched operation parity pairs.
  Report numeric/limits review passes at its retained SHA.
- Initial direct analysis used an outdated release executable and exited1.
  Explicit Cargo release build/run from measured current source exited0. Both
  receipts remain; this did not require any campaign, source or data change.

Results change documentation/evidence only. Raw stdout whitespace remains exact.
No new TUN traffic campaign was necessary for the unchanged packet path; platform
integration tests execute in the Linux run. No new Internet-WAN, MVM or energy
claim is made. See local-cleanup.md for exact task-owned scratch relocation after
custody; no saved user work, other labs or shared target/cache is cleaned.
