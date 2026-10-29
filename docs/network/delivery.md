# SpiralCore object-transfer delivery

Current approved sequence: preserve local originals; establish origin/main; extract the pinned Rust object store; harden transaction/closure/limits/durability; implement the standalone exact object-transfer profile; measure real two-machine WAN transfers; then return to MVM's canonical encrypted mailbox and cooperative policy-controlled streams.

## Source and scope

- SpiralCore audit base: `23380f00afb78a5712306f0001c6ad7499d5fbaa` (372 files).
- UOR extraction: `b1b1ef6f0a5d59d8198207c198431701fff5fd63`, crate tree `d1392e09e07200009a6b50b6f09067c2ef8a853b`.
- MVM audit base: `330abfa19ee30c33d444f14918669362f0d51f93`.
- Original local runtime and sessions were copied and SHA-256 verified before Git initialization. They remain outside the tracked import.

## Current priority

The user prioritized working standalone SpiralCore over MVM integration, then requested real WAN transfers with byte, elapsed-time and compute measurements. The local engine and a real two-machine HTTPS relay campaign now have exact-byte evidence. MVM runtime, direct-peer/shaped-WAN qualification and compliance remain separate gates.

## Milestones

| Milestone | Status | Acceptance |
|---|---|---|
| Preservation and origin/main checkout | Complete | Original seven files preserved and backup hashes match |
| Standalone compatibility import | Complete | 54 retained tests, exact-byte example and loopback fleet; immutable vectors unchanged |
| Strict precommit, closure repair, bounded decoding and durable save | Complete for experimental core | 71 tests pass including adversarial, guarded restart, process-exit lock release and read-once pinned input |
| SHA-256 object profile and standalone endpoint | Implemented and tested | Reviewed contract, scoped trusted authority, raw/zstd/DAG, private durable cache, retries/cancel, real TCP CLI and restart equality |
| Canonical two-MVM encrypted mailbox | Deferred by user priority; upstream prerequisites still open | Actual guest service/transport, no replacement port or plaintext host shortcut |
| Cooperative FlowMux streams | Deferred | Existing policy gates and byte ordering preserved |
| Real two-host WAN relay experiment | Exact object fidelity and protocol-body measurements verified | Original 906 operations plus 1,140 efficiency-campaign operations; 30 paired trials per 1 MiB arm; elapsed time and both endpoint CPU measured |
| Efficient exact updates | Implemented and measured | Direct canonical builder, retained verified state, ordinary repeat hits and versioned sparse COPY/LITERAL; warm body savings and end-to-end improvements with seed costs retained |
| Complete per-transfer IP accounting | Qualified only for the new prefix campaign's runner-to-relay leg | Prefix capture counters and ownership agree; other captures retain attribution/counter limitations; no whole-path all-wire-byte acceptance claim |
| Direct/shaped WAN and three-peer qualification | Not run | Controlled impairment, recovery across three peers and eventual MVM boundaries |
| CUI/FIPS deployment qualification | Not claimed | Concrete validated module/OE/mode and contract-specific control assessment |

## Performance priority

The user explicitly prioritized fewer bytes over the link, allowing additional computation at either endpoint. Exact reconstruction is mandatory. Transferred bytes are the primary performance acceptance metric; CPU, memory and elapsed time remain measured tradeoffs. A slower loopback mutation is not a failure if it materially reduces transmitted bytes. The earlier 2x latency target is a later optimization target, not a gate for the first working delivery.

## Invariants

`Decode(verified_receiver_state, recipe, residual) == original_bytes`, including original length. Object identity is independent of representation. Every record and complete reconstruction must validate before publication or commit acknowledgment. Cache existence does not grant authority. Resume revalidates principal, recipient, namespace and policy/execution epoch. Cards and source code are never executable network recipes.

Cold content uses raw/zstd bytes; warm content uses batched missing-record exchange or versioned sparse COPY/LITERAL recipes. The current sender is synchronous; further pipelining is a later optimization. The sparse encoder shares the same identity and commit contract. Legacy UOROBJ01 remains explicitly distinct from the full-SHA256 transfer profile. Cache, staging, message, output, work and concurrency budgets are separate and reserved before admission.

