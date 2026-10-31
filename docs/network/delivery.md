# SpiralCore object-transfer delivery

Current approved sequence: preserve local originals; establish origin/main; extract the pinned Rust object store; harden transaction/closure/limits/durability; implement the standalone exact object-transfer profile; measure real two-machine WAN transfers; then return to MVM's canonical encrypted mailbox and cooperative policy-controlled streams.

## Source and scope

- SpiralCore audit base: `23380f00afb78a5712306f0001c6ad7499d5fbaa` (372 files).
- UOR extraction: `b1b1ef6f0a5d59d8198207c198431701fff5fd63`, crate tree `d1392e09e07200009a6b50b6f09067c2ef8a853b`.
- MVM audit base: `330abfa19ee30c33d444f14918669362f0d51f93`.
- Original local runtime and sessions were copied and SHA-256 verified before Git initialization. They remain outside the tracked import.

## Current priority

The user has now requested finishing this optimization phase and moving toward
MVM integration. Codec policy is frozen at `b2dacc6`: the existing Rust warm
session, schema and coupled ternary/Kuramoto/CA implementations remain active.
The new [canonical MVM adapter](../../crates/spiralcore-mvm-adapter/README.md)
wraps their bounded frames using the real MVM core at
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b`. This is a host conformance milestone;
the dedicated guest mailbox service, live vsock/Iroh, durable mailbox admission,
direct/shaped WAN qualification and compliance remain subsequent gates. Earlier
HTTP WAN results and the new canonical-envelope measurements have distinct scope.

The [first canonical-envelope result](mvm-readiness-results-2026-10-31.md)
records 78/78 exact stopped-store readbacks and fifteen passing adapter tests,
including the existing ternary/Kuramoto/CA path. Complete paid mixed host sessions
use 65.710%/66.608% fewer canonical envelope bytes at 1/16 MiB versus prepared
Whole/Raw. The small unrelated-object CPU penalty, about 22.6 KB of actual warm
envelopes, fixed fixture clock and pending live guest boundaries remain explicit.
This advances integration without reopening codec optimization or claiming WAN
speed from an in-memory host test.

Jeff Venable's [schema-fitting direction](schema-fitting-direction.md) now has an opt-in Rust template profile and a genuine trained-zstd dictionary profile alongside the coupled runtime. The [held-out comparison](schema-fitting-results-2026-10-30.md) records 400 exact target transfers and 240 exact artifact seeds. The template family saves bytes versus raw on matching batches but loses to conventional compression; trained dictionaries help small records conditionally on shared setup. Fitting, seeding, CPU, durable elapsed time and drift/fallback negatives are recorded. These new profiles have local qualification only.

The subsequent [compressed-schema and persistent-receiver result](schema-reuse-results-2026-10-30.md)
adds a compressed exact value/residual stream and reusable sender/receiver codec
state. All 1,600 targets and 120 seeds matched immediately and after reopening.
Matching ~1 MiB batches use 127,557.5 median body bytes versus 130,450.5 with
ordinary zstd, but prepared schema CPU/elapsed are 11.789/20.1545 ms versus
3.846/12.2465 ms for zstd. Reuse retains identical body costs; it has not yet
produced a substantial combined CPU gain. The report includes actual ten-send
setup amortization, retained state and negative controls. No new WAN result is
claimed for these changes.

The [sender follow-up](schema-sender-results-2026-10-30.md) now matches the first
prepared layout directly, avoiding repeated layout allocations and hashing while
preserving every encoded byte. In the paired local run, matching ~1 MiB sender
CPU fell 32.06% by the registered paired metric; median total CPU fell
11.8645→9.5525 ms and durable elapsed 19.841→16.989 ms. All 800 targets and 40
seeds matched immediately and after reopening, with unchanged protocol frames.
Ordinary zstd still uses less CPU and local elapsed time. Full session setup,
drift/fallback controls and zstd timing variation are recorded separately.

## Milestones

The [unrelated-object CPU follow-up](unrelated-cpu-results-2026-10-31.md) removes
avoidable sparse allocation and schema scanning/parsing work. It preserves all
selected frame bytes, 600-byte warm transfers and exact readback across two
700-operation local campaigns and one 420-operation real-WAN diagnostic. WAN
replacement sender CPU falls 14.21%/15.46% at 1/16 MiB against the previous
selector, but Whole still uses less CPU. Both local large-object CPU gates fail,
small WAN warm CPU regresses, and elapsed gains are inconsistent. The candidate
is implemented and measured, **not performance-accepted**. Mixed paid sessions
retain about 48% fewer body bytes and lower elapsed time against Whole in all
six pairs at each size, with more measured CPU. Raw evidence, source identities,
paid setup, instrumentation and every negative are retained in the report.

The [persistent-session follow-up](persistent-session-results-2026-10-31.md)
retains acknowledged base ownership and prepared selector/runtime state, with
bounded optional HTTP reuse. It verifies 660 local and 396 subsequent WAN objects
exactly. WAN localized updates remain 600 body bytes; median completion is
56.0905/106.6970 ms at 1/16 MiB versus 132.8555/1,159.6070 ms for matched
prepared Whole/Raw. Paid seed-plus-ten-update sequences and CPU are recorded.
Sender CPU improves against fresh selection, while large-object outer elapsed
slightly regresses. An initial failed pilot remains unresolved; a separate
66-object diagnostic and the subsequent pilot do not establish a transport fix.
All failures, replacement penalties and scopes remain in the report. Live MVM/vsock
integration and production transport reliability remain subsequent work.

The [setup and compressible-fallback follow-up](selector-setup-results-2026-10-31.md)
preserves the 600-byte warm path and verifies 540 more WAN objects exactly.
Measured setup CPU falls about 21%; compressible-update sender CPU falls 3.44%
against the previous selector. Current 1/16 MiB warm completion medians are
153.0445/212.7585 ms versus matched Whole/Raw 353.2995/1,897.569 ms.
This round does not establish a general WAN improvement over the old selector:
small random-update paired completion regresses, and the highly compressible
case has equal bytes and approximately tied completion versus Whole. Paid seeds,
setup, deceptive-sample CPU costs and every negative remain in the report.

The [bounded-selection follow-up](bounded-selection-results-2026-10-30.md)
now avoids fully encoding a losing small Whole candidate using a source-qualified
first-block bound. All 360 new WAN objects reopen exactly and all warm selector
updates retain 600 body bytes. Small-update sender CPU falls 28.0% in the median
paired comparison with the old selector; completion falls 9.0%, with 20/30 wins.
Current candidate completion medians versus matched Whole/Raw are 67.9955 versus
153.5535 ms at 1 MiB and 141.5105 versus 1,638.849 ms at 16 MiB. The unchanged
large path, compressible fallback penalty, seed/setup charges, small paid-lifecycle
losses and measurement limits remain explicit in the report.

The [integrated warm selector](warm-selection-results-2026-10-30.md) now carries
that verified-base path into automatic Raw/Whole/Sparse/Schema selection. All
360 WAN seed/update objects reopen exactly, with thirty pairs per size and all
sixty selector updates costing 600 body bytes. At 16 MiB, selector median
CPU/completion is 51.178/179.8485 ms versus tuned Whole/Raw 85.8375/1,840.930 ms.
The winning plan is consumed without duplicate encoding/hashing; proven size
bounds skip sixty cold-schema and thirty large Whole candidates. Small-object
selection still costs more CPU than forced Sparse, and the small paid lifecycle
uses more CPU than Whole. Actual seed/setup and all negative trials are retained.

The [matched whole-object control and selector](schema-selection-results-2026-10-30.md)
now have real-WAN evidence: 1,860 exact transferred objects. Whole/Raw wins every
selector decision on that new-object corpus; selection adds CPU without further
byte reduction. This warms codec contexts only and does not test the earlier
verified-base sparse/prefix or shared-runtime mechanisms. The subsequent
[warm-object recheck](warm-object-results-2026-10-30.md) now completes thirty
pairs at 1 and 16 MiB with 240 exact reopened seed/update objects. The existing
sparse path sends 600 body bytes and, at 16 MiB, uses 54.5955 ms median CPU and
201.771 ms completion versus 85.9755 ms and 1,626.617 ms for tuned Whole/Raw.
Actual paid seed-plus-update costs, the one large elapsed loss and all small-case
results remain recorded. This measures prior-object reuse separately from the
schema experiment and changes no production encoder or decoder.

| Milestone | Status | Acceptance |
|---|---|---|
| Preservation and origin/main checkout | Complete | Original seven files preserved and backup hashes match |
| Standalone compatibility import | Complete | 54 retained tests, exact-byte example and loopback fleet; immutable vectors unchanged |
| Strict precommit, closure repair, bounded decoding and durable save | Complete for experimental core | 71 tests pass including adversarial, guarded restart, process-exit lock release and read-once pinned input |
| SHA-256 object profile and standalone endpoint | Implemented and tested | Reviewed contract, scoped trusted authority, raw/zstd/DAG, private durable cache, retries/cancel, real TCP CLI and restart equality |
| Canonical MVM envelope adapter | Implemented host conformance integration; live guest prerequisites still open | Real pinned sealing/opening, bounded authorization, existing transfer engine; actual guest service/transport still required |
| Cooperative FlowMux streams | Deferred | Existing policy gates and byte ordering preserved |
| Real two-host WAN relay experiment | Registered broad 16 MiB campaign completed | Twelve cases, four methods and thirty pairs per case; all 2,640 operations exact after reopening. Body bytes, elapsed time including compute, endpoint CPU and seed costs retained. Earlier 906-operation and 1,140-operation campaigns remain distinct historical records. [Final broad report](broad-wan-results-2026-10-29.md). |
| Efficient exact updates | Implemented and measured | Direct canonical builder, retained verified state, ordinary repeat hits and versioned sparse COPY/LITERAL; warm body savings and end-to-end improvements with seed costs retained |
| Pre-Begin byte-first codec selection | Implemented opt-in; locally measured | All 240 paired inputs choose minimum configured body cost; 1,800 exact reopened operations at 1/16 MiB. Losing encoders add CPU; explicit senders remain available. [Local result](codec-selection-results-2026-10-29.md). New adaptive WAN qualification remains unrun. |
| Sparse candidate allocation pruning | Experiment complete; candidate not adopted | Exact output and diagnostic memory gates pass, but three CPU gates fail. Active encoder restored to the published baseline; source, 140 timing observations, 140 diagnostic observations and all negatives preserved. [Result](sparse-pruning-results-2026-10-30.md). |
| Coupled ternary/Kuramoto/CA transfer | Implemented; local comparisons and real WAN pilot verified | 480 exact local and 72 exact WAN operations. A 14.16 MiB generated/edited trace uses 747/766 body bytes, or 4,630/4,649 with checkpoint seeding. WAN completion improves versus raw, but zstd is faster and runtime CPU is higher. Native ARM64 and Linux x86-64 golden vectors/plans agree. [Result](shared-runtime-results-2026-10-30.md), [contract](../specs/2026-10-30-shared-runtime-design.md). Three-trial WAN pilot, not full acceptance. |
| Schema fitting and trained dictionaries | Implemented opt-in; local held-out comparison complete | Ten paired trials per family/size/arm, all 640 operations exact after reopen. Matching ~1 MiB batches: template 796,771 median body bytes versus raw 1,051,446.5 and zstd 130,432. Small normal records: trained dictionary 590 bytes versus zstd 667.5, before 8,705-byte per-destination seed. CPU and cold costs prevent an unconditional win. [Result](schema-fitting-results-2026-10-30.md). |
| Compressed schema and prepared codec reuse | Implemented opt-in; persistent local campaign complete | 160 ten-target sessions, 1,600 targets and 120 seeds all exact immediately and after reopening. Matching ~1 MiB body falls 796,723→127,557.5 bytes; prepared CPU 11.789 ms remains above ordinary zstd 3.846 ms. Prepared/fresh bodies match; actual setup, cache hits and losses are retained. [Result](schema-reuse-results-2026-10-30.md). |
| Exact prepared-layout sender optimization | Implemented; registered paired local CPU goal passed | Matching ~1 MiB sender CPU falls 32.06% by ten paired session ratios; total median CPU 11.8645→9.5525 ms and elapsed 19.841→16.989 ms. All 840 operations reopen exactly; complete protocol frames and 127,557.5-byte matching-batch body remain unchanged. Zstd remains cheaper in CPU/local time. [Result](schema-sender-results-2026-10-30.md). |
| Per-transfer IP accounting | Fifty-nine of sixty broad-campaign captures complete for the runner-to-relay leg | Middle-insertion shard 1 retains ambiguous TCP tuple reuse and no whole-case IP statistic. Earlier campaigns retain their original limitations. No whole-path or physical-link byte claim. |
| Direct/shaped WAN and three-peer qualification | Not run | Controlled impairment, recovery across three peers and eventual MVM boundaries |
| CUI/FIPS deployment qualification | Not claimed | Concrete validated module/OE/mode and contract-specific control assessment |

## Performance priority

The compressed-schema WAN follow-up is complete: 1,920 independently reopened
exact objects and 660 unchanged before/after body/mode pairs across thirty trials
per size. The optimized sender cuts matching-batch sender CPU by 31.67% versus
the old encoder. Its 2.28% body advantage versus the configured ordinary-zstd
profile costs more CPU; mixed ten-object sessions remain slower. See the
[real-WAN result](schema-wan-results-2026-10-30.md) for paid setup, losses and
measurement limits. The WAN origin now supports an explicit prepared-codec cache
and credential-free observation records; its default remains unchanged.

The user explicitly prioritized fewer bytes over the link, allowing additional computation at either endpoint. Exact reconstruction is mandatory. Transferred bytes are the primary performance acceptance metric; CPU, memory and elapsed time remain measured tradeoffs. A slower loopback mutation is not a failure if it materially reduces transmitted bytes. The earlier 2x latency target is a later optimization target, not a gate for the first working delivery.

## Invariants

`Decode(verified_receiver_state, recipe, residual) == original_bytes`, including original length. Object identity is independent of representation. Every record and complete reconstruction must validate before publication or commit acknowledgment. Cache existence does not grant authority. Resume revalidates principal, recipient, namespace and policy/execution epoch. Cards and source code are never executable network recipes.

Cold content uses raw/zstd bytes; warm content uses batched missing-record exchange, versioned sparse COPY/LITERAL recipes, or the bounded zstd prefix mode. The WAN sender batches protocol frames into bounded HTTP exchanges; a streaming sender with independently bounded memory remains a later qualification. These encodings share the same identity and commit contract. Legacy UOROBJ01 remains explicitly distinct from the full-SHA256 transfer profile. Cache, staging, message, output, work and concurrency budgets are separate and reserved before admission.

The object protocol uses complete frames of at most 64 KiB, including its 56-byte header; an HTTP batch may contain multiple frames. Its core object ceiling is 64 MiB, with four active transfers per peer within a shared 1 GiB cache quota and 512 MiB resident budget. The batched client remains bounded to 16 MiB. Larger-object claims and the final mailbox adapter require separate qualification. Object commit, mailbox admission, delivery and application processing are separate events; no exactly-once execution claim.

## Acceptance matrix

Exact equality: empty, NUL/trailing zero, chunk boundaries, append/truncate, insert/delete, localized/scattered changes, compressed bytes and ciphertext. Negative cases: malformed/oversized/truncated frames; missing or evicted descendants; corrupt/conflicting records; scale/padding faults; wrong recipient; revoke/epoch/replay; restart/cancel/crash before and after publication. No unverified or invalid object may be published. A transport or post-rename durability error can leave an uncertain commit outcome, which must be resolved through authorized verification.

Performance: compare raw, zstd, conventional CAS and rsync/VCDIFF over identical authenticated transport and durability. Time encoding through durable verified commit. Count both directions, metadata, requests, receipts, security, repair, retransmission and relay overhead. Report seeding cost, cache/storage, CPU/RSS and latency distributions. Use 30 randomized paired trials per acceptance workload, proper bidirectional impairment, then real direct/relay hosts and MVMs.

Targets, not achieved claims: warm repeated/localized-change 1–16 MiB objects save at least 90% total bytes versus the same secured raw baseline; halving median duration is a secondary future optimization; cold incompressible >=1 MiB incurs at most 5% extra bytes; report p95 duration and CPU separately rather than rejecting useful byte savings solely for extra compute. Report small objects, scattered edits and prefix insertion separately; compare against the best conventional baseline before claiming a SpiralCore-specific benefit.

## Delivery and external prerequisites

Deliver source/provenance, tests/vectors, protocol/threat decisions, claim ledger, adapter, reproducible demo/harness, measured receipts and an explicit gap list. MVM mailbox service/transport work (#4629/#4633) owns the canonical binding. Existing FlowMux/vsock 5253 is not a mailbox port. Experimental Iroh may remain outside a first validated-crypto deployment. HeatNet/vnet lacks a pinned implementation; historical HEAT.NET is only a possible reference.

Use public/synthetic data until deployment qualification. Enumerate envelope, transport, storage, identity, signing, RNG and key-management cryptography and bind qualification to exact module/version/operating environment/approved mode. A crypto-provider interface or SHA-256 algorithm name is not FIPS validation.

## Current runnable delivery

The latest sender optimization evaluates the runtime candidate first and skips
other codecs only when a proved bound rules out a smaller transfer. Bulk residual
scans and immutable identity reuse preserve the selected frames. The new paired
local comparison reduces prepared large generated-transfer CPU from 111.437 to
35.039 ms and elapsed time from 127.094 to 49.678 ms, with the same 747-byte body.
All 840 operations reopen exactly. The [sender result](sender-cpu-results-2026-10-30.md)
records edited/unrelated controls, regressions, cold setup and preparation. These
changes still need a new WAN qualification; earlier WAN observations remain tied
to their original revisions.

The coupled runtime now uses packed CA replay and supports immutable prepared
sender predictions. In the recorded local comparison, large-trace replay CPU fell
from 228.305 to 12.184 ms, with exact bytes and unchanged wire format. Complete
generated-transfer CPU fell from 559.601 to 117.050 ms when preparation was reused;
the separate 19.228 ms preparation cost and 14,854,835 retained bytes are recorded
in the [CPU optimization result](replay-cpu-results-2026-10-30.md). Receiver replay,
authorization, resource limits and durable verification remain enforced.

Use `crates/spiralcore-transfer`. Its original `spiralcore` CLI remains a loopback-only public/synthetic-data experiment, now with explicit sparse sends. The separate WAN binaries use a temporary bearer capability, allowlisted experiment scopes and verified HTTPS through Cloudflare. This bounded synthetic-data adapter is a working Internet transfer experiment, not a general public file service or a production security qualification. The existing warm-cache/wire prototypes remain separate research implementations. Geometric peer selection and MVM integration remain future work.

The release demo transfers 1 MiB cold/repeated/32-byte-mutated content over real TCP, then reopens the cache and checks exact bytes. A separate 16 MiB random cold/repeat run also passed exact recovery. See [validation receipt](validation.json) for source identities, measured bytes, complete test/coverage scope and limitations.

The [real WAN report](wan-results-2026-10-29.md) records the subsequent GitHub Linux → Cloudflare → Mac campaign. Warm DAG mutations reduced HTTP-body bytes substantially but were slower than full raw transfer on that path. Ordinary content-addressed exact-repeat hits reduced both bytes and time. Cold random and prefix-insertion DAG expansion remain explicit negatives. Neither the earlier smoke's original `capture_complete` flag nor a green workflow establishes complete packet accounting; the report retains the later counter reassessment.

The [subsequent efficiency report](efficiency-results-2026-10-29.md) records direct graph construction,
prepared state and sparse recipes at `b5520171ab1cfbc988e16b2b79f41ab831d3668a`. Sparse warm updates
save 99.80–99.98% of protocol-body bytes and complete 1.83–2.58 times faster than the current full-raw
arm in matched trials. Seed-plus-update gains are smaller and explicitly measured. In that campaign, raw/zstd
warm a DAG synchronously while sparse publication skips it, so equal durable-object fidelity does
not imply equal future DAG readiness. Exact repeats also match ordinary CAS. These results do not
establish superiority over a matched rsync/VCDIFF WAN implementation or energy savings.

The [broad v4 report](broad-wan-results-2026-10-29.md) supersedes that comparison's
publication-readiness mismatch: optional DAG warming is disabled equally for all
four methods. It registers thirty paired trials per workload, including a
conventional zstd shared-base control, and retains warm-update and paid-seed
statistics separately. Its measured scope is the bounded 16 MiB batched client;
the [64 MiB design](64m-client-design.md) is not an implemented or measured claim.

The live MVM adapter must support a small multi-MVM environment and its
orchestrator. It maps authenticated node, recipient, tenant/capability and epoch
identities to the canonical mailbox/vsock route; a reusable vsock CID is not an
identity. Bound queue bytes and concurrency, propagate backpressure, and define
restart/rebind/revocation and persistent idempotency. Preserve the canonical
encrypted envelope boundary and separate object commit, mailbox admission,
delivery and application processing. Small-VM memory budgets require explicit
qualification; the host experiment's budget is not an assumed guest budget.
