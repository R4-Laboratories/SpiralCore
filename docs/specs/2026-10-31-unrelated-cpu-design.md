# SC-OPT-019: avoid materializing unrelated-object losers

Date: 2026-10-31. Baseline: `56e15db5c77fe2ecdc03476d8e09664cd4213fd1`.
The user authorized reducing unrelated-transfer CPU while retaining minimum
winning bytes, exact recovery and mixed persistent-session measurements. This
is the bounded design recommendation; the fresh baseline stage profile and
parent release precede production edits. No further user approval is required
for routine implementation within that authorization.

## Decision and source finding

`schema_selection::PreparedSelector::prepare_warm` first evaluates sparse reuse,
then eligible Whole and compressed-schema candidates. `delta::encode` generates
prefix/suffix and aligned recipes, packs both, selects the cheaper and finally
filters against Raw. An unrelated base/target commonly produces two candidates
consisting solely of `Literal(0, target.len())`: both allocate and copy the entire
target only to be discarded. `schema_zstd` separately builds the exact SCVAL001
intermediate and compresses it. Their relative CPU costs are hypotheses until
the stage profile completes.

Recommend **rejecting only a generated whole-object literal candidate before
packing** as the first candidate, if the profile makes that work material.
Keep both scans, operation construction, all copy-bearing candidates, existing
packing, strict ties, Raw filter and public signatures unchanged. A private
helper around the two existing `pack` calls is sufficient; no new receipt or
wire mode is needed. If both candidates are rejected, the result remains `None`.
If either contains a COPY, run its existing pack/selection path unchanged.
Do not introduce an input-similarity heuristic or reinterpret unavailable state.

For target length `N > 0`, let `R = ceil(N / MAX_CHUNK)`. A whole-object literal
recipe needs at least `R` frames. Each carries its literal bytes plus at least
five operation bytes, nine sparse-header bytes and 128 request/reply bytes.
Thus sparse cost is at least `N + 142*R`; the existing Raw comparison is
`N + 133*R`, and sparse also requires the extra 32-byte base identity. Therefore
this candidate cannot pass `sparse_cost + 32 < raw_cost`. Additional splitting
only increases its cost. Match the exact generated shape and positive length;
do not generalize the proof to arbitrary malformed or zero-length operations.

This preserves `Recipe.frames`, `operations`, `None`, byte-minimum selection and
all successful encoded outputs. Existing input/operation/output bounds and
decoder errors remain. Eliminating an unnecessary allocation can avoid its
allocation failure; there is no promise to reproduce allocation failure timing.
The core and batched-client limits remain unchanged. Warm ACK promotion,
context/epoch binding, missing-base recovery, receiver validation and durable
commit are outside the change.

## Alternatives and tradeoffs

1. **Narrow literal rejection — recommended.** Constant-sized recognition of
   the generated case removes full-object copies without a general operation
   prepass. Scanning and both compression candidates still cost CPU; do not
   promise it removes the full replacement penalty.
2. **General lower bound or exact sizing before packing — defer.** This handles
   more cases but adds work to useful sparse candidates or duplicates packer
   rules. SC-OPT-007 already measured a general lower-bound candidate and rolled
   it back after CPU/control gates failed. Its unrelated-object improvement is
   historical context, not acceptance of a restored implementation.
3. **Skip compressed schema for binary/unmatched data — reject without a new
   proof.** Failure to match a JSON template does not prove that zstd over the
   literal-record stream cannot beat Whole or Raw. Entropy samples and unchanged
   original bytes are not sound cost certificates. Profile any future schema
   scan/copy change separately and require exact intermediate/frame parity.

## Qualification and measurement contract

Pin the current delta source and reuse the existing immutable delta oracle and
its established parity checks; do not duplicate the oracle/harness. A focused RED must observe
whole-literal frame materialization; GREEN removes that work, without a timing
assertion. Compare complete recipe bytes, operation counts, `None` and decoded
output against the oracle for empty/repeat/unrelated/one-byte coincidences,
append/truncate/insertion, chunk boundaries, scattered edits, operation limits
and periodic data. Include useful-COPY cases where only one candidate is a
whole literal. Through the existing selector, compare winning mode, actual
serialized frames, full bidirectional body costs and truthful pruning receipts.
The registered localized random updates must retain 600 body bytes and a
replacement must resume as the next acknowledged base. Exercise reuse after errors with existing
session tests; no implicit retries are added.

The fresh diagnostic independently times delta, prepared schema intermediate,
Whole, compressed schema, identity hashing and complete warm preparation at
1/16 MiB for localized and unrelated inputs: three fixed seeds, two sizes, two
input families and six stages give 72 observations. Record newline/record counts
because random 16 MiB data can cross the schema whole-literal record limit.
Contexts are reused within a stage
and released between stages. Record setup, generation, validation and drop
separately. These are independent calls, not additive components of one transfer.

Use the existing persistent-session harness: baseline persistent selector,
candidate persistent selector and equally configured prepared Whole. Both
selector arms fit identical training once per session; all arms pay the same
actual seed, use identical HTTP connection policy and preserve receiver checks.
Freeze an exact `delta.rs` baseline overlay only if this recommendation is
selected; this is a source-derived baseline, not a historical full revision.

Use one new profile in the existing harness, not a copied framework. Freeze ten
distinct target identities per session, covering localized random edits, unrelated
replacement then localized reuse, compressible content, held-out structured
records and schema drift. Keep training separate from those targets. Do not
assert 600 bytes for compressible/structured targets where another candidate can
win; require exact baseline/candidate winner and frame parity throughout.

Proposed local dose: ten paired trials per size, each base seed plus ten updates:
660 base/target transfers. If schema-ready cases are included, explicitly publish
the artifact once for each selector session, charged in its paid lifecycle:
at most 40 extra local transfers, 700 total. Readiness cannot substitute for an
ACKed publication. Freeze the exact sequence and seed policy in the contract;
include all failed preparation/seed work, and never fit held-out records.
Before candidate
timing, freeze gates in the measurement contract: replacement sender-CPU paired
median candidate/baseline <=0.95 at both sizes; mixed setup-plus-ten-update sender
CPU and localized-update paired CPU medians <=1.05 at both sizes; exactness and
byte/frame parity mandatory. Do not tune gates after results. Advance only after
those checks to one separately registered six-pair-per-size WAN pilot, 396
base/target transfers plus at most 24 artifact publications (420 total) in six
bounded stores, with every order represented. A failed pilot
stops and remains separate; no success-conditioned retries or pooling.

Report sender CPU, receiver handler CPU, durable completion, body bytes and
actual paid setup/seed lifecycle, including p95 and losing pairs. Whole's seed
is an equal-state comparison cost it does not inherently need. Exchange is a
nested scope, not pure wire time; headers/relay traffic and energy are unmeasured.
