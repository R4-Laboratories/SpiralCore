# SC-OPT-018: acknowledged persistent-session measurement

Registered before implementation measurements. Follow the
[design](../../../specs/2026-10-31-persistent-session-design.md) and
[plan](../../../plans/2026-10-31-persistent-session.md), based on
`28a373ddb7ab9d236fd4b872db128559d4404cb8`. This is a same-binary comparison of
current public fresh-selector composition, the new owned `WarmSession`, and
equally tuned persistent Whole zstd. It does not compare an historical binary
against a different transport. Runtime prediction ownership is qualified
separately; the warm-object campaign is not a unified runtime/schema codec claim.

## Arms, fitting and acknowledged ownership

- `fresh-selector`: generate the unchanged 512-record training corpus, fit and
  verify the frozen 169-byte artifact **once per session**. Construct a fresh
  `PreparedSelector` from that artifact for each update. Manually retain and move
  the previous input only after the existing client validates the durable ACK.
- `persistent-selector`: perform the same once-per-session generation, fit and
  artifact check. Construct one `WarmSession`; explicitly seed its owned base,
  then send ten owned targets. The session retains its private verified base ID
  and promotes the same target Vec only after a validated durable ACK.
- `whole`: construct one prepared Whole encoder and reuse it for ten updates.
  Maintain the same input-generation/ownership sequence, although Whole does not
  need the prior-object seed. Its Raw fallback and attempted compression remain
  timed. It does not require a schema fit.

All arms use the same source, compiler, dependencies, features, durable receiver
policy, namespace rules and connection policy. Fresh is a composition of the
existing public API, not a deliberately repeated ten-fit comparator. Record
training/fit and constructor counts and costs. There are 20 fresh-selector,
two persistent-selector and two Whole constructions per store; the two selector
arms perform four actual fits per store. Context retention and base caching are
not treated as permission; bind every session and exchange to `wan::context(scope)`.

The API is `WarmSession::new(context, artifact)`, `seed_base(context, owned_vec,
id, exchange)`, and `send(context, owned_vec, id, false, exchange)`. Use its
selection/Stats results and read-only base identity/bytes/readiness/retention
getters. No timed send invokes automatic recovery or schema seeding. Unexpected
state loss, an unseeded winning Schema or any failed network operation ends the
sample and preserves its evidence. Never advance the manual base after failure.
Explicit recovery/restart and separate `RuntimeSession` behavior belong to the
focused correctness tests, not an invisible retry inside this campaign.

## Deterministic sequence and bounded dose

Use the existing trial/size-keyed xorshift64 little-endian random base, at exactly
1,048,576 and 16,777,216 bytes. Each session sends that actual Raw seed, then:

1. Updates 1–4: clone the immediately acknowledged base and XOR 32 bytes by
   `0xa5`, at `length/2 + (position-1)*64`.
2. Update 5: replace the entire object with independent pseudorandom bytes,
   domain-separated by experiment, trial and size. This is the escape case.
3. Updates 6–10: clone the newly acknowledged base and XOR 32 bytes at
   `length/2 + (position-6)*64`.

The escape seed is the first eight bytes, interpreted little-endian and clamped
to at least one, of the existing domain-separated `hash` function applied to
`b"spiralcore.persistent-session.escape.v1\0"` and the ordered parts
`trial_u64_le`, `[large_as_u8]`. The xorshift steps are left 13, right 7, left 17;
emit each updated state's little-endian bytes. The seed base retains the existing
`spiralcore.warm-object.fixture.v1` convention unchanged. Receiver attribution
uses trial zero for both sizes and clock modes.

All eleven object IDs must be distinct within a session. The nine localized
updates must select Sparse and cost 600 request-plus-response body bytes in both
selector arms. The escape must select the measured exact byte minimum and remain
identical between the two arms; its actual encoding and cost are recorded.
Recovery means warm compression resumes after the committed replacement, not an
error retry. Whole is a measured control, not assumed to choose a particular
mode. Cold schema readiness and exact-repeat probing remain false. DAG warming
is disabled; receiver codec-cache policy is equal for every arm.

One store contains one trial of both sizes and all three arms: six sessions,
six seeds, sixty targets, **66 transfers**. Retained object payload is
`3 * 11 * (1+16) MiB = 561 MiB`. The conservative store bound is 561 MiB payload
+ 40 MiB codec cache + 16 MiB transient allowance + 66 × 4096 B metadata =
**617.2578125 MiB**, below 768 MiB. Distinct scopes prevent one arm borrowing
another arm's cache state. Verify and clean each local store before the next;
retain a failed store. Check the 4 GiB free-disk floor before each store.

Local: ten paired trials per size, ten sequential stores, **660 transfers**, sixty
sessions and forty fits. WAN: six paired trials per size, six bounded stores,
**396 transfers**, thirty-six sessions and twenty-four fits. The WAN pilot uses
all six arm permutations once per size in a fixed seeded shuffle. Local uses the
same first six orders followed by four orders from the next seeded permutation
block. Bind the exact order in each canonical plan. This is a **six-pair WAN
pilot**, not thirty-pair acceptance or a claim of general network performance.

## Sender memory and generation timing

Do not precompute and retain eleven full-size input buffers beside the session:
176 MiB of fixture history plus retained codecs and the session's transient
allowance would exceed the shared budget. Instead, obtain the acknowledged base
slice from the manual Vec or `WarmSession::base_bytes`, generate one owned next
Vec, and move it into the operation. All arms perform the same generation and
move operations. No original-object history is retained for verification; it is
regenerated after receiver shutdown.

