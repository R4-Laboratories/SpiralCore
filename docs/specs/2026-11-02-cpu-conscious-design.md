# Opt-in CPU-conscious Combined selection

User approved implementation of an explicit CPU-conscious mode alongside the
exhaustive reference. Baseline `9f5912698f6ab46ddc771fe15fd0c0a7bcec8c5b`.
Existing isolated checkout/shared Cargo target reused; no new dependencies.

## Decision

Count actual useful template records while emitting the canonical SCVAL stream.
Only emitted tag-1 records count: a matched template or a successful fast helper
can still emit a literal. Public encoding bytes remain identical. Count both fast
and fallback paths; empty/whole-literal/work-cap fallbacks report zero unless an
earlier useful template was actually emitted. Do not rescan the output.

Add `SelectionPolicy::{Exhaustive,CpuConscious}` with Exhaustive as default and
`CombinedSession::new_with_policy(context,prediction,artifact,policy)`. Existing
`new` delegates to Exhaustive. Policy is fixed for the session. After existing
early proof bounds, prepare the canonical intermediate and check limits/context
validity. CpuConscious skips further schema probe/compression iff its useful
template count is zero. Any positive count follows the exhaustive path. Existing
warm, repeat, runtime, cost ties, scope, authorization, ACK and receiver behavior
remain unchanged. No new wire encoding or capability is introduced.

`CombinedSelection` adds `policy` and `policy_skipped`, separate from proven
`pruned`. A `PolicySkippedCandidate` contains mode, reason
`NoUsefulTemplateRecords`, and useful-template count zero; it has no fabricated
encoded size or lower bound. Exact repeats retain policy with no policy skips.
Standalone schema and Warm selection keep their existing behavior.

The gateway facade adds `with_combined_policy`; existing `with_combined` remains
exhaustive. Explicit configuration mode `combined-cpu` selects CpuConscious;
`combined` keeps Exhaustive. Both require checkpoint/recipe/schema and reject
generic seeding. JSON keeps legacy policy identification and adds
`selection_policy` and `policy_skipped` with a named reason, apart from proof
prunes. Unknown configuration modes fail closed.

## Tradeoff and alternatives

Context/buffer reuse was measured and rejected in SC-NET-006. Literal-only
SCVAL is not proved dominated by Whole. This policy may miss a smaller schema
encoding while preserving exact bytes after decoding. Exhaustive remains the
default; no claim of globally minimum transfer size is made for CpuConscious.
It is an eligibility policy, not a wall-clock CPU-budget guarantee.

## Evidence contract

Focused RED/GREEN: actual emission counts, frozen canonical byte parity, no-fit
skip versus mixed/fit evaluation, actual avoided compression, poison/limit
propagation, explicit/default exhaustive parity, authenticated gateway opt-in,
invalid configuration, runtime/warm/repeat and exact reopened bytes.

Same-source paired campaign: Combined, CombinedCpu and equally tuned Whole-zstd;
three arm-order rotations, LAN and 80 ms RTT/10 Mbps shaped namespaces. Existing
1,047,202/14,851,384-byte tiers retain six sends and add two controls each. Total:
18 sessions, 336 operations including 48 auxiliary seeds, 288 exact deliveries.
Record both-direction logical/IP bytes, process/sender CPU, elapsed through
durable commit/readback, setup, skips and all byte opportunity costs.

Keep original schema artifact. Fit control uses 256-byte lines with trained
value/message layout (10-digit number and 222 hex characters). Drift control has
one fitting row followed by independent rows replacing message with drifted;
both have deterministic no-LF literal tails to exact tier lengths. Seeds are
`0x4350555f53434831 XOR trial XOR (steps<<32)`, with drift additionally XOR
`0x44524946545f3031`. No target refitting or tuning after measurement. Any useful
fit keeps schema eligible, so drift deliberately retains that cost.

Historical fixture profiles and analyses remain unchanged. Freeze code and
binaries before timing. Require fidelity and warm/runtime/repeat byte invariants;
report rather than conceal differing policy/exhaustive bytes elsewhere. Do not
promote the opt-in mode to default in this task. Shaped single-host execution is
not new Internet-WAN, MVM, energy or compliance qualification. Archive originals
in the existing private HF dataset and keep compact reviewable receipts in Git.
