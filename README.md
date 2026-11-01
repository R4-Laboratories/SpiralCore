# SpiralCore

SpiralCore networking prototypes, exact geometric transport research, and preserved experimental evidence. This repository combines a curated snapshot of work from [UOR-R4](https://github.com/UOR-Foundation/uor-r4) with owner-supplied SpiralCore source materials; it is not the UOR-R4 language-model repository.

## Start here

- [Shared Rust gateway and Linux TUN results](docs/network/gateway-results-2026-11-01.md): ordinary IPv4/IPv6 TCP and UDP traffic now crosses paired authenticated gateways. All 48 application flows passed exact verification, with complete bidirectional captures. Packet compression saves about 41.8% of mixed-session IP bytes but raises CPU and local completion time; warm-object reuse remains shared with MVM. Includes the runnable TUN configuration, raw receipts and an immutable capture/executable archive.
- [MVM overhead and exact-repeat optimization](docs/network/mvm-overhead-results-2026-11-01.md): 36 actual two-guest sessions, 324 exact transfers and reboot readbacks. Exact-repeat CPU/time improve; an explicit 64 MiB/s mailbox profile removes much of the inherited rate wait. At that same profile, the seven-object 16 MiB workload uses 64.4 MB and 3.291 s with Warm versus 192.9 MB and 6.777 s with prepared zstd. Default 4 MiB/s, non-repeat regressions, shared-host contention and CPU-accounting limits remain explicit.
- [Actual MVM/vsock integration and results](docs/network/mvm-vsock-results-2026-10-31.md): real Linux guest→host and two-guest transfers under a small native HVF orchestrator; 100 exact accepted objects and 50 exact reboot readbacks. The matched 16 MiB two-guest workload saves 66.62% of encrypted stream bytes and completes in 16.03 s versus 43.85 s for prepared whole-object zstd. Exact-repeat losses, the inherited 4 MiB/s limit, CPU accounting differences and local-only scope are explicit.
- [Ongoing optimization ledger](docs/network/optimization-ledger.md): source-pinned experiments, recorded results, regressions and the next active measurement plan.
- [Unrelated-object CPU and mixed WAN results](docs/network/unrelated-cpu-results-2026-10-31.md): exact allocation/scan changes preserve 600-byte warm transfers and all selected bytes across 1,400 local and 420 WAN operations. WAN unrelated sender CPU improves about 14–15% against the previous selector; mixed sessions send about 48% fewer body bytes than prepared zstd, with higher CPU. Local large-object gates and some WAN controls fail, so this remains a measured candidate, not a performance-accepted optimization.
- [Artifact storage and restoration](docs/network/artifact-storage.md): compact results stay in GitHub; large historical WAN snapshots are retained in a private Hugging Face dataset with immutable revisions and checksums, so verified local copies can be removed.
- [Run standalone SpiralCore](crates/spiralcore-transfer/README.md): `serve`, `send`, `get`, and a real TCP cold/repeat/delta demonstration with exact restart verification. Reducing transmitted bytes is the primary performance objective.
- [Sender selection, residual and hashing CPU results](docs/network/sender-cpu-results-2026-10-30.md): prepared large generated-transfer CPU fell from 111.437 to 35.039 ms and local durable elapsed time from 127.094 to 49.678 ms, preserving the 747-byte body. All 840 operations reopen exactly and all runtime frame comparisons match. Edited content improves substantially; unrelated-content and small-case regressions, preparation and cold costs remain explicit. These are local measurements, not new WAN or energy results.
- [Packed replay and prepared-state CPU results](docs/network/replay-cpu-results-2026-10-30.md): large-trace replay CPU fell from 228.305 to 12.184 ms. Complete local generated-transfer CPU fell from 559.601 to 139.071 ms, then to 117.050 ms with sender preparation reused. All 720 campaign operations reopen exactly, with unchanged 747/766-byte favorable bodies. Preparation, retained memory, small workloads and conventional controls are recorded; this is a local CPU comparison, not a new WAN result.
- [Coupled ternary/Kuramoto/CA transfer results](docs/network/shared-runtime-results-2026-10-30.md): the retained runtime is in the Rust transfer path, with 480 exact local operations and 72 exact Linux-to-Mac WAN operations. A 14.16 MiB generated trace uses 747 protocol-body bytes; the WAN pilot beats raw elapsed time but loses to zstd and uses more CPU. The [runnable Rust example](crates/spiralcore-transfer/README.md#shared-ternary-kuramoto-and-cellular-automaton-replay) and full evidence retain these tradeoffs. Core compute optimization precedes MVM integration.
- [Byte-first codec selection, October 29](docs/network/codec-selection-results-2026-10-29.md): opt-in Rust API chooses Raw, chunked zstd, sparse or zstd prefix before Begin. All 240 local paired inputs matched minimum configured body cost and all 1,800 operations reopened exactly. CPU penalties and paid seeds are recorded; these are local results, separate from WAN qualification.
- [Two-machine WAN experiment](docs/network/wan.md): authenticated synthetic transfers through a temporary HTTPS relay, paired raw/zstd/cache/DAG workloads, endpoint CPU and durable-commit timing, and packet accounting.
- [Broad 16 MiB WAN campaign, October 29](docs/network/broad-wan-results-2026-10-29.md): four methods with matched verified-byte publication policy, including a conventional zstd shared-base delta control. All twelve registered cases are independently reviewed: 2,640 exact operations, with CPU, elapsed time, cold controls and conventional delta comparisons. Read the report for exact byte, elapsed, CPU, paid-seed and capture results.
- [Earlier 16 MiB WAN results, October 29](docs/network/large-wan-results-2026-10-29.md): thirty paired localized edits, 180 exact reopened operations, 16,814,026 to 600 HTTP-body bytes, 8.73× paired elapsed and 3.17× endpoint-CPU improvement over the then-current raw path, which included optional DAG warming. Seed-plus-update improves 1.81× in elapsed time. The third capture's three-packet counter gap remains an explicit limit on full-campaign IP accounting.
- [Earlier 1 MiB efficiency and WAN results](docs/network/efficiency-results-2026-10-29.md): direct graph construction, retained verified state and sparse exact deltas; 1,140 successful operations, 99.80–99.98% warm protocol-body savings and 1.83–2.58× paired elapsed improvement, including computation. Cache seeding and packet-accounting limits are reported separately.
- [Receiver-cost follow-up](docs/network/receiver-cost-results-2026-10-29.md): 256 exact controlled local updates; the small quota-scan refactor missed its preregistered CPU target and was reverted, with all negative and positive observations retained.

- [Hardware SHA-256 result](docs/network/hardware-sha-results-2026-10-29.md): the opt-in backend passed all three registered CPU gates on native Apple M1, with 256 exact local updates and unchanged protocol bytes. Sparse CPU fell from 16.360 to 4.402 ms without unrelated cached objects. All raw observations and independent reviews are retained; this is separate from the WAN measurements.
- [Direct quota-scan profile](docs/network/cache-profile-results-2026-10-29.md): 32 exact diagnostic operations attribute 84.39% of populated-cache sparse-operation CPU to the two quota scans. No production scan shortcut was selected; filesystem and corruption checks remain intact.
- [Earlier WAN baseline](docs/network/wan-results-2026-10-29.md): 906 successful operations with exact persisted-byte verification; the measured compute costs and negative cases that motivated the efficiency work.
- [Original compute audit and efficiency plan](docs/network/compute-efficiency-2026-10-29.md): baseline graph/verification costs, retained-state and incremental-update design, and a separate hardware-SHA feature-build diagnostic.
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
- `crates/spiralcore-transfer/`: versioned SHA-256 object transfer, durable scoped cache, bounded raw/zstd/DAG/sparse/zstd-prefix and shared-runtime decoding, coupled ternary/Kuramoto/CA replay, prepared Rust APIs, loopback CLI and bounded WAN experiment harness.
- Other directories under `crates/`: SpiralCore-related model-source references and integration points, not a complete model build.
- `source-manifest.tsv`: source-relative paths and source Git blob IDs for the 344 UOR-R4 files.
- `import-manifest.tsv`: origins, byte counts, and SHA-256 hashes for the 18 attachments, four extracted files, and webpage.

The copied research retains its original paths and wording. Links to unselected UOR-R4 context are intentionally preserved; use the source repository for the complete surrounding project.

## Visibility and licensing

This repository is public at the owner's explicit request. The object-store crate retains its source MIT license; this does not grant a license to the other imported research or owner-supplied artifacts. See [provenance](PROVENANCE.md).
