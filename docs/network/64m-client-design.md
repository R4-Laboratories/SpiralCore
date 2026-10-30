# Bounded 64 MiB transfer: source architecture proposal

Recorded 2026-10-29 from live source `c58efe2970219f38f5ebfac2eb9eec165c29baa7`. This is a design, not implementation or a measured 64 MiB result. No production changes, builds or timing runs were performed for this proposal. The running frozen 16 MiB qualification is a separate activity.

## Decision

First extend **Raw, ordinary per-chunk Zstd, and SparseV1** through an explicit opt-in 64 MiB client profile. Retain modes 0, 1 and 3 and all existing wire/identity semantics. Keep existing public batched helpers at their 16 MiB default. Do not enable 64 MiB by merely changing `MAX_BATCHED_OBJECT`: replace eager full-object frame materialization with a one-batch-at-a-time sender, and select sparse recipes from bounded operation descriptions before copying literals.

Use `ColdGraphPolicy::Disabled` consistently for the initial 64 MiB comparison. Preserve Endpoint/WAN quota defaults, authorization, exact ObjectId checks, private ownership, pending-disk reservation, fresh Begin/Finish scans and publication fsync/rename rules. Keep 20 MiB HTTP batches, 512 frames per batch, 60 KiB literal chunks, 64 KiB complete frames, 4,096 sequence/ledger entries and the existing 64 MiB object ceiling. A smaller local sender batch setting can be qualified later; it must not silently change a comparison.

Keep the current **mode-4 prefix profile at 16 MiB**. A 64 MiB whole-prefix profile is feasible in the receiver's present memory envelope, but requires a separately versioned mode and encoder-memory qualification. Do not enlarge mode 4's published v1 bounds silently. General 64 MiB DAG/prepared transfer and file-backed receiver streaming are later work, not prerequisites for the first bounded Raw/Zstd/Sparse qualification.

## Current constraints traced to source

| Area | Current behavior | Consequence |
|---|---|---|
| `lib.rs`, `protocol.rs` | `MAX_OBJECT=64 MiB`; Raw/Zstd/Sparse Offer permits it | Receiver wire already supports these object sizes |
| `client.rs` legacy sender | 64 MiB ceiling, exchanges individual frames | Existing source support is not a comparable large WAN benchmark |
| `client.rs` batched helpers | 16 MiB ceiling; build all Data bodies before `BatchSender::plan` slices uploads | Raising the ceiling alone retains another full object's worth of frames |
| `delta.rs::encode` | Compares simple and aligned recipes after packing both into owned literal bodies | Two near-full literal candidates can coexist before raw fallback |
| `wan.rs::encode_batch` | Copies encoded frames into a growing HTTP-body Vec | Length cap is not an exact allocated-capacity/RSS bound |
| `wan.rs::HttpClient::exchange` | Retains request body while reading and decoding bounded response bytes | Account for both serialized and parsed representations |
| `endpoint.rs::begin` | Reserves output, base, optional graph work and ledger before substantial allocation | Keep this admission boundary, including the temporarily removed active attempt |
| `endpoint.rs::publish` | Writes complete raw publication, syncs file, reauthorizes, renames, syncs directory/root | COPY saves network bytes; it does not avoid full materialization or durable write |
| `endpoint.rs::read_object` | Returns a fully verified owned Vec to the caller | Returned storage becomes caller-owned, not free endpoint memory |
| `zstd_prefix.rs` | v1 base/target16 MiB, encoded cap16 MiB+64 KiB,512 chunks,32 MiB window | Not a64 MiB profile |
| Current benchmark analysis | Requires exactly one HTTP request per row | Must be extended with an explicit new campaign schema and expected batch plan |

The source inspection includes the full active Begin/data/Finish path, batch builder/parser, sparse planner/decoder and publication/read paths. The allocation figures below are source-derived reservations or estimates; none is a process-memory measurement.

## Client API and bounded preparation

Add an explicit options/profile entry point, sharing implementation with existing helpers. For example, validated `ClientProfile::wan16()` and `ClientProfile::bounded64()` select an object ceiling; `transfer_batched_with_profile` and `transfer_sparse_batched_with_profile` use it. Existing APIs call `wan16()`. The64 profile initially rejects DAG and both prefix modes until their own qualification. The receiver independently applies its wire and resource policy; sender options cannot raise receiver limits.

The sender owns at most one pending upload batch and releases it after verified replies. It retains enough immutable request data to repeat that same batch explicitly after an uncertain transport outcome, but does not automatically change mode or base after Begin. Assign sequence numbers as frames enter the current batch, rather than preconstructing every frame in the transfer. Validate the batch's count and byte size before exchange. Reuse the existing strict transfer/sequence/Ready/Accepted/Published checks.

