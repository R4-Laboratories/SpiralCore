# SC-OPT-014: byte-first schema selection with an equally tuned zstd control

This is a registration, not a measured result. Keep the SC-WAN-004 fixture bytes,
fixed 169-byte SC012 artifact, ten-target sequence and thirty paired trial blocks
per size. Sizes are individual small records and whole-record batches of about
1 MiB. Each session sends four matching targets, two lexical/order variants, two
schema-drift targets and two unrelated binary targets. Do not refit on these
targets, change fixtures in response to results, discard observations or retry a
failed transfer for a better measurement.

Compare three arms from one frozen source and executable:

1. **schema:** the prepared schema-plus-zstd path with its existing raw fallback;
   pay one artifact seed before the first target, even if no target uses it.
2. **whole:** prepared whole-object zstd with raw fallback and no artifact seed.
3. **selector:** prepare all raw, whole-zstd and schema-zstd candidates, then
   transfer the minimum complete HTTP-body cost. Include a previously unpaid
   artifact seed in the schema candidate's effective cost. Seed only if that
   candidate wins; mark it available only after actual committed seed transfer.

Whole zstd and schema's zstd stage use the same level-3, window-25, long-distance
matching, checksum, content-size and single-worker settings. Their representations
and framing differ, and complete framing costs remain in the comparison. No
candidate encoding/hash work is silently reused from another arm or excluded.
Raw wins ties before whole zstd, then schema. This is a greedy per-transfer byte
minimum under verified session state, not a future-aware optimum over a sequence.

There are three shards of ten trials each, sixty sessions per shard and 180
sessions overall. Six counterbalanced arm permutations occur five times per size
over all thirty trials. Every session uses a fresh authorized scope within a
persistent receiver, and its own retained sender context. Each session sends ten
distinct objects: **1,800 target operations** total. There are sixty mandatory
schema seeds and zero through sixty selector seeds, giving **1,860–1,920 actual
object transfers**. Sender-context construction is recorded once for each of the
180 sessions; selector construction includes both compression contexts.

Plans bind exact fixture/artifact hashes, source hashes, all allowed scopes and
the maximum storage/operation budget before execution. Each shard is bounded
below 768 MiB of conservative storage and the existing 1 GiB cache quota. Whole
and schema decoders use the same bounded, namespace-scoped retention policy.
Receiver observations must agree with actual selected modes and per-mode cache
preparations/hits; retention is not an implicit authorization grant.

For selector targets, record three disjoint measured phases: candidate preparation,
the optional actual artifact seed, and sending the already prepared selected
transfer. The preparation receipt binds the target identity, all three exact
target-body costs, cold seed charge, effective candidate costs, selected mode and
seed requirement. The current artifact's successful raw-seed cost is 670 body
bytes; validate this against the actual transfer rather than substituting an
unmeasured constant for observation. Candidate errors are failures, not silently
suppressed alternatives. No candidate is recomputed after selection.

The primary target wall interval starts before selection/encoding and ends after
the final durable acknowledgment, including any selected seed. Metadata is
buffered and serialized after this outer timer stops. Primary sender CPU covers
that same outer operation; receiver-handler CPU is disjoint and added once.
Disjoint phase sums are supplementary diagnostics, not a replacement for true
completion elapsed. A seed is never added to both its charged target and session
totals. Context construction remains separate and is included once in actual
ten-send session totals. Forced-arm target timers include their encoding, fallback
work and transfer. The schema arm's upfront seed is included once in its actual
session cost. Report all phase rows, actual setup, retained-context charges and
first versus later targets.

Measure both directions of actual HTTP protocol bodies, full sender-process CPU
including HTTP/TLS, disjoint receiver-handler CPU and durable completion elapsed.
Exchange elapsed includes remote work and waiting; it is not pure wire time.
The CPU sum excludes receiver HTTP/socket processing, logging and relay CPU.
Body counts exclude HTTP/TLS/TCP/IP overhead, retransmissions and extra relay-hop
traffic. No energy or complete-system efficiency result is implied.

After stopping each receiver, reopen its store and independently compare every
target and every actually seeded artifact byte for byte. Inspect optional selector
artifacts as well: only an exact object or the specific missing-object error is
accepted. The summary joins those presence/absence checks to actual paid seed
rows. No successful readback is inferred from an acknowledgment alone.

Before reporting a complete campaign, require all three shards, identities, phase
rows, admitted request observations, actual seed receipts and reopened comparisons.
For each target, verify the selected target plus its newly required seed costs no
more than the equally tuned whole arm on that exact input; verify the same
inequality for actual cumulative session bytes. Cross-check candidate costs with
the paired forced arms where applicable and retain all choice/fallback modes.
Statistics use thirty per-trial family sums, with median/p95, geometric paired
ratios and wins/losses. Positions within one trial are not independent trials.
Report negative CPU and elapsed results even when the byte-selection invariant
passes. This is one bounded synthetic WAN route; MVM, real-application corpora,
shaped-network acceptance, FIPS qualification and general speed claims remain
outside this campaign.

The new commands preserve the historical WAN harness's behavior:

```text
SPIRALCORE_SELECTION_SOURCE=<frozen commit> schema_fitting selection-plan RUN SHARD NEW_PLAN
SPIRALCORE_WAN_TOKEN=<capability> schema_fitting selection-paired PLAN EXE URL RECEIVER_SHA NEW_JSONL
SPIRALCORE_SELECTION_OBSERVATIONS=receiver-stderr.log schema_fitting selection-verify PLAN STOPPED_CACHE NEW_VERIFY_JSONL
schema_fitting selection-summary PLAN0 RAW0 VERIFY0 PLAN1 RAW1 VERIFY1 PLAN2 RAW2 VERIFY2 NEW_SUMMARY
```

The parent campaign receipt supplies the source/compiler/features/executable,
workflow attempt, route and hardware pins. Qualify locally, freeze source/plan,
then run the registered WAN dose once. No builds, tests, profiling or uploads may
overlap the timed run. Exclusive output creation and retained failures apply to
every phase and child process. Reuse shared fixture, observation and statistical
helpers where behavior is identical; do not add another analysis framework.
