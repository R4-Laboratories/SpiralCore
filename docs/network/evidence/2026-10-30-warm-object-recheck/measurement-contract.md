# SC-WAN-005 measurement contract

This registered recheck measures verified prior-object reuse, independently of
schema or codec-context reuse. The implementation follows the committed
[design](../../../specs/2026-10-30-warm-object-recheck-design.md) and
[plan](../../../plans/2026-10-30-warm-object-recheck.md). No production codec is
changed. Use the current `schema_fitting` executable's `warm-*` commands; earlier
schema commands retain their semantics.

## Inputs and dose

The base is exactly 1,048,576 or 16,777,216 pseudorandom bytes. The generator is
the historical xorshift64-v1: x ^= x << 13; x ^= x >> 7; x ^= x << 17, emitted
as little-endian u64 words. Its nonzero seed is the first eight little-endian
bytes of `hash("spiralcore.warm-object.fixture.v1\0", trial_u64_le, large_u8)`.
The changed object XORs each of the 32 bytes starting at `length / 2` with 0xa5;
all32 positions differ and the length is unchanged. Plans retain the actual
seed, both whole-object identities, SHA-256 values and lengths. These are new
explicit seeds under the historical generator, not a replay of old trial IDs.

WAN: trials0–29, two sizes, two arms, one seed and one update each. Three shards
contain ten trials per size each: 40 sessions, 80 transfers and exactly680MiB
stored object payload per shard. The conservative736.3125MiB bound includes
40MiB codec retention, 16MiB slack and80×4KiB headers. The 15 A/B and15 B/A orders are shuffled by fixed-seed Fisher-Yates for each
size, using xorshift64 from fixture_seed(30, large) and descending indices29 to1.
This is reproducible randomized counterbalancing over30 paired trials.

Local qualification: trials0–2, both sizes and arms:12 sessions/24 transfers.
The frozen executable launches a separate receiver subprocess using the same
`wan::serve_observed`, authentication, serializers and endpoint implementation.
It learns the bound loopback URL from READY stdout without a probe request. The
receiver stops normally after24 requests, then the stopped store is reopened.
The local dose is a small mechanism qualification, not WAN acceptance.

## Arms and measured boundaries

Both arms first send the identical Raw base to a fresh, separate authorized
namespace. The base/target identities differ, no target was previously sent to
that scope, and `warm_probe=false`. Forced Sparse calls
`client::transfer_sparse_batched`; Whole calls
`client::transfer_prepared_zstd_whole_batched`. Whole's attempted compression,
raw fallback, target hashes and transfer remain in its timer. Sparse performs
its existing comparison, delta construction, hashes and transfer without the
all-codec adaptive selector. Whole encoder context construction is timed once
per session; Sparse has no prepared encoder. Both use disabled DAG warming and
the same enabled, bounded receiver codec cache. Cache observations are checked
against actual modes; incompressible Whole may choose Raw and prepare no decoder.

Each update's primary elapsed time starts before the sender API and ends after
its validated durable acknowledgment. Sender ProcessTime includes HTTP/TLS and
serialization; receiver-handler CPU is a separate process scope and is added
once. Exchange CPU/elapsed are nested subsets; exchange elapsed includes server
computation and waiting, and is not pure wire time. The same helper measures
Raw seeding. Receiver socket/logging/relay CPU, process startup, fixture
construction, identity hashing for receipts and final independent verification
are outside transfer timers. Context CPU/wall are charged to the actual
seed-plus-update lifecycle as disjoint sums. No extra amortization or hidden
receiver warming is modeled.

Both HTTP-body directions, modes, request counts and exact lengths are retained.
Sparse must actually select SparseV1 and transfer600 body bytes; an exact-hit
shortcut or fallback cannot satisfy that gate. Raw seed/fallback body costs
must equal368 + length +133×ceil(length/61,440). Whole may select ZstdWholeV1 or
Raw; both are reported. Request and response body counts exclude HTTP/TLS,
TCP/IP, retransmissions and additional relay-hop traffic. No packet, energy,
MVM, FIPS or general compression claim is made.

After receiver shutdown, every seed and update is regenerated, read from its
original namespace, and compared byte-for-byte. Verification records actual
length/hash as well as comparison status. The summary requires all expected
rows, admitted receiver observations, source/executable/plan identities, costs,
contexts and seeds; missing/duplicate/failed rows reject it. Report medians,
p95, all30 paired sparse/whole ratios and wins, update and paid lifecycle
separately. The local three-pair summary uses the same formulas and explicitly
labels its small dose. All negative results and failed attempts remain retained;
no favorable reruns or warmup deletion.

## Commands and custody

Set `SPIRALCORE_WARM_SOURCE` to the committed implementation revision and
`SPIRALCORE_WAN_TOKEN` to the temporary capability without printing it.

```text
schema_fitting warm-identity
schema_fitting warm-plan RUN SHARD NEW_PLAN
schema_fitting warm-paired PLAN EXE URL RECEIVER_SHA NEW_JSONL
schema_fitting warm-verify PLAN STOPPED_CACHE NEW_JSONL
schema_fitting warm-summary PLAN0 RAW0 VERIFY0 PLAN1 RAW1 VERIFY1 PLAN2 RAW2 VERIFY2 NEW_SUMMARY
schema_fitting warm-local EXE NEW_DIRECTORY RUN
```

`warm-verify` additionally requires `SPIRALCORE_WARM_OBSERVATIONS` and
`SPIRALCORE_WARM_RECEIVER_SHA256`; the latter must match sender receipts and the
independently frozen receiver executable. Platform/feature identity is separate
from shared source pins, so native-Mac verification can check Linux sender
plans. `warm-session` and `warm-local-receiver` are bounded worker commands used
by the driver. Outputs use create-new semantics; failed child output and partial
JSONL remain available. Only a successful local run with complete reopened
verification removes its own cache, and records that cleanup. Remote cleanup
belongs to the parent after full verification. No measured run starts before
source and executable freeze; no owned builds/tests/profilers overlap it.
