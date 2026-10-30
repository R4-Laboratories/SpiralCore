# Persistent schema transfers over a real WAN

The user authorized filling the SC-OPT-013 WAN integration gaps and running real
transfers. Start from `cf490d3b4acdd9c3a417784042bd8e3a2d9c3640`. Preserve exact
bytes, existing compression settings, authorization, resource limits and durable
commit. This is an implementation and measurement task; no new codec or REST
endpoint is needed. The plan is approved for execution, subject to the existing
source freeze and qualification before measurements.

## Transport and persistent receiver

Use `wan::HttpClient` and the existing authenticated `POST /v1/batch` protocol.
Keep one receiver process and endpoint alive for a whole bounded shard, and one
client and prepared sender per ten-target session. The origin currently closes
each HTTP connection deliberately; persistence here refers to endpoint/codec
state, not a TCP keep-alive claim. Use the same HTTPS relay route, receiver
executable, filesystem durability and cache policy for every arm.

Add a strict optional boolean `codec_cache_enabled` to the server plan: absent
means false, non-booleans fail. Enable it before accepting requests. Retain the
existing eight-entry, namespace/mode/artifact-keyed codec cache and its shared
resident charge; artifact bytes are still reverified on every Begin. Each
arm/session receives a unique admitted scope, preventing earlier object hits or
cross-arm decoder state from warming its initial transfer. Within a session, the
artifact and prepared decoder can serve all ten distinct targets.

Retain the existing `serve(...)` API as a no-observer wrapper. Add an explicit
`serve_observed(..., observer)` wrapper sharing the same private request loop.
Only when supplied, the observer receives a `RequestObservation` containing
request ordinal, admitted scope, HTTP status and cumulative `CodecCacheStats`.
Invoke it after `process()` timing and before sending the response. The opt-in
CLI writes compact JSONL to stderr, without credentials or payloads, and treats
logging failure as a failed run. The no-observer path does not collect or print
these statistics. Logging is outside the receiver-handler CPU metric but inside
client-observed elapsed time and is identical for all arms in this experiment.

## Fair source comparison and dose

Run three arms: baseline prepared compressed schema, optimized prepared
compressed schema, and ordinary zstd. Both schema senders use the current frozen
harness and source tree; the baseline substitutes only `src/schema.rs` with the
immutable SC-OPT-012 fixture from `97b01a1`. Record the current source revision,
substitution provenance, complete relevant source hashes and both executable
hashes. Call it a **source-derived encoder baseline**, never an unchanged build
of the old repository revision. Other source, dependencies, compiler, flags and
zstd settings must match. Use the optimized build for the ordinary-zstd control.
Reuse the SC-OPT-013 immutable codec oracle; pair baseline/optimized body-byte
counts and selected modes without adding duplicate transcript capture.

Preserve the existing sequence: four normal targets, two lexical/order variants,
two drift targets and two unrelated targets. Use small records and approximately
1 MiB batches, with actual lengths recorded. Thirty paired trial blocks per
size use all six arm permutations exactly five times; freeze a seeded shuffle
of these orders before execution. Fixture index is `global_trial * 10 + position`,
with global trials 0 through 29; no target-time fitting or retry substitution.

Use three fresh receiver shards of ten trials each. Each shard has 60 sessions,
600 target operations and 40 raw artifact-seed operations: 640 operations total.
The complete campaign has 180 sessions, 1,800 targets and 120 seeds. Precompute
all fixture identities, exact object charges and operation/request bounds before
launch; reject a shard above 768 MiB of conservative object storage or existing
scope/state/request limits. The server's 1 GiB cache limit and resident limits
remain unchanged. Only one session runs at a time; no build, profiling or other
measurement overlaps the campaign.

Each schema session transfers the frozen artifact once, constructs its sender
once, then sends ten targets. Ordinary zstd pays no artificial schema setup.
Record setup separately and include actual setup in session/amortized totals.
Retain the frozen training and artifact provenance; fitting is an offline cost,
not a repeated per-session cost, and any broader amortization must name the reuse
assumption. Run a bounded loopback qualification first; actual WAN observations
use a remote machine and the registered HTTPS route.

## Metrics, exactness and interpretation

Record actual request and response body bytes, including SCW framing, manifests,
requests and acknowledgments. This is an **HTTP protocol-body byte** measurement;
it excludes HTTP/TLS/TCP/IP headers, retransmissions and additional relay hops.
No packet-level or complete-path byte claim follows from it.

Record sender process CPU across the complete encode/send operation, receiver
decode/handler/encode CPU reported by SCWREP01, their disjoint sum, and wall time
from operation entry through the verified durable commit acknowledgment. Record
exchange elapsed as a subset of that wall time, not pure wire time. Receiver
metrics exclude HTTP parsing/socket I/O and relay/host CPU; do not call the sum
complete system CPU. Preparation CPU and elapsed are separate. Report per-family
paired medians, p95, wins/losses and setup-inclusive session totals; do not turn
fewer bytes into an assumed speed or energy win.

A remote Published reply is initially labeled commit-acknowledged with reopen
verification pending. After each shard, stop its server, reopen the store with
the qualified receiver verifier, regenerate every artifact and target from the
frozen plan, and compare full lengths and bytes. Record missing, duplicate or
unexpected rows and all errors; no failed operation is silently retried or
discarded. Delete an owned shard cache only after exact verification and retained
evidence. The analyzer requires complete rows, source/config pins, baseline versus
optimized body/frame parity, and all 1,920 exact object verification receipts
before presenting a completed comparison. These synthetic results qualify this
route and workload; MVM integration and broader WAN generalization remain later.