The mailbox profile uses at most 64 KiB application bodies, a 64 MiB object ceiling, four active transfers per peer within a shared 1 GiB cache quota and 512 MiB resident budget. Initial mandatory demonstration: objects through 16 MiB. Larger-object claims require qualification. Object commit, mailbox admission, delivery and application processing are separate events; no exactly-once execution claim.

## Acceptance matrix

Exact equality: empty, NUL/trailing zero, chunk boundaries, append/truncate, insert/delete, localized/scattered changes, compressed bytes and ciphertext. Negative cases: malformed/oversized/truncated frames; missing or evicted descendants; corrupt/conflicting records; scale/padding faults; wrong recipient; revoke/epoch/replay; restart/cancel/crash before and after publication. No failed transfer may expose a committed object.

Performance: compare raw, zstd, conventional CAS and rsync/VCDIFF over identical authenticated transport and durability. Time encoding through durable verified commit. Count both directions, metadata, requests, receipts, security, repair, retransmission and relay overhead. Report seeding cost, cache/storage, CPU/RSS and latency distributions. Use 30 randomized paired trials per acceptance workload, proper bidirectional impairment, then real direct/relay hosts and MVMs.

Targets, not achieved claims: warm repeated/localized-change 1–16 MiB objects save at least 90% total bytes versus the same secured raw baseline; halving median duration is a secondary future optimization; cold incompressible >=1 MiB incurs at most 5% extra bytes; report p95 duration and CPU separately rather than rejecting useful byte savings solely for extra compute. Report small objects, scattered edits and prefix insertion separately; compare against the best conventional baseline before claiming a SpiralCore-specific benefit.

## Delivery and external prerequisites

Deliver source/provenance, tests/vectors, protocol/threat decisions, claim ledger, adapter, reproducible demo/harness, measured receipts and an explicit gap list. MVM mailbox service/transport work (#4629/#4633) owns the canonical binding. Existing FlowMux/vsock 5253 is not a mailbox port. Experimental Iroh may remain outside a first validated-crypto deployment. HeatNet/vnet lacks a pinned implementation; historical HEAT.NET is only a possible reference.

Use public/synthetic data until deployment qualification. Enumerate envelope, transport, storage, identity, signing, RNG and key-management cryptography and bind qualification to exact module/version/operating environment/approved mode. A crypto-provider interface or SHA-256 algorithm name is not FIPS validation.

## Current runnable delivery

Use `crates/spiralcore-transfer`. Its original `spiralcore` CLI remains a loopback-only public/synthetic-data experiment, now with explicit sparse sends. The separate WAN binaries use a temporary bearer capability, allowlisted experiment scopes and verified HTTPS through Cloudflare. This bounded synthetic-data adapter is a working Internet transfer experiment, not a general public file service or a production security qualification. The existing warm-cache/wire prototypes remain separate research implementations. Geometric peer selection and MVM integration remain future work.

The release demo transfers 1 MiB cold/repeated/32-byte-mutated content over real TCP, then reopens the cache and checks exact bytes. A separate 16 MiB random cold/repeat run also passed exact recovery. See [validation receipt](validation.json) for source identities, measured bytes, complete test/coverage scope and limitations.

The [real WAN report](wan-results-2026-10-29.md) records the subsequent GitHub Linux → Cloudflare → Mac campaign. Warm DAG mutations reduced HTTP-body bytes substantially but were slower than full raw transfer on that path. Ordinary content-addressed exact-repeat hits reduced both bytes and time. Cold random and prefix-insertion DAG expansion remain explicit negatives. Neither the earlier smoke's original `capture_complete` flag nor a green workflow establishes complete packet accounting; the report retains the later counter reassessment.

The [subsequent efficiency report](efficiency-results-2026-10-29.md) records direct graph construction,
prepared state and sparse recipes at `b5520171ab1cfbc988e16b2b79f41ab831d3668a`. Sparse warm updates
save 99.80–99.98% of protocol-body bytes and complete 1.83–2.58 times faster than the current full-raw
arm in matched trials. Seed-plus-update gains are smaller and explicitly measured. Raw/zstd currently
warm a DAG synchronously while sparse publication skips it, so equal durable-object fidelity does
not imply equal future DAG readiness. Exact repeats also match ordinary CAS. These results do not
establish superiority over a matched rsync/VCDIFF WAN implementation or energy savings.