For sparse preparation, retain at most two candidate lists of bounded `COPY(base_offset,length)` / `LITERAL(target_offset,length)` descriptions. Preserve the existing prefix/suffix and aligned comparison algorithm and8192-operation cap. Compute exact packing/traffic cost without copying literals, select a candidate or Raw before Begin, and materialize selected literal slices only into the current batch. Simulated packing and emitted packing must share one checked rule set; differential tests must show the same valid recipe or fallback as the existing implementation. No public caller-created plan becomes a receiver trust object.

Immutable input bytes remain borrowed for the entire operation. A caller must supply a consistent snapshot; a pathname or dirty log is not proof of stable contents. Target/base hashes and comparisons still scan these bytes. Do not add a file-path API that can change content after hashing without specifying snapshot ownership.

Precompute exact batch encoded length and reserve once before filling the HTTP body. Avoid repeated temporary `Frame::encode()` allocations for size calculation: a checked encoded-length method and bounded append routine can share the existing canonical serializer. Keep the old serializer/wire vectors as the compatibility oracle. This is a proposed allocation reduction; it is not evidence of a speedup.

Bound incoming reply allocation by the requests actually outstanding. For a legitimate64 profile batch with at most one Begin and one Finish, success/error/Accepted replies fit below64 KiB. A generic bound can sum per-request maximum reply sizes, including a bounded Need response if the generic adapter still supports DAG, and clamp to the existing20 MiB transport ceiling. Validate before response allocation, then retain the existing reply-content checks. A malicious relay must not gain20 MiB of response allocation per tiny Data acknowledgment merely because the generic envelope permits it.

## Exact batch arithmetic

For a target-absent, unprobed **64 MiB Raw transfer**, with current framing and20 MiB batches:

- Target bytes:67,108,864.
- Data chunks:1,093 (1,092 full61,440-byte chunks and one16,384-byte chunk).
- Application frames:1,095 including Begin and Finish; comfortably below4,096.
- At most340 full Raw Data frames fit the20 MiB envelope. The greedy batches contain341,340,340,74 application frames, respectively: the first includes Begin and the last includes Finish.
- HTTP operations:4, not1. Request bodies total67,180,123 bytes and response bodies total74,598 bytes on the all-success path:67,254,721 combined body bytes. These are arithmetic from SCWBAT01/SCWREP01 and application framing; they exclude HTTP headers, TLS, TCP/IP and retries.

Ordinary Zstd compresses each60 KiB chunk separately and can use fewer batches, depending on encoded lengths. Sparse can encode a single64 MiB COPY span in one operation, but final output is still64 MiB. Its operation, literal, output and total-frame limits all remain checked. Every plan must check its final frame count before Begin; a pathological candidate that exceeds limits chooses Raw before transmission or returns Limit.

Keep a fresh HttpClient/Agent per object operation. Multiple batches and TCP connections within that operation are permitted, and their bytes/time/CPU accumulate into the same row. Do not share a connection across different measured operations without changing capture ownership rules. Extend analysis to validate the exact planned multi-batch count/observed retries; do not simply remove the existing one-request assertion and accept arbitrary counts.

## Sender memory accounting

Introduce an application-owned allocation budget for the new bounded sender, with a proposed256 MiB logical ceiling for Raw/Zstd/Sparse. Charge visible retained input lengths, pending frame bodies/capacity, serialized request capacity, reply buffers and operation descriptions before allocating them. Borrowed input length is a conservative ownership charge, not knowledge of the caller's Vec capacity or total process heap. All capacity arithmetic is checked; allocation failure returns Limit, not a partially accepted result.

| Component | Proposed bounded Raw/Zstd/Sparse peak |
|---|---:|
| Target immutable input | up to64 MiB |
| Base immutable input, sparse only | up to64 MiB |
| Current batch bodies | at most20 MiB plus bounded frame metadata |
| Serialized request | at most20 MiB, reserved to checked exact length |
| Candidate sparse descriptions/packing scratch | conservative1 MiB allowance, replacing two encoded literal candidates |
| Expected reply bytes, parsed frames and bookkeeping | conservative1 MiB allowance after request-derived reply cap |
| Per-chunk cold zstd context/work | qualify pinned implementation separately; no unreviewed large window |

The table's owned payload/metadata subtotal is about170 MiB for sparse and106 MiB for raw, before a qualified cold-codec allowance. It is not an RSS prediction. TLS/HTTP internals, native codec tables, allocator retention, thread stacks, shared libraries and caller state require independent peak-memory observation. A256 MiB owned-allocation budget does not certify a256 MiB process limit.

Do not carry the current eager sparse implementation into a claimed low-memory64 profile: two candidate literal payloads alone can approach128 MiB of logical bytes, before target/base, frame capacities and HTTP copies. Similarly, raw frames for the whole64 MiB object are avoidable retained copies. Eliminate those structural copies before debating unsafe buffers, SIMD or allocator micro-optimizations.

