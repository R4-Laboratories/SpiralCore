# SC-OPT-019: reduce losing-recipe copies and exact schema scanning

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
intermediate and compresses it. The completed 72-observation baseline profile
recovered all outputs exactly. Unrelated-input median CPU milliseconds were:

| Size | Delta | Schema intermediate | Schema plus zstd | Full selector |
|---|---:|---:|---:|---:|
| 1 MiB | 0.258 | 1.005 | 3.333 | 5.687 |
| 16 MiB | 5.268 | 15.994 | 65.850 | 124.473 |

These are independent calls, not additive pipeline components. Delta alone is
unlikely to meet the registered 5% full-sender reduction; schema scanning is a
second measured opportunity. Preserve a delta-only diagnostic before adding it.

First, **reject only a generated whole-object literal candidate before packing**.
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

## Bounded schema iterator amendment

After retaining the delta-only profile, promote the already locked `memchr`
2.8.3 package to an exact direct dependency (`memchr = "=2.8.3"`). No package
version or native codec settings change. Replace only the two newline scans in
`schema::encode_with` with a private borrowed iterator:

```rust
fn records(bytes: &[u8]) -> impl Iterator<Item = &[u8]> + '_
```

Use `memchr::memchr_iter(b'\n', bytes)` to produce inclusive-record end offsets
`i + 1`. Append `bytes.len()` only for a nonempty, unterminated final record;
map consecutive endpoints to borrowed slices using one previous-offset state.
An equivalent duplicate-tail suppression is acceptable. Do not allocate an
offset vector or change record contents. Rename the current local `records`
count to avoid shadowing the helper.

The iterator must equal `split_inclusive(|b| *b == b'\n')` exactly: empty input
produces no records; consecutive LF bytes produce separate nonempty records;
CR and arbitrary invalid UTF-8 bytes are retained; a terminal LF does not create
an empty tail. Keep the count pass's `take(MAX_RECORDS + 1)`, whole-literal
decision, record count, output reservation and encoding pass unchanged. No
training, template match budget, decoder, schema artifact or SCVAL001 format
changes. Identical intermediate bytes enter the same prepared zstd context,
with unchanged parameters; full compressed-frame parity remains mandatory.

This accelerates exact delimiter finding, not codec selection. It neither
predicts entropy nor skips a candidate. The iterator retains only borrowed
input, a bounded scanner and one offset; all existing output/resource bounds
apply. Both changes are independently testable. Stop production edits while
the parent captures each released diagnostic stage.

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
   original bytes are not sound cost certificates. The approved delimiter
   iterator instead removes scalar scan work while preserving every record.

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

For the iterator, start with a focused private-helper RED and compare all
yielded slices with the old standard iterator. Cover empty/no-LF/terminal-LF,
consecutive LF, CRLF, binary bytes, vector-size boundaries and counts immediately
below/at/above MAX_RECORDS. Reuse the existing schema oracle for full SCVAL001
and zstd-frame equality on structured, drift and random inputs, including both
sides of the whole-literal cutoff. Preserve malformed artifact/length errors
and alternating reused contexts. The performance gate is measured separately;
do not add a wall-clock assertion to correctness tests.

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
The final matched baseline overlays exactly `src/delta.rs`, `src/schema.rs`,
`Cargo.toml` and `Cargo.lock` from the pinned revision into the current common
harness. The manifest/lock overlay removes the added direct dependency edge
while retaining the same already-locked memchr package version. Restore and
verify all four candidate files after building. Identity comparison permits
only their corresponding `delta`, `schema`, `cargo_manifest` and `lockfile`
source fields to differ; all other measured source fields must match. This is
a source-derived baseline, not a historical full revision. Preserve delta-only
diagnostic receipts separately from final combined measurements.

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
