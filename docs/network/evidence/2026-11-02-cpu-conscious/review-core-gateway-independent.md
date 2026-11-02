# Independent CPU-conscious core and gateway review

Reviewer: `mvm_tests`. Date: 2026-11-02. Verdict: **PASS; no blocking findings.**
Baseline `9f5912698f6ab46ddc771fe15fd0c0a7bcec8c5b`. This bounded cross-author
Rust/code/security review covers the four transfer production files, their
focused tests, gateway facade/service and authenticated gateway test changes.
No product edits, builds or new test runs were performed by the reviewer.

## Correctness and safety

Useful-template counts increment only at the two actual tag-1 emission sites.
The fast helper's boolean result is not mistaken for an emission: its literal
size/work-budget fallbacks do not increment. Counts cannot exceed MAX_RECORDS;
empty and excessive-record whole-literal streams return zero. Existing SCVAL
bytes, target limits, match-work accounting and public encode APIs are preserved.

Exhaustive is the enum default and the existing CombinedSession constructor's
explicit delegation. Warm and standalone schema paths remain exhaustive. Only
the new immutable session policy enables the skip. Existing early proven bounds
run first; an otherwise eligible schema candidate prepares its canonical bytes
and checks caps/context validity before the zero-useful-record policy decision.
Positive useful counts retain the existing probe/compression path. A policy skip
has its own named receipt with no invented compressed size or lower bound; it
is never placed among evaluated candidates or proof-pruned candidates.

Runtime/Sparse ordering, exact cost ties, target identity, admission and final
publication/acknowledgement are unchanged. The repeat path still requires a
current receiver acknowledgement and retains the policy with an empty skip
list. Existing namespace poison, preflight versus attempted-failure handling,
receiver-state invalidation and explicit auxiliary reseeding remain shared code.
No new decoder, authorization permission, wire mode or received execution is
introduced. The policy can miss a smaller literal-only schema encoding; its
exact-decoding guarantee is not a byte-optimality guarantee.

Gateway `with_combined` and configuration `combined` explicitly remain
Exhaustive. Only `with_combined_policy` / `combined-cpu` opt in. Configuration
continues to reject unknown fields/modes, incomplete checkpoint/recipe/schema
sets and generic seeding for Combined. JSON preserves the existing cost-model
label and separately serializes selection_policy, policy_skipped, candidates and
pruned. The mode changes sender eligibility, not peer authentication or receiver
authority. Existing path-preservation and namespace-binding code is untouched.

## Focused evidence reviewed

Retained missing-API RED precedes implementation. The first attempted GREEN had
one erroneous fixture assumption: its public matched row actually emitted a
useful template. The retained correction uses an explicitly internal defensive
size-fallback case; it did not weaken or change production behavior. Subsequent
focused GREEN passes. The final scoped regression reports **73 passing tests**
(43 unit + 6 Combined + 10 schema codec + 6 prepared schema + 5 schema reuse +
3 schema sender),
including the cap/poison negative. Recorded strict Clippy and final format exits
are zero. The initial failed format receipt remains retained.

Tests check actual compress2 invocation counts for skipped, mixed-fit, exhaustive
and unseeded cases; fast/later-template counts, empty/literal/mixed streams,
record overflow and work/size fallback; canonical bytes against a frozen encoder;
default/explicit Exhaustive Offer/Data parity; and repeat acknowledgement.
The inherited frozen selection witness remains explicitly Exhaustive. Poisoned
context and oversized target failures propagate without compression.

The new gateway test uses real pinned mutual-TLS QUIC and a persistent endpoint,
checking Runtime, rolling Sparse, both exact-repeat cache hits, honest no-fit
receipts and stopped-store exact reopened bytes in both modes. Root's retained
gateway GREEN reports 4/4; its separate gateway-core suite reports 15/15. Root
owns final integrated execution; source review is not substituted for a run.
No expanded coverage or new benchmark was requested by this review.

## Reviewed source identities

All eight entries in `core-source.sha256` were checked against current files.

| Production file | SHA-256 |
|---|---|
| transfer/src/schema.rs | `1622926e757b8b9049ff3e78ae390a015240b110ff7e1480347463e1fe28f276` |
| transfer/src/schema_zstd.rs | `f61046139efd118f02920ea27a9eeb68104d8283f8860bfeaa8bc594873dd212` |
| transfer/src/schema_selection.rs | `de7eebda0df4333f5770033df93c16d0705aac9a09553c4f503ad561c578c89a` |
| transfer/src/session.rs | `21117f904f7806bcdae9a2e24420bde82aa68f12077b7fe87f80904bcb48822a` |
| gateway-core/src/object.rs | `b2268631e8a4b90946e6b19db86fbcdc7b5b5a87fa37b3344f0ba75dfaf97534` |
| gateway/src/object_service.rs | `9d031e3226ac36005788b5e7c1ab919429d752d3a3ab817225765d748134db4f` |
| gateway/tests/object_combined.rs | `1da5bfc43060e95919321ce47d8e1c51c56fd016014e0bd9e6985f4399da92d7` |

Paths in the table are under `crates/spiralcore-`. Performance, byte opportunity
cost and campaign custody require their separate measurement review; this source
review makes no speedup or unchanged-byte claim for the opt-in policy.
