# SC-OPT-017 measurement contract

Registered before candidate performance collection. This extends the existing
`warm_object.rs` runner; historical `warm-*`, `warmselect-*`, and `bounded-*`
commands retain their original fixtures and doses. The new profile is
`selector-setup.v1`. Implementation hypothesis: reduce actual fitting work and
avoid an inconclusive optional native probe on recognizable repetitive input,
while preserving the selected bytes and full-encoder fallback.

## Matched sources and setup attribution

Both executables contain the same final harness, receiver, dependency lockfile,
compiler target and feature configuration. The baseline overlays exactly
`src/schema.rs` and `src/schema_selection.rs` from
`4bb79f976e0939f3ef8b48c3830b07086b98f280`; their SHA-256 values are respectively
`8c81c80b94faf4579a9875a825e47575b61652b9e4c03130ab6bc783e2905109` and
`f5fb4a17015ac795af32c08b1daac128a1f861acded57671217c6ef5477940cc`.
`zstd_whole.rs` is identical. Both builds must report the qualified first-block
probe available. Do not set `SPIRALCORE_DISABLE_ZSTD_PROBE` for this baseline.
This is a common-harness production overlay, not an historical-full-checkout claim.

Each selector session actually generates the unchanged 512 training records,
builds the references, calls `schema::fit`, checks the exact frozen 169-byte
artifact, and constructs `PreparedSelector`. The entire setup CPU/wall interval
is charged once to that session. Temporary training and artifact cleanup are
included. Whole pays its actual prepared-context construction; no artifact fit
is necessary for that control. There is no cross-session fit cache or fabricated
amortization. The receiver persists within each shard; each sender session has
one seed and one update. Production `PreparedSelector::new` receives an artifact
and does not itself generate training records or fit it.

`setup-profile NEW_JSONL` retains exactly 20 iterations, including the first.
It records the outer setup interval and five nested, disjoint stage intervals:
training generation/reference construction, fit including serialization,
artifact comparison, context construction, and temporary cleanup. Returned
context destruction, hashes and JSON writing follow the timer. Stage values
must not be added again to the outer total. A pre-change baseline was captured;
a candidate profile may describe attribution separately, but those sequential
profiles are not a randomized paired performance gate. Actual paired setup
observations come from the transfer campaign. The corpus generator is unchanged.

## Fixtures and dose

Three explicit cases use a 32-byte midpoint XOR by `0xa5`:

| Case | Prior object | Size | Required selected update |
|---|---|---:|---|
| `random-1m` | Existing trial-keyed xorshift64 fixture | 1,048,576 B | Sparse, 600 body bytes |
| `zero-1m` | All zero bytes | 1,048,576 B | Whole, 570 body bytes |
| `random-16m` | Existing trial-keyed xorshift64 fixture | 16,777,216 B | Sparse, 600 body bytes |

The three arms are `baseline-selector`, `candidate-selector`, and equally tuned
prepared `whole` with Raw fallback. All have their own authorized namespace,
actual Raw base seed, absent target, disabled exact-repeat probe and disabled DAG
warming. Codec retention is enabled equally on the persistent receiver. Cold
schema readiness is false; this dose must not require an artifact seed. Every
seed and target must reopen byte-for-byte after stopping the receiver.

For every case the existing deterministic seeded six-permutation order contains
each arm permutation five times over 30 WAN trials. The zero case uses the same
small-case order rotated by seven trial indices. Local collection uses the first
ten trials; WAN uses all thirty. Each five-trial shard has 45 sessions, 45 seeds,
45 targets and 30 actual fits. Two local shards total **180 transfers/60 fits**;
six WAN shards total **540 transfers/180 fits**. Each shard retains 540 MiB of
payload. The conservative bound is 540 MiB + 40 MiB codec cache + 16 MiB transient
allowance + 90 × 4096 B metadata = **596.3515625 MiB**, below 768 MiB. A verified
local shard is cleaned before the next begins. Failures preserve their stores.

## Timers, bytes and checks

