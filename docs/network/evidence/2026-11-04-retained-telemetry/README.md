# Mandatory telemetry capacity before endpoint effects

2026-11-04. A prerequisite of the installed retained-storage work in
[#43](https://github.com/R4-Laboratories/SpiralCore/issues/43) and
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).

Code candidate `63c87961d3da4c8eba25733cef5c3e8cd681bccf`, tree
`ff2d657a72dd5dd0472729d403f785cca97b7f4e`, parent `7a080af1`.
The immutable dependency graph remains MVM `bef4146c`, adapter `7f638428`,
shared core `ca5d9ca1`. All 131 supplied inputs match the candidate's Git
blobs. No path override or dependency update was used.

The original endpoint projection's cache allowance is TOTAL capacity,
including canonical telemetry's mandatory 8 MiB records cap. The previous
parent admitted a synthetic 64 KiB total allowance. The new check refuses that
profile before endpoint issuance, managed reservations or disposable directory
effects. Available aggregate quota cannot enlarge an original projection.
Positive private fixtures now include 8 MiB plus their 64 KiB allowance;
these synthetic numbers do not activate a native resource profile.

| Executed scope | Result |
| --- | --- |
| New actual parent preparation regression, previous product code | Release build 0; test 101: undersized total was admitted |
| Corrected locked release `installed_parent::` scope | 23 PASS, 2 ignored, 0 failures |
| Corrected strict release Clippy, library and test targets | Exit 0, zero emitted warnings |
| Source and protected test object checks | Before/after match; all 131 inputs match Git |
| Independent scoped Code/Rust/Security source review | PASS |

The regression checks refusal, `NeverIssued`, unchanged aggregate usage and
unchanged installation-directory names. The full parent scope overlaps earlier
checkpoints; counts across RED/GREEN iterations must not be summed. Both real
canonical-helper cases remain ignored in this ordinary run; their historical
executions retain their own source and binary identities. No guest or helper
process is started by this scope.

Reproduce the ordinary scope from this complete checkout:

```sh
cargo +1.97.1 test --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib installed_parent:: \
  -- --test-threads=1
cargo +1.97.1 clippy --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib --tests -- -D clippy::all
```

The original commands, manifests, test failure, corrected output and object
hashes are in `capacity-receipts.tar.gz`: 110,583 bytes, SHA-256
`f251287e8bbbcb8b34e58024af3831814377eba9ffb4c084e683106451a3c0cb`.
All 31 indexed files plus the checksum index verify. Product/test executables
and build caches remain on Wick and are excluded. No raw payload or VM disk
was copied locally. The RED source manifest was captured before execution;
the archive contains corrected source bodies and both phases' source hashes,
not a separately retained copy of the complete RED source tree.

## Remaining integration

This fixes the pre-effect capacity refusal only. The disposable inventory
still receives TOTAL capacity; the checked disposable/retained split, original
retained-file owner, durable catalog, deletion protection, restart adoption and
native service consumer remain required. The shipping profile is inactive.
See the [implementation contract](../../../specs/2026-11-04-retained-telemetry-budget.md)
and [continuing plan](../../../plans/2026-11-04-retained-telemetry-budget.md).

The final upstream MVM contribution remains open. There is no new transfer,
byte/CPU/WAN measurement, positive native retirement, capacity envelope or
FIPS/CUI qualification in this policy checkpoint.