## Receiver accounting and unchanged guarantees

With optional cold DAG warming disabled, the current source reservations for maximum64 MiB target/base are:

| Mode/path | Reserved inside endpoint resident ceiling |
|---|---:|
| Raw or ordinary Zstd | `2*64 MiB +2 MiB` =130 MiB |
| SparseV1 with64 MiB base | `2*64 +2*64 +2` =258 MiB |
| Future whole-prefix64, provisional | `2*64 +2*64 +64.25 +4 +2` =326.25 MiB |

WAN endpoint allowance stays448 MiB, with the existing64 MiB transport allowance outside it. Defaults elsewhere remain512 MiB resident,512 MiB per namespace and1 GiB cache. These are reservation/admission limits, not measured process RSS. Three raw reservations can fit before other state; two maximum sparse reservations cannot. Four is an active-count upper bound, never a promised concurrency level. Graphs, terminal ledgers and active reservations continue to compete for the same endpoint allowance.

The current transport simultaneously holds a request body and decoded Frame bodies, up to about40 MiB logical bytes at the20 MiB request cap, plus bounded response/header work. Confirm capacity and native-codec peaks before treating the64 MiB allowance as sufficient. An exact-size publication read plus stack EOF probe is preferable to relying on generic `read_to_end` growth behavior when tightening allocation accounting; such a change needs its own corruption/growth regressions, not a silently reduced reservation.

Default eager raw warming would reserve another128 MiB at the node cap, making a64 MiB cold raw attempt258 MiB. Cold DAG admission reaches418 MiB at the default staging cap; a warm DAG adds base-read/graph charges and can exceed448 MiB. Random64 MiB DAGs also exceed the existing unique-node limit. Do not promise general64 MiB DAG support because the top-level object-length constant permits64 MiB; lower node budgets/readiness or a different admission contract would be separate decisions.

Full ObjectId hashing remains linear in target size for arbitrary changes, and a verified base requires its own read/hash under current trust rules. COPY reconstruction and full publication writes are also linear. No total-computation `O(changed_bytes)` claim follows from a small network recipe. COW, lazy objects, authenticated dirty logs and incremental object identities require separate contracts.

## Prefix64 feasibility, separately versioned

A future experimental mode5 (`ZstdPrefixV2`, name provisional) can bind whole-base/target ObjectIds up to64 MiB, use version2 Data bodies, keep60 KiB chunks and permit at most2,048 encoded chunks. The maximum encoded cap from the pinned `compress_bound` is67,371,008 bytes (64.25 MiB), requiring at most1,097 full-size fragments. The4,096 sequence ceiling remains ample, and transport envelopes stay20 MiB.

Raise only this new profile's advertised window bound to128 MiB (`windowLog=27`) so one complete64 MiB prefix can participate across the64 MiB target. Retain exactly one ordinary frame, required content size/checksum, no dictionary ID, full ObjectId verification and the safe one-shot decoder. The pinned one-shot DCtx/raw-prefix fixed-table argument should still apply independently of window size, but must be rechecked and qualified at the new boundary. Reserve326.25 MiB before work; existing terminal/cache charges may still reject admission.

Sender encoding is the unresolved resource gate. Phase one can simultaneously hold64 MiB base,64 MiB target,64.25 MiB encoded output and match-finding context. The prior128 MiB encoder-context number was an estimate, not an enforced bound, and cannot simply be reused after raising windowLog. Determine a conservative source-backed context bound with fixed parameters, then qualify it. If it cannot fit the registered sender budget, keep prefix64 disabled; do not quietly raise limits or substitute a repeatedly rebuilt dictionary per60 KiB chunk. Drop encoder context before HTTP frame construction, and avoid retaining both the full encoded frame and a second full set of Data bodies.

Streaming compressed input to a decoder is a different memory tradeoff: it can remove the complete encoded buffer but may allocate a large streaming history window. It is not automatically lower-memory than the one-shot path. File-backed encoded/output staging also changes quota, crash and API behavior and belongs to a separately reviewed receiver-streaming phase.

## Timeouts, retry and crash semantics

Keep current30-second idle retention,300-second active lifetime and60-second HTTP exchange deadline initially. At the previously targeted10 Mbps, one20 MiB upload has roughly16.8 seconds of payload serialization time before other work. At5 Mbps it takes roughly33.6 seconds and can exceed the endpoint's idle interval between batches even though the HTTP deadline is60 seconds. This is a real support boundary, not a reason to claim broad WAN support. Qualify the intended rate/RTT envelope; a smaller uniform batch profile may be needed for slower links without weakening timeout rules.

