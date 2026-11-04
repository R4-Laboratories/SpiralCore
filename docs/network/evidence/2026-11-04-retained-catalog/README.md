# Original retained telemetry ownership — development checkpoint

2026-11-04. Part of [mailbox integration #43](https://github.com/R4-Laboratories/SpiralCore/issues/43)
and [capacity #49](https://github.com/R4-Laboratories/SpiralCore/issues/49), under
the complete [MVM delivery #42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
This is an installed-service component checkpoint. The shipping profile remains
inactive; no guest, transfer, WAN campaign or cryptographic qualification is
established by these tests.

## Implementation

Product source is [bd83ba66](https://github.com/R4-Laboratories/SpiralCore/commit/bd83ba66f1bdb1ae538aa4b34a04f4be3afa66c8),
tree `90e2bfe7ebdc4d17a67e4dd92e53755689525f0e`, based on the earlier
`f7d06da6` capacity checkpoint. Twelve changed files include the actual
supervisor-owned fixed table, original endpoint callbacks, bounded durable
catalog, disposable/retained split, tests and dependency pins.

The same aggregate resource pool retains R=8 MiB of canonical ciphertext and
K=32 KiB of logical sidecars per original endpoint scope. Disposable object
storage receives checked C−R−K from total C. A fixed metadata/header reservation
comes from that same pool. Dropping a scope does not refund its retained debt;
incomplete inventory Drop quarantines its original reservations and file pins.
The fixed table holds at most sixteen entries. These are logical managed limits,
not measured physical disk blocks, process RSS or fleet capacity.

The original handoff checks its existing registration, directory pin, guard and
projection. Canonical file binding checks private ownership, identity, links,
size and records read/write append access. Binding and sealing repeat directory
privacy/ACL/filesystem checks; sealing rechecks descriptor access. The actual
producer still must supply those same original files through the canonical
collector path before native activation.

Catalog append/update compares the complete published set with the live held
records. A synced monotonic denial beacon is written through the original
installation owner inode before catalog or marker effects. Any nonempty owner
file prevents generic empty-debt epoch transitions, including after catalog
loss and process exit. The beacon grants no recovery, exit, deletion or new
admission authority, and is never cleared by this implementation. It is not a
guarantee against a hostile same-UID actor truncating authoritative files.

The catalog is **plaintext sensitive metadata**. Its checksum detects canonical
corruption; it is not encryption, authentication or authority. Canonical records
and wrapped-key bytes remain opaque to this owner. A valid endpoint-retirement
receipt permits retained sealing, not telemetry deletion or application
completion.

## Actual executed sequence

All builds and executions below ran on Wick, with one Cargo job and no local
Mac build target. Original failure records are retained.

| Recorded phase | Observed result |
| --- | --- |
| `api-red` | Missing inventory API; release build exits 101. |
| `all-git-first` | Coherent SDK dependencies compile; Root fails on missing typed I/O error conversion. |
| `all-git-io-fix` | Build succeeds; fixture fails before catalog execution because its parent was not explicitly private under the host umask. |
| `all-git-private-fixture` | Explicit 0700 fixture; first retained-debt ownership test passes. |
| `lost-catalog-red` | Actual missing-catalog append succeeds incorrectly; test exits 101. |
| `lost-catalog-restart-red` | Actual nested child exits without Drop; restart incorrectly retires an empty epoch after catalog loss; test exits 101. |
| `write-only-fd-red` | Actual write-only records descriptor is incorrectly bound; test exits 101. |
| `continuity-green-directory-red` | Catalog-loss cases pass; records refusal reaches a new changed-directory case, which fails because binding still accepts 0755. Four entries pass, one fails. |
| `retained-faults-green` | Final release build succeeds; five entries pass, including one environment-gated helper and a parent that executes the actual nested restart child. No ignored entries. |
| `retained-broad-strict` | Same protected binary: 152 library entries pass, two helper cases ignored; scoped release Clippy exits zero with three acknowledged unconnected-production-constructor warnings. |

The focused entries overlap the broad suite; do not sum them. Expected caught
panic diagnostics in the broad suite are retained. The ignored helper cases
were not executed by this invocation. No fake native retirement receipt was
constructed.

The final source snapshots match before/after execution. All **134 supplied
compiled source/manifest/lock inputs** match the product Git tree. The protected
test executable remains on Wick outside Cargo's overwriteable target. The graph
uses one MVM SDK revision
[`9a3a964b`](https://github.com/Casey-allard/mvm/commit/9a3a964bd540d5e74c71f9b7df93cc5593295546),
adapter
[`9153dfe2`](https://github.com/R4-Laboratories/SpiralCore/commit/9153dfe20be9518a2277519c312dd0709664610d)
and shared transfer core `ca5d9ca1`, with no local dependency overrides. The pin
change preserves all 419 package names/versions; this does not transfer unrelated
intermediate SDK test acceptance to the final service.

Independent [Code/Rust/Security successor review](code-rust-security-successor.md)
passes the exact corrected source; its reviewer separately verifies all 134
inputs against the Git candidate. The [privacy successor](privacy-successor.md)
passes the narrow source corrections, while retaining native macOS ACL,
production and compliance exclusions. The initial HOLD reports remain alongside
the successors. Review does not substitute for execution or broaden its scope.

Reproduce the release library scope from the product commit on Linux:

```sh
cargo +1.97.1 test --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib -- --test-threads=1
cargo +1.97.1 clippy --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host,canonical-host --lib --tests -- -D clippy::all
```

Original commands additionally used the already-fetched graph with `--offline`.
The records include each exact command, source maps, exit statuses, original
failures, output and executable digest. The compressed archive contains 109
members, 451,917 bytes, SHA-256
`da395d86542121f509f5059240231cff989528777d449d84a867b29716fedb3a`.
`archive-members.json` binds every member. Extract into an empty directory for
inspection; extraction does not execute a test or stored command.

## Remaining release work

Idempotent recovery after incomplete I/O, closed-debt adoption and authorized
retention disposition remain open. The child must deduct the same K, the strict
key/records producer must pass the actual original descriptor, and the shipping
controller must own all resulting processes and files. Positive native endpoint
retirement, installed one-/two-guest recovery, long sessions, capacity/fairness,
native macOS ACL execution, final platform/WAN acceptance and the concrete
FIPS/CUI deployment profile remain separate gates. This component supplies no
new byte, CPU, latency, energy or transfer-speed measurement.

During this checkpoint, user-requested local storage cleanup removed six exact
owned disposable payload/tool-cache directories: 2,829,705,216 allocated bytes.
Historical exact-readback receipts were checked against Git before deletion;
this was not a fresh payload reread or a claim those payload stores were remotely
archived. All sibling evidence, captures, source, executables, raw coverage and
saved sessions were preserved. `owned-storage-cleanup.json` records the precise
paths, evidence and filesystem observations. Heavy builds remain on Wick.
