# Closed retained-debt adoption: first component

2026-11-04 development checkpoint for [mailbox service #43](https://github.com/R4-Laboratories/SpiralCore/issues/43)
and [capacity #49](https://github.com/R4-Laboratories/SpiralCore/issues/49).
Product source: [325f662b](https://github.com/R4-Laboratories/SpiralCore/commit/325f662b41bf1c5fbcc9ce978fc7c22d8fa4c070),
tree `ca50f0f932ce65f8608ba691269decc15217ecb0`, based on `8476d758`.
This restores closed custody and accounting. **It does not restart useful
service, issue a VM/network grant, or complete the recovery release gate.**

## Implemented boundary

The private recovery controller consumes the actual installation owner and
retains both owner EX and fresh holders EX. Its initial compatibility allowlist
supports one precisely identified previous Linux/x64 profile. The trusted
canonical parent is an actual pinned directory File; catalog paths cannot
select arbitrary filesystem roots.

One closed authority owns one aggregate ledger. Fixed bookkeeping is charged
before bounded catalog parsing; every known historical row imports its full
8 MiB records plus 32 KiB sidecar ceiling atomically. Observed file length is
not substituted for the reservation. A protected phase-2 row without records
or a wrapped key remains fully charged unresolved debt.

No ordinary reserve operation is enabled, even temporarily. Recovery creates
no old grant, `RetainedTelemetryScope`, endpoint-retirement receipt or native
issuer. Its snapshot reports closed managed usage and unresolved entries.
Successful adoption retains original catalog, state, marker and any observed
files. Refusal and unwind retain the complete bounded controller, fixed
observation slots, exclusions and funded debt. Refused observations remain
private and confer no authority. Fresh revalidation candidates do not replace
accepted pins before identity comparison.

The catalog remains plaintext sensitive metadata with a corruption checksum,
not encryption or authentication. Wrapped keys remain opaque. V1 has no
historical key-content digest; this code does not invent one or decrypt keys.

## Actual test sequence

All compilation/execution ran on Wick with one Cargo job per command. The
immutable SDK graph remains MVM `9a3a964b`, adapter `9153dfe2` and transfer core
`ca5d9ca1`, with the existing locks and no local dependency overrides.

| Phase | Observed result |
| --- | --- |
| Previous-product fixture | Unchanged `8476d758` product bodies plus test-only scaffolding create actual phase-2 debt and exit via `process::exit(0)`. Catalog, marker and beacon remain; no Rust Drop or VM receipt supplies finality. |
| API absence | Actual release compilation fails with E0432 for the two missing recovery types and E0599 for `prepare_retained_recovery`. |
| Initial closed adoption | Actual previous-profile fixture passes: full R+K/bookkeeping, changed source profile, unchanged files and closed owner exclusion. |
| Malformed-marker regression | Actual late validation drops an already-qualified state pin: test exits 101. Early arming repairs that selected case; the test and fresh old-profile positive pass. |
| Independent acquisition review | HOLD identifies remaining local-descriptor lifetimes before controller arming. The original report is retained. |
| Five acquisition regressions | The same tests fail before the fix, then pass after it: corrupt-catalog observation, late-current holders loss, oversized marker, post-open state privacy and explicit post-holder unwind. |
| Final protected regressions | 153 ordinary library entries pass; nine cases are ignored by that invocation. The five acquisition cases, previous malformed-marker case and fresh old-profile positive pass in separate explicit invocations. Scopes overlap. |
| Scoped Clippy | Release `--lib --tests -- -D clippy::all` exits zero with 38 ordinary unused-import/dead-code messages for the inactive recovery code. The test build separately emits two ordinary warnings. No zero-warning or `-D warnings` claim. |

The late-current and unwind cases use explicit test fault injection. Fixtures
are real processes/filesystem objects, **not booted MVMs**. The source contains
classification for all four durable phases, but these positive executions
qualify phase 2 only. Phase-1/3/4 positive matrices, 16-row saturation and
additional funding/poison fault cases remain unexecuted here. Funding-error
retention is source-supported, not a separately executed result.

All **135 supplied compiled inputs** match the Git product tree and remain
unchanged across the final run. Historical and fresh fixture hash maps also
match. The final protected executable SHA-256 is
`d22faf6febf65a30564f8617d1bdf5f80a7f4effd614f75517065b779b1acc12`.
It remains on Wick outside Cargo's overwriteable target. The protected previous
fixture executable is `45ddf9d61f3bdb853f554788645aa3a176aac9294712f7da482755d33e996659`.
Its old test body was reconstructed by removing only the later API test and
verified against the original compiled-input SHA; this is explicitly a verified
reconstruction, not a claim it was copied before modification.

[Independent successor review](successor-review.md) passes the narrowly scoped
acquisition correction and preserves the [original HOLD](initial-review.md).
The exact test module is identical in RED and GREEN. The [author response](author-response.md)
maps each finding to its fix and executed evidence. The reviewer read actual
records and source; it did not rerun tests. Reviewer roster authorization was
local to this chat and supplies no regulatory or FIPS certification.

## Records and reproduction scope

`receipts.tar.gz` is 438,893 bytes, SHA-256
`9284f7fc035119b7e985e451a9ab505b9c26c200ff28f162d1cfd446bc376e27`.
All 196 regular relative-path members are bound by `archive-members.json`.
Commands, source maps, exit statuses, outputs, original failures, test overlays
and fixture readbacks remain intact. Extract into an empty inspection directory;
extraction executes nothing. No normal/instrumented executable, raw VM image,
payload store or local Cargo target was added to the laptop.

The ordinary Linux scope is reproducible from the product commit:

```sh
cargo +1.97.1 test --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib -- --test-threads=1
cargo +1.97.1 clippy --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib --tests -- -D clippy::all
```

The ignored fixture tests require the explicitly protected previous-product
executable/state described in their recorded commands. Ordinary test success
does not imply those tests ran. The original commands used already-fetched
dependencies with `--offline`.

## Required continuation

The next stage is the separately versioned, debt-preserving generation/profile
transfer in the [original plan](original-plan.md). It must preserve old origin
identity and files, import all old ceilings into the same aggregate ledger,
survive transaction-barrier crashes, and allow only fresh canonical admission.
It must also distinguish durable shutdown with retained debt from zero-debt
completion or deletion. Codec/private-format fixtures cannot replace an actual
sealed row from the canonical producer and endpoint lifecycle.

P2 diagnosis, observed-byte/reason snapshots and same-controller retry remain
unfinished. Useful customer restart, authorized retention disposition, the full
phase/fault matrix, original producer/backend integration, sustained sessions,
capacity/fairness, supported native platforms, real WAN and FIPS/CUI deployment
qualification remain open under the complete MVM delivery goal. No new bytes,
CPU, latency, energy or transfer-speed measurement follows from this checkpoint.
