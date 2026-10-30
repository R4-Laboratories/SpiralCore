# Registered schema WAN follow-up

This implements the approved [design](../../../specs/2026-10-30-schema-wan-design.md).
No results are implied by this registration. One campaign contains three shards,
each with ten global trials, two sizes and three arms: source-derived baseline
prepared schema, optimized prepared schema, and ordinary zstd. Each session sends
four normal targets, two variants, two drift targets and two unrelated targets.
The six arm permutations are shuffled with seed `0x25103013abba0303`, XOR the size
flag, and each occurs five times over thirty trials per size. No trial is discarded.

The fixed 169-byte SC012 artifact is seeded once per schema session; it is never
refitted. Preparation occurs once per session. Each shard has 600 targets, forty
seeds, forty preparations and sixty persistent sessions. Full dose is 1,800
targets, 120 seeds and 180 sessions. Every session has a distinct authorized scope;
fixture index `trial * 10 + position` preserves previous indices and extends
through 299. Plans bind exact fixture hashes, source hashes, actual payload
charges, scope allowlists and a conservative storage bound below 768 MiB. Baseline
and optimized Linux senders share the complete source/build configuration except
the pinned old `src/schema.rs`. Platform feature flags and executable hashes are
recorded separately from platform-independent source/plan hashes.

The existing HTTPS adapter carries real request/reply bodies. Sender CPU spans
encoding and HTTP/TLS calls. Receiver CPU spans the separately timed handler,
decode and reply encoding; adding these disjoint CPU intervals excludes receiver
HTTP/socket and relay CPU. Wall time ends at the verified durable-commit reply.
Exchange wall time is a nested interval including remote processing and waiting,
not pure wire time. Body bytes include both directions of SCW framing and exclude
HTTP/TLS/TCP/IP overhead, retransmissions and extra relay-hop traffic. No energy
or complete-path-byte claim follows. The origin closes each HTTP connection;
persistence means retained endpoint and codec state across distinct targets.

Remote success is initially marked `commit-acknowledged-independent-reopen-pending`.
After stopping each receiver, the verifier regenerates and compares all 640
objects byte for byte. It also binds the raw receiver-observation log, admitted
request scope/order, cache counters and any ordinal gaps caused by rejected or
incomplete requests. Summary requires all 1,920 exact reopened comparisons, source
and executable pins, complete observations, and before/after body/mode parity.
Per-family paired statistics use thirty trial-block sums, never 120 independent
normal-position samples. Session totals charge actual seeds and preparation;
all controls, fallbacks, failed attempts and negative speed results are retained.

Commands use the `schema_fitting` executable:

```text
SPIRALCORE_SCHEMA_SOURCE=<frozen commit> schema_fitting wan-plan RUN SHARD NEW_PLAN
SPIRALCORE_WAN_TOKEN=<capability> schema_fitting wan-paired PLAN BASE_EXE CAND_EXE URL RECEIVER_SHA NEW_JSONL
SPIRALCORE_SCHEMA_OBSERVATIONS=receiver-stderr.log schema_fitting wan-verify PLAN STOPPED_CACHE NEW_VERIFY_JSONL
schema_fitting wan-summary PLAN0 RAW0 VERIFY0 PLAN1 RAW1 VERIFY1 PLAN2 RAW2 VERIFY2 NEW_SUMMARY
```

`wan-paired` invokes one `wan-session` child per registered source/arm/session.
Outputs use exclusive creation. Child failures retain partial rows, stdout,
stderr and exit status; no transfer is retried. Receiver caches remain intact
until independent verification and evidence preservation. The frozen source,
compiler/feature/executable identities, source-substitution check and workflow
attempt are supplied by the final campaign receipt. Builds, tests, profiling and
uploads must finish before the measured run begins.
