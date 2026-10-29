# SpiralCore

SpiralCore networking prototypes, exact geometric transport research, and preserved experimental evidence. This repository combines a curated snapshot of work from [UOR-R4](https://github.com/UOR-Foundation/uor-r4) with owner-supplied SpiralCore source materials; it is not the UOR-R4 language-model repository.

## Start here

- [Run standalone SpiralCore](crates/spiralcore-transfer/README.md): `serve`, `send`, `get`, and a real TCP cold/repeat/delta demonstration with exact restart verification. Reducing transmitted bytes is the primary performance objective.
- [Two-machine WAN experiment](docs/network/wan.md): authenticated synthetic transfers through a temporary HTTPS relay, paired raw/zstd/cache/DAG workloads, endpoint CPU and durable-commit timing, and packet accounting.
- [Measured WAN results, October 29](docs/network/wan-results-2026-10-29.md): 906 successful operations with exact persisted-byte verification; substantial warm payload savings, compute costs, negative cases, and explicit packet-accounting limitations.
- [Compute costs and efficiency path](docs/network/compute-efficiency-2026-10-29.md): measured graph/verification costs, retained-state and incremental-update design, and a separate hardware-SHA feature-build diagnostic.
- [Exact Rust object store](crates/uor-r4-object-store/README.md): independently buildable byte-object storage, ternary DAG replication, `objpeer`, and pinned compatibility vectors.
- [Delivery plan and implementation status](docs/network/delivery.md): staged storage hardening, MVM integration, WAN acceptance, and qualification gaps.
- [SpiralCore v69 page](index.html): a self-contained copy of the Netlify page, ready for a future GitHub Pages deployment from `main` / root.
- [Owner-supplied reference files](references/): all 18 attachments, both original ZIPs, and the meaningful payload extracted from `spiralcore.zip`; see [`import-manifest.tsv`](import-manifest.tsv) for origins, sizes, and SHA-256 hashes.
- [Wire transport](projects/spiralcore/wire/README.md): Python and Rust adapters, socket layer, routed hub, multiplexing, process interposer, benchmarks, and bounded loopback evidence.
- [Warm-cache networking](projects/spiralcore/warm-cache/README.md): Kuramoto time, Merkle manifests, WarmWire/WGR1 frames, WMX1 multiplexing, WarmSession, and TCP/QUIC prototypes.
- [SpiralCore/FBS mechanism guide](docs/mechanisms/spiralcore-fbs.md): exact signed-axis transport, ternary-tree binders, experiments, limitations, and source links.
- [Latest full-rung FBS record](docs/labs/spiralcore-fbs-offline-float-sheet/README.md): the completed four-arm, two-seed offline-float experiment and its receipts.
- [Provenance and export scope](PROVENANCE.md): source identity, the 344-file source manifest, the 23-file additions manifest, history limits, and exclusions.

## Status and claim boundaries

The standalone wire layer uses UORADP01 framing with raw or zlib-compressed data. It does **not** yet integrate SpiralCore/FBS routing. Its historical timings are loopback measurements, not general WAN or production claims.

The warm-cache work is a prototype. Existing records include TCP/QUIC loopback results, emulated-WAN tests, and a remote TCP/SSH campaign; direct QUIC WAN was blocked by the test pod's port policy. Those historical results are preserved, not freshly reproduced by this export.

The latest full-rung offline-float FBS experiment is a **negative for one configuration**, not a verdict on SpiralCore as a whole. Learned routing lost to frozen routing on the registered top-1 comparisons, and trained-query results did not reproduce between byte-identical builds. No useful language capability is claimed by this repository.

## Build and test the standalone networking crates

```bash
# Working standalone transfer demo (choose a fresh cache path)
cargo run --release --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --bin spiralcore -- demo --cache local/first-demo/cache

# Exact object store (from the repository root)
cargo test --locked --manifest-path crates/uor-r4-object-store/Cargo.toml --all-targets
cargo run --locked --manifest-path crates/uor-r4-object-store/Cargo.toml --example exact_bytes

# Wire transport
cd projects/spiralcore/wire/crates/wire-rs
cargo test --locked
cargo build --release --locked

# Warm-cache prototype
cd ../../../warm-cache/rust/warm-cache-proto
cargo test --locked
cargo build --release --locked
```

The wire smoke tools live in `projects/spiralcore/wire/tools/`. The imported object-store crate supplies `objpeer`; microVM checks still require their actual runtime and guest services.

## Repository layout

- `references/`: the 18 owner-supplied attachments and the four meaningful files extracted from `spiralcore.zip`.
- `index.html`: the self-contained SpiralCore v69 webpage for optional future GitHub Pages hosting.
- `projects/spiralcore/`: complete tracked standalone project tree, including reference notes.
- `docs/`: current research guides, pre-registrations, task records, lab records, and receipts.
- `archive/`: preserved research reviews, evidence, historical patches, and archived GitHub discussions.
- `crates/uor-r4-object-store/`: standalone exact object storage and peer protocol, with its own MIT license and source provenance.
- `crates/spiralcore-transfer/`: versioned SHA-256 object transfer, durable scoped cache, bounded raw/zstd/DAG decoding, and loopback CLI.
- Other directories under `crates/`: SpiralCore-related model-source references and integration points, not a complete model build.
- `source-manifest.tsv`: source-relative paths and source Git blob IDs for the 344 UOR-R4 files.
- `import-manifest.tsv`: origins, byte counts, and SHA-256 hashes for the 18 attachments, four extracted files, and webpage.

The copied research retains its original paths and wording. Links to unselected UOR-R4 context are intentionally preserved; use the source repository for the complete surrounding project.

## Visibility and licensing

This repository is public at the owner's explicit request. The object-store crate retains its source MIT license; this does not grant a license to the other imported research or owner-supplied artifacts. See [provenance](PROVENANCE.md).
