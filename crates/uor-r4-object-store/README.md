# Exact byte object store

Standalone Rust extraction of UOR-R4's ordered 32-byte, ternary reconstruction DAG. Repeated content shares records; unique content may expand. This compatibility implementation preserves UOROBJ01, original 12-byte BLAKE2b identifiers, manifest encoding, and fixed known-answer vectors. It does not authenticate peers or provide a production security profile.

`Store::fold` writes immutable records. `range` verifies visited records against a caller-pinned manifest; `rebuild` also checks exact length and whole-object SHA-256. A digest is not authorization. `verify` checks available closure and must not be described as a fresh full-object digest check.

```sh
cargo test --locked --manifest-path crates/uor-r4-object-store/Cargo.toml --all-targets
cargo run --locked --manifest-path crates/uor-r4-object-store/Cargo.toml --example exact_bytes
cargo build --release --locked --manifest-path crates/uor-r4-object-store/Cargo.toml --target-dir target --bin objpeer
OBJPEER="$PWD/target/release/objpeer" bash crates/uor-r4-object-store/tools/fleet_maintenance.sh
```

Commands run from the repository root with Rust 1.89 or newer. The example performs byte/data-structure work and optionally accepts an output path to exercise save/restart. The fleet helper starts only loopback listeners and removes its temporary files. Legacy helpers use ordinary shell tools and Python 3 for timing/JSON assertions; the crate and its test vectors do not require Python.

The extraction excludes tokenizer bindings, model consumers, CUDA and training machinery. Dependencies are SHA-256 and test-only JSON parsing. See [protocol](PROTOCOL.md), [fixture provenance](tests/fixtures/README.md), [MIT license](LICENSE), and [import identities](../../docs/provenance/object-store-import.json).

The [delivery record](../../docs/network/delivery.md) distinguishes compatibility import, subsequent hardening, the new mailbox profile, and unrun MVM/WAN acceptance. No historical timing is presented as a fresh result.

Peer pulls stage immutable records and validate the full cached/staged closure, declared scales,
canonical padding, exact length and whole-object digest before committing records. Cached parents are
traversed, so an evicted child is fetched again. Framing rejects partial headers/bodies and nonempty
DONE. `PeerLimits` offers separate message, staged-count/bytes, cumulative-work, request and in-flight
budgets; existing public APIs and default storage limits remain available.

For a read/modify/save lifecycle, acquire `StoreWriter::open(path)` before loading and keep the guard
until saving finishes. `Store::save(path)` provides ownership for that save alone. Saves stream to an
owned temporary file, sync it, atomically replace the canonical snapshot, and sync the directory.
The persistent `.uor-lock` sidecar must not be unlinked. Restart validates the canonical snapshot and
ignores abandoned temporary files. This is cooperative local ownership in a trusted directory. Crash-durability assumes its directory
ancestry was durably provisioned; CLI-created ancestors are not individually synced.

Legacy DONE acknowledges a verified in-memory commit. Disk persistence is separate, and a failed DONE
write can leave a valid committed cache with uncertain peer receipt. A directory-sync error after
rename likewise requires reconciliation; neither error is silently described as a rolled-back commit.
These semantics are distinct from the new authenticated object-transfer profile.