The primary update wall interval starts before selection/encoding/hashing and
ends at the verified durable acknowledgment. Sender-process CPU and disjoint
receiver-handler CPU are reported separately and summed once. Exchange CPU/wall
are nested subsets; exchange wall is not pure wire time. Local HTTP uses a
separate receiver process to preserve that accounting. Body bytes count requests
and replies, including reply metric fields, but omit HTTP/TLS/IP headers,
retransmissions and extra relay hops. Receiver socket/logging and relay CPU are
not included; energy is not measured.

Actual lifecycle totals add setup, paid Raw seed and update exactly once. Input
fixture generation, child startup, receipt serialization/hash, and independent
reopen checks are outside operation timing. Both selector arms incur the same
winning-frame capture overhead; frame hashing runs afterward. Normalize only
transfer IDs, then require complete winning request-frame hash, frame count,
mode, object identity and body parity. The three registered cases must also
preserve their complete selection receipts. Pruned bounds stay distinct from
evaluated costs. Source, executable, canonical plan, fixture and raw receipt
identities are retained and joined across shards. Failed child output is retained;
there is no implicit retry, discarded warmup or performance rerun.

## Fallback and misleading-sample controls

`setup-control BASE_EXE CAND_EXE NEW_DIRECTORY` runs ten interleaved pairs for
each of three 1 MiB bases: zero, a repeated 64-byte pseudorandom pattern, and
pseudorandom data whose eight sampled 64-byte windows are zero. All receive the
same midpoint edit. Each child pays actual setup and Raw seeding, times only
`prepare_warm` for the control metric, then serializes the selected transfer and
reopens both objects outside that timer. All 60 observations and 120 exact object
checks remain. The zero case must retain its 570-byte Whole winner; the periodic
and deceptive cases report whichever exact winner occurs. Full winner-frame
parity is mandatory. On deceptive samples, a prior sound Whole pruning receipt
may become an evaluated losing Whole candidate; its cost must respect the prior
bound and all other receipts must agree. This is not a WAN timing dose.

## Predeclared advancement gates

The local transfer summary and separate zero-control summary must jointly pass:

1. Exactness, source binding, complete counts and unchanged selected frames/bytes.
2. Median of the **ten `random-1m` paired context-setup CPU ratios**
   candidate/baseline ≤ **0.80**. Do not pool or select among the other cases.
3. Median of the **ten zero-control paired preparation CPU ratios** ≤ **0.95**.
4. Median of the **ten `random-1m` paired update sender CPU ratios** ≤ **1.05**.

Every ratio must be finite and positive. A failed gate is a valid negative result;
retain it and stop WAN advancement. Report all case/setup/update/lifecycle
distributions, paired ratios, wins and losses, including the unchanged large
path and misleading-sample controls. Neither the stage profile nor a median of
another cohort substitutes for the registered joins.

## Commands and environment

```
schema_fitting setup-identity
schema_fitting setup-profile NEW_JSONL
schema_fitting setup-plan RUN SHARD NEW_PLAN
schema_fitting setup-paired PLAN BASE_EXE CAND_EXE URL RECEIVER_SHA NEW_JSONL
schema_fitting setup-verify PLAN STOPPED_CACHE NEW_JSONL
schema_fitting setup-summary PLAN0 RAW0 VERIFY0 ... PLAN5 RAW5 VERIFY5 NEW_JSON
schema_fitting setup-local BASE_EXE CAND_EXE NEW_DIRECTORY RUN
schema_fitting setup-local-summary PLAN0 RAW0 VERIFY0 PLAN1 RAW1 VERIFY1 NEW_JSON
schema_fitting setup-control BASE_EXE CAND_EXE NEW_DIRECTORY
```

`SPIRALCORE_SETUP_SOURCE` supplies the immutable common-source commit.
`SPIRALCORE_WAN_TOKEN` supplies the capability and is never printed.
`SPIRALCORE_SETUP_OBSERVATIONS` and `SPIRALCORE_SETUP_RECEIVER_SHA256` bind the
independent stopped-store verification. Outputs use exclusive creation. Parent
owns baseline overlay/build custody, frozen plans/executables, the single build
and timing lane, remote orchestration and publication. No builds, tests, profiling
or uploads overlap registered timing. No new dependencies or Python are used.