Record generation CPU/wall separately before each operation. It remains
**included in the true outer session CPU/wall interval**. It is not silently
subtracted from that interval. The initial random fixture is provisioned before
the session clock; its generation duration is also recorded separately. The
predeclared setup-plus-update CPU gate uses its named disjoint protocol phases,
while the report additionally shows outer generation-inclusive cost.

Before I/O enforce a shared 512 MiB conservative admission bound:

`retained session/context + acknowledged-base charge + offered target capacity
+ 128 MiB transient allowance + 64 MiB external-harness allowance`.

Do not double-count the base if already included in `retained_charge()`. For a
fresh selector, include its context charge and manually owned base explicitly.
Both owned input length and capacity are at most 16 MiB. The maximum two-selector
codec charge is 256 MiB plus 512 KiB schema metadata; a 16 MiB base, 16 MiB target,
128 MiB transient allowance and 64 MiB harness allowance leave headroom below
512 MiB. Include the actual bounded session metadata charge as well and reject
any total over budget. Do not substitute the smaller observed native workspace
for its conservative context charge.

Account for actual external capacities inside the 64 MiB allowance: capture of
the two selector arms' selected request frames (one escape payload plus nine
tiny updates), training/reference/artifact buffers while live, plans and deferred
receipt metadata. Do not capture ten Whole payloads or duplicate inputs for later
readback. Hash captured frames only after the outer timer. All observed retained
bytes, input capacities and conservative charges are reported; these are bounded
owned-state counters, **not measured process RSS, allocator peaks or energy**.

## Connection, operation and cumulative accounting

All arms retain one HttpClient per session and opt into the same bounded server
policy: `max_requests_per_connection=32`, `idle_timeout_ms=500`. Default-close
behavior is unchanged outside this profile. Qualify actual direct HTTP reuse
across all eleven requests. Record `connection_ordinal` and
`request_on_connection` for every admitted request. Relay telemetry describes the
observed hop only; it does not prove one socket spans sender, relay and receiver.

Each operation wall timer encloses preparation/encoding/hashing, sending and the
validated durable ACK. The fresh arm's per-update construction and destruction
are charged in that update; the persistent arm's initial construction is charged
once in setup. Sender-process CPU and disjoint receiver-handler CPU are recorded
separately, then summed once. Exchange time is nested and is not pure wire time.
Bytes count both HTTP bodies including reply metrics, excluding transport
headers, retransmissions and extra relay hops. Receiver socket/logging and relay
CPU remain outside the handler metric.

Buffer measurements until the last ACK so JSON writing and transcript hashing
do not become unreported gaps in completion. Record true outer sender CPU/wall
from setup entry through the final ACK, including actual generation and loop
overhead. Separately report setup, seed, each update, first use, updates 2–10,
all ten updates, generation, and actual cumulative paid lifecycle. These are
overlapping views: never add outer totals to their component phases. Add the
disjoint receiver CPU once to outer sender CPU for an outer CPU subtotal.

Whole's seed is deliberately paid for equal initial prior state; it is unnecessary
for that codec. Label paid lifecycles accordingly rather than portraying this as
an optimal cold-start Whole comparison. No hypothetical reuse replaces observed
setup or seed costs, and no missing stage is estimated as zero.

## Exactness, gates and separate attribution

Pin source/executable/plan identities and every expected original byte string.
Before a target, require its expected ID to differ from prior IDs. Validate all
ACK identities and compare fresh/persistent selected modes, costs and complete
normalized winning request frames at every position. After stopping the receiver,
regenerate and compare every seed and target byte-for-byte. Preserve all errors,
partial rows and stores; use exclusive output creation and no implicit rerun.

Advance from local to WAN only after exactness and complete counts pass, and the
median of ten paired persistent/fresh **setup-plus-ten-update sender CPU ratios**
is **≤1.05 at each size**. The sum includes one actual generation/fit of training,
all actual constructors, ownership/hash work inside those phases and ten transfers;
the initial seed is shown separately and in full paid lifecycle. Target-fixture
generation has its explicit separate phase and is included in true outer cost.
All ratios must be positive/finite; all first-use and unfavorable observations
remain. There is no guaranteed 10% gain, and no loopback wall-time gate is used as
a substitute for WAN evidence. Report Whole and receiver/outer-time comparisons
even when they regress.

The design's receiver attribution is a separate 44-object diagnostic: two sizes,
two clock modes and eleven objects per sequence. Retain both clock modes and
exact reconstruction; use grouped Begin/Data/Finish attribution only if median
instrumentation overhead is ≤1.10. It does not change the main dose or authorize
an unmeasured receiver optimization. No builds, tests, profiling or uploads may
overlap registered timing. Parent owns the single build/timing lane and freezes.

## CLI contract

```
schema_fitting persistent-identity
schema_fitting persistent-plan RUN SHARD NEW_PLAN
schema_fitting persistent-paired PLAN EXE URL RECEIVER_SHA NEW_JSONL
schema_fitting persistent-verify PLAN STOPPED_CACHE NEW_JSONL
schema_fitting persistent-summary PLAN0 RAW0 VERIFY0 ... PLAN5 RAW5 VERIFY5 NEW_JSON
schema_fitting persistent-local EXE NEW_DIRECTORY RUN
schema_fitting persistent-local-summary TEN_PLAN_RAW_VERIFY_TRIPLES NEW_JSON
```

Use `SPIRALCORE_PERSISTENT_SOURCE`, `SPIRALCORE_PERSISTENT_OBSERVATIONS` and
`SPIRALCORE_PERSISTENT_RECEIVER_SHA256` for source and verification bindings.
`SPIRALCORE_WAN_TOKEN` remains an unprinted environment capability. Public synthetic
fixtures only. Existing historical commands and sealed evidence retain their
original semantics. Reuse the existing serializers, timers, observer parser and
summary primitives; no new generic measurement framework is introduced.