A timeout can occur after durable publication but before the client receives Published. Report an uncertain transport outcome, not proof that nothing was committed. Exact retransmission must reuse the same immutable Offer and request bytes/sequence under the same current authorization. Never reuse a transfer identity with a different mode/base/body. The first64 implementation need not add automatic resume: explicit replay/query/restart is acceptable if documented and tested. A recovery handle can retain DescriptorId, Offer, transfer ID and the bounded last batch, but must not manufacture durable progress from Accepted.

Partial state remains volatile. After receiver restart, Resume of an incomplete transfer returns Unknown, and a caller starts a new authorized attempt with a fresh ID and current base. A complete publication is discoverable and fully reverified. Cancel/expiry release memory and pending reservations; lost acknowledgments do not permit premature application delivery. Reauthorization remains before handling and immediately before durable publish. No streaming or retry path may bypass final full-byte verification.

## Qualification matrix and staged acceptance

1. **Bounded source change.** Add explicit64 client profile, incremental batch builder and description-only sparse selection. Freeze exact source/configuration and retain16 MiB API/wire regressions. Reject64 MiB+1 before preparation/exchange; retain cold/default behavior for existing callers.
2. **Local boundary and resource checks.** Test lengths0,1,60 KiB−1/at/+1, batch boundary−1/at/+1,64 MiB−1/at/+1. For base and target independently cover random, compressible, repeat, sparse edits, insertion/deletion and inefficient raw fallback. Inputs that grow must start below64 MiB. Verify exact bytes after close/reopen; validate raw's deterministic four-batch/body counts. Record logical live reservations and separately measure fresh-process high-water memory on Mac and Linux. A proposed512 MiB process peak is a qualification target, not a current guarantee; failure requires redesign or explicit reconsideration, not an automatic budget increase.
3. **Hostile and recovery checks.** Wrong base/namespace/epoch, corruption and inconsistent header lengths; allocation budgets just below/at required reservation; oversized frame/chunk/count/expansion; overflow offsets and8192-op boundaries; wrong sequence, duplicate/conflicting replay, malformed/oversized response and false Published; revoke during base read and before rename; cancel after each batch; response loss after Data/Finish; receiver restart after each boundary; truncated staged file; disk full, quota-changing external addition/growth/deletion; duplicate target publication; fsync/rename failure. Preserve owned staging cleanup and current filesystem observations. Retain failed cases and ambiguous outcomes.
4. **Small WAN qualification.** First run a bounded pilot with Raw/Zstd/Sparse and no claimed speed gate, after Linux capture lifecycle passes. Bind actual multi-request counts, exact reopened bytes and both endpoint CPU/wall to every operation; include complete seeds and retries. Keep all failures. A64 MiB wire result needs its own capture/body evidence;16 MiB success does not transfer automatically.
5. **Registered performance comparison.** Freeze workloads/order/counts and byte/time/CPU criteria before larger campaigns. Report transfer and seed-plus-transfer costs and all cold cases. Original localized-transfer goals (90% fewer bytes and2× wall speed) can remain hypotheses to test, not architectural promises. CPU can rise even when network bytes/wall improve; report that explicitly. No energy conclusion without an energy measurement.
6. **Separate prefix64 and low-memory receiver work.** Only after the resource proof and hostile/recovery checks are complete should mode5 join an equal-policy comparison. File-backed streaming and64 MiB DAG work remain independent proposals.

Storage constrains campaign layout. At64 MiB, one warm four-arm paired trial publishes eight objects =512 MiB plus headers. Two such pairs exceed the existing1 GiB cache quota. With only three arms, two pairs use768 MiB plus headers and fit; three do not. Use fresh bounded cache groups, never raise the quota to retain a convenient trial count. Preserve exact verification receipts before clearing only experiment-owned synthetic caches; account for retained artifacts and setup/cleanup separately. A30-pair four-arm campaign would otherwise retain about15 GiB of object payload across isolated roots. Choose that storage/runner schedule explicitly after the pilot rather than assuming the current five-shard16 MiB layout applies.

## Later small-MVM/orchestrator adapter

The64 MiB host service does not imply a512 MiB heap is acceptable inside every small VM. Keep MVM integration as a later canonical mailbox/vsock adapter, with explicit process placement and memory/queue budgets. Stable authenticated node and tenant/capability/epoch identity must map through the bridge/orchestrator to current vsock routes; CID reuse is not identity. Bound concurrent objects and queued bytes, propagate backpressure, and specify restart/rebind/revocation and persistent idempotency.

Object Published means exact verified durable object publication. It remains distinct from mailbox acceptance, delivery and application processing acknowledgments in an at-least-once environment. Preserve the encrypted canonical envelope boundary, and do not repurpose FlowMux5253 or add a guest NIC. If small-node budgets require streaming/spooling, implement that adapter/storage contract explicitly rather than making unverified caches or partial chunks visible as objects.
