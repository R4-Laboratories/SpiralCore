# SC-OPT-015: integrated warm-object selection measurement

Status: registered under design/plan commit
`a438fc7658a823a48fe2a83efb502e2e8c87d331`. Implementation is authorized; timing
awaits the implementation commit, executable freeze and parent release. Extend the existing `warm_object` Rust harness under a new
campaign profile and `warmselect-*` commands. Preserve the historical `warm-*`
commands and tests. Production and harness are frozen together before measurements.

## Matched inputs and arms

Reuse the SC-WAN-005 xorshift64-v1 fixtures and exact identities for each trial:
1,048,576 and16,777,216 pseudorandom bytes; the changed target XORs32 bytes at
length/2 with0xa5. Every target differs from its base at exactly32 positions.
Each independent authorized namespace receives the same actual Raw base seed,
then one previously absent changed target. All arms use `warm_probe=false`,
disabled DAG warming and the same bounded enabled receiver codec cache.

Compare three current-source arms:

- Forced Sparse through the existing `transfer_sparse_batched`.
- Whole zstd through the existing prepared level-3/window-25/LDM/checksum path,
  including its attempted compression and possible Raw fallback.
- Integrated warm selector using `PreparedSelector::prepare_warm` followed by
  the consuming `transfer_warm_selection_batched` without duplicate encoding.

The selector starts without a receiver schema artifact. Its sender has the
same fixed169-byte schema artifact fitted from the frozen512-record training
corpus. Context construction, schema fitting/artifact construction, actual
base seed, complete target preparation and transfer remain visible and paid.
This campaign neither supplies a prepared base nor claims its hash is free.

The expected Sparse target transfer is600 HTTP-body bytes. At16MiB, Whole's
structural lower bound896 proves it cannot win; at1MiB its lower bound536
requires actual evaluation. Cold Schema includes its actual670-byte artifact
seed in its effective bound and cannot beat this fixture's600-byte Sparse
candidate. These are pruning rules to verify, not assumed observed outcomes.
An unexpected `needs_schema_seed` in this fixed warm workload fails registration
rather than silently adding a transfer or treating setup as free. Behavioral
schema-winning and absent/empty-base cases belong to the production tests.

## Dose and order

WAN:30 paired trials per size, three arms, one seed and one update each:
180 sessions,180 seeds,180 targets,360 transfers. Six shards each contain five
trials per size,30 sessions and60 transfers, retaining exactly510MiB object
payload. The registered conservative bound is566.234375MiB: payload plus40MiB
codec retention,16MiB slack and60×4KiB publication allowance, below768MiB.
For each size, each of the six three-arm permutations occurs five times; a
fixed-seed Fisher-Yates shuffle of30 permutation indices determines their trial
order reproducibly, using xorshift64 seeded from fixture_seed(31, large).

Local qualification:three paired trials per size, three arms,18 sessions and
36 seed/update transfers. Use the same separate receiver subprocess as the
existing harness, stopping after exactly36 registered requests and then
reopening the store. No readiness probe consumes a registered request.

Each selector session actually fits the existing frozen512-record training
corpus inside its context-setup timer, checks the resulting169-byte artifact
against its fixed identity, and constructs the prepared selector. This is60
paid fits in the WAN campaign and six in local qualification. Record training
bytes and artifact identity/length with each setup; charge the complete measured
setup once to that session's lifecycle. Whole context preparation is also paid
once per session; Sparse has no prepared context. No shared cross-session fit
amortization or hypothetical longer update sequence is claimed.

## Timing, validation and reporting

The target timer starts before the selected arm's preparation/encoding and ends
after validated durable ACK. Selector preparation and consuming send share one
outer timer; the small immutable receipt is cloned before consuming the plan and serialized
afterward. Receipt-clone overhead remains charged to the selector. Record exact
evaluated candidate costs separately from proven pruning lower bounds, including
cold setup costs, selected mode and schema-seed requirement. Verify the chosen
exact cost is no greater than any evaluated candidate or pruned lower bound,
and join actual body/mode/object receipts with both forced controls.

Sender process CPU and receiver-handler CPU are separate process scopes; add
once. Nested exchange CPU/wall remain subsets, and exchange wall is not pure
wire time. Report request and response HTTP bodies, actual selected modes,
context retention/charges, seed/update timings and setup-inclusive actual
lifecycles. Combined fitting/context setup, seed and update use disjoint scopes;
raw rows retain the complete components so actual totals remain reviewable.
Fixtures, process startup and independent verification are outside transfer
timers. HTTP/TLS/TCP/IP, retransmissions, extra relay-hop bytes and relay CPU
remain outside these body/CPU measurements. No energy or WAN generalization
follows from a local result.

After each receiver stops, independently regenerate and byte-compare every seed
and target in its original scope. Reject missing, duplicated, failed, mismatched
or undercharged observations; maintain source/executable/plan/receiver binding.
Report medians/p95 and all30 paired per-trial ratios for selector versus both
forced Sparse and Whole, including setup and negative outcomes. Preserve every
failed attempt, without retries or discarding warmups.

## Interfaces

Command contract (same argument meanings as existing `warm-*`):

```text
schema_fitting warmselect-identity
schema_fitting warmselect-plan RUN SHARD NEW_PLAN
schema_fitting warmselect-paired PLAN EXE URL RECEIVER_SHA NEW_JSONL
schema_fitting warmselect-verify PLAN STOPPED_CACHE NEW_JSONL
schema_fitting warmselect-summary PLAN0 RAW0 VERIFY0 ... PLAN5 RAW5 VERIFY5 NEW_JSON
schema_fitting warmselect-local EXE NEW_DIRECTORY RUN
```

Use `SPIRALCORE_WARMSELECT_SOURCE`, `SPIRALCORE_WARMSELECT_OBSERVATIONS` and
`SPIRALCORE_WARMSELECT_RECEIVER_SHA256`; temporary authentication still uses
`SPIRALCORE_WAN_TOKEN`, never printed. Source pins remain platform-independent;
executable/platform/feature pins remain explicit. No builds, tests, profiling or
uploads overlap the one registered measurement run.
