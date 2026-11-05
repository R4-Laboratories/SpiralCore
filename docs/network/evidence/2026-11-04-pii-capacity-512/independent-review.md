# Independent scoped review: selected 512 KiB owned PII scanner

2026-11-04. Reviewer: independent Codex review task `pii_512_review`. Scope: selected Core PII source and candidate/public regression tests, the single Hostd NFA equality hunk, actual Aux admission and requested-allocation receipts. Read-only source/evidence review; no builds, tests, remote execution, source mutation, external comments or new agents. The only authored artifact is this review.

**Verdict: PASS for this bounded source/evidence delta; no blocking code, Rust, security or privacy/compliance-engineering defect identified.** This permits consideration of the exact selected change for integration. It is not validation of the destination source closure, installed activation, whole-process memory, two-endpoint capacity, native backend/platform behavior, WAN behavior, recovery, FIPS/CUI or regulatory certification. Those broader gates remain OPEN.

## Review authority and boundary

Revalidated logical roster `5e31d33c344d39b0` (14 agents / 14 skills). Checked actual `code-reviewer`, `rust-reviewer`, `security-reviewer` and `compliance-reviewer` names: all active. Read their profiles and the empty `.agent-sync/ROUTING.md` File Claims table. The task explicitly authorized the compliance role for this chat. INIT.md declares synthetic/public fixtures and no production/CUI/FIPS qualification; it declares no complianceScope. Privacy/data-minimization checks were applied to this delta, without asserting regulatory applicability or certifying the wider system. Source/build reruns requested by generic reviewer guidance were superseded by this task's explicit read-only/no-build boundary.

Reviewed handoff files byte-for-byte against their archive members. The entire Hostd `owned-reference-only.rs` body is custody reference only: **apply/review only the exact guard hunk; never overwrite newer collector/producer work with that body.** No live GitHub or roadmap status was inferred. The intermediate pure1025 dependency context is not SDK9a or newer upstream integration qualification.

## Source findings

- Core's owned query changes NFA 1,048,576 to 524,288 bytes. Its existing assertion changes accordingly; the only additional Core production-file material is a cfg(test) module declaration. Comparing the archived immediate before/after Core leaves shows exactly the two literal replacements; the candidate module was already present in the before snapshot used for the private experiment.
- Actual `curated_owned` obtains the query facts and calls the existing private `build_owned`. Both `RegexSetBuilder` and every per-rule `RegexBuilder` receive that NFA cap. Unicode remains true, DFA cache 262,144, nesting 64, fixed canonical pattern/name/validator data and selected ordering unchanged. Unknown/duplicate/oversize input and build failures still return static owned-policy errors; there is no fallback to legacy construction, rule omission, reordering, new public arbitrary-control API or copied scanner.
- Hostd accepts exactly NFA 524,288 together with its unchanged facts predicate. The scanner charge still derives from actual returned facts using checked multiplication/addition, with unchanged 8*N, 2*D, compiler overlap and fixed terms. Core query, actual builder and Hostd guard/model agree for the selected source.
- The private candidate module enumerates the 325 distinct nonempty ordered subsets of five rules, compares the actual `(masked bytes, rule-name sequence)` tuple to the legacy implementation, and performs representative/default/mode comparisons only for candidates that finish the constructor sweep. Diagnostic candidate rejection is not a skipped policy accepted for production.
- No changed unsafe product code, new panic path, policy/data logging, sensitive-value error, networking authority, retention behavior or live-object refund/repricing was introduced. Tests use bounded generated synthetic bodies. The Aux addition exercises the existing actual ledger with source-derived scanner/collector requirements rather than mirroring a standalone quota literal.

The preserved requested-workspace formula is a provisional logical declaration, not a proved aggregate allocator/RSS bound. Finite measurements support the reported observations and do not establish all-input, all-allocator or whole-process safety. This review does not turn preserved model multipliers into a formal physical-memory guarantee.

## Independently checked evidence

Archive SHA-256 and byte length match: 1,872,670 bytes; 124 members; all 123 listed member hashes match. Checked the relevant raw failures, stdout/stderr, exit files, source maps, build/Clippy diagnostic JSONL, binary custody records and measurement sources.

- Private sweep: actual protected test stdout reports 512 KiB PASS for 325 selections, 54 first/repeat representative searches and five default/mode cases. 256 KiB rejects at selection 5/order `[0,1,2,3,4]` after four successes; 128 KiB rejects at selection 3/order `[0,1,2]` after two. These are retained failures, not 975 passing constructors. Before/after maps match on all 583 supplied inputs.
- Aux RED/GREEN: both release/offline/locked commands use the same configuration. The old request is scanner 69,451,776 plus metadata 27,394,048 = 96,845,824 bytes; the actual 80 MiB prepaid ledger refuses with WouldBlock and test exit 101. Selected scanner 40,091,648 gives actual collector request 67,485,696, admitted with exit 0. The same test then refuses one byte past its actual remainder and a second Collector claim. Each arm's 5,235-entry before/after manifest is identical. Comparing RED to GREEN manifests finds exactly two changed inputs: Core pii.rs and the Hostd owned.rs guard. No test, ledger, lock or dependency input differs between the two arms.
- Recorded regressions: actual selected public-Core API binary passes three tests; selected Hostd passes eight AuxiliaryLedger entries and eleven owned-collector entries. The standalone new Aux invocation overlaps the eight. Scoped release Core(host-state library)/Hostd-library Clippy exits are both 0, with no compiler-message diagnostics; these are not all-target/native checks.
- Selected public allocator run: independently parsed all raw JSONL. Exactly 325 selection records have indices 1..325; all 325 unique orders equal the complete mathematical set of nonempty permutations. There are exactly 325 constructor rows, 325 drop rows and 325 verified equality rows; every postdrop delta is 0, not merely the reported maximum. Completion is present. Recomputed maxima: requested constructor increment 2,254,054 bytes; conservative realloc overlap 2,391,826; retained compiled allocation 1,938,125. Sample controls are NFA 524,288 / DFA 262,144 / concurrency 1, and both policies return 99,980 to 99,980 after outputs/scanners are dropped.
- The witness wraps successful System allocations only, uses checked sticky-fault accounting, does not allocate/log inside callbacks, reports conservative realloc old+new overlap distinctly, and observes one Linux measurement thread. Constructor comparison work is outside constructor measurement windows. The unchanged probe/permutations/accounting/allocator sources match the nested historical packet byte-for-byte. Nested historical raw rows corroborate the same 2,254,054 / 2,391,826 / 1,938,125 maxima and all postdrop deltas 0.
- Public witness source-before has 597 inputs; every original hash matches source-after. Source-after adds exactly the two expected protected executables, giving 599 entries. It is not literally an identical-sized before/after map. Source/control/runtime pins align. The protected object hash/check records are present. Original executables and the full source trees remain remote and were not independently reread/rehashed live by this reviewer; this review verifies their compact custody records and included source leaves, without claiming independent remote execution.
- The initial stale-lock failure (exit 101) and postbuild copy-path wrapper failure (exit 1) are retained and occur before their intended execution. The continuation uses the actual successful build target before the one selected measured public run; this is not evidence of a second silently substituted measurement.

The exact declared scanner envelope decreases 69,451,776 to 40,091,648 bytes (29,360,128 = 28 MiB). Measured allocator peaks are unchanged, so **28 MiB measured heap reclaimed is unsupported**. Recomputed selected constructor median CPU is 1,639,907 ns and destructor median 34,545 ns; the retained historical comparison is 1,667,529 ns / 32,802 ns. These nonrandomized historical/shared-host timings remain diagnostic, with a slower selected destructor median. No causal speedup, energy, send-only-time, transfer or WAN claim follows.

## Open integration/release gates

The supplied partial arithmetic is internally consistent: one endpoint subtotal 178,595,772 leaves 89,839,684 under 268,435,456; two endpoints total 269,111,160, already exceeding that tenant ceiling by 675,704 before remaining positive disjoint terms. This component change therefore cannot close #49 or establish two simultaneous mapped transfers. Native signal/guard supplements, TLS/carrier, reader/relay, Root/SDK/bootstrap and persistent-session terms still require a complete source-coupled worksheet and exact destination-source execution. No omitted term is treated as zero.

Full native Linux/macOS/Windows role coverage, real WAN, sustained/fairness/saturation/tail behavior, lifecycle/recovery and upstream delivery remain unexecuted by this task and OPEN. FIPS/CUI and broader legal/compliance claims remain separately gated. No new blocking finding in this bounded delta should be confused with closure of those pre-existing gates. Integration must requalify the actual coherent destination source, not transfer acceptance from this intermediate closure or copy over another owner's Hostd body.

## Exact source and evidence pins

All values below are SHA-256. Member names are relative to the reviewed archive unless stated otherwise.

| Item | SHA-256 |
|---|---|
| pii-capacity-512-receipts.tar.gz | `3492f095fe1432ebaa28d8e894f6fd4479327f262c633553201903847a0ccf39` |
| selected-source/pii.rs (handback Core) | `d5e850eb54be8c2313b934967fb30ca04649cff32105e5545a78c4a1fcb6b984` |
| selected-source/owned_capacity_tests.rs | `0eb6db8b8f43d69d633b3f3ec151b555e6a74cc1a69de0b03ad3ab147aa3eadc` |
| selected-source/owned-reference-only.rs (guard hunk only) | `c882da894ac7c95033cefc632553dd980f24bd39ef932c2bee3d59bab09c72d8` |
| selected-source/auxiliary-tests-reference-only.rs | `b1f210d8bd0602e93143efdc30e9f5c8ddfe5c546a959ab67711044761d0b56d` |
| selected-512.patch | `ff6e7ecd57f4eef9af322d7dc8f74a3a353bf3336d5dd5216c37fedfa673d7f0` |
| plan.md | `83697204aeacc934878007aa8b441e55ebe22cf0cb1fa9c5763ba339651f0c3a` |
| README.md | `46bda48ee2cc75cb061abf33b7b5cbf8d6cf9947bdd604c7e9c32e71df9a209a` |
| validation/private-sweep-locked/test.stdout | `d8e1cf06d0882efab9599af0509020da10377821814299f7c4e9f8dd694be691` |
| validation/aux-old-default-red/test.stdout | `32f1af0af081aba320aa8a4c1577076f42e51f7752dcbb33d57db8119645b281` |
| validation/aux-old-default-red/test.stderr | `4638f77cbe6ea651fb3d12f36b85550ce1acbfe3194783aa2d9cf89ef6ca9f3b` |
| validation/aux-selected-512-green/test.stdout | `a15ba8a38984632dddedd1899c0cb15f901c52e90395d50504927a9df7effa4c` |
| requested-witness/validation/selected-512/constructors.jsonl | `e1fcedf4d141c2b44bc802db63468d954220fc93ce66a69bc0202956f668c306` |
| requested-witness/validation/selected-512/sample.jsonl | `fabef511ba9ca70f36afa17bf9a6ed85e625ff29cd7a89f3cba663ba2872c196` |
| witness-source/Cargo.lock | `049a7bd0c2fa5fff09e92fdae4fa32d6f17b893a8ed737a32cddb7e27dbf82ec` |
| witness-source/compact-constructors.tar.gz (historical baseline) | `4528b175326b3e74e6b8e7773aebe3eeeeb1373728c239058f33149f67fe8417` |
| witness-source/probe.rs | `30aee48810f59899dc8e413ca31744ac354af44aac5b8f347549001ac847b90e` |
| witness-source/allocator.rs | `aca123497037d2a744176f5756d346a03c2f53f2c8f9e78614e5c461344eda92` |
| witness-source/accounting.rs | `5a165c3489d9e00ebb87e4e854a9366a001eacb2595445229c390cedb4b5a6b1` |

Recorded protected executable hashes: private Core test `871e812e86ff941f04324b33ef97b2458b473b547d55467bb6b286ef268c968b`; Aux RED `20eebdcd673abd94df6653645cf18e7526c17287ff5ec6f9b565eb5afd744f4a`; selected Hostd `0040f1a066109ff1fa26f8c0aff3ad164457e0c55c5d937bb310ae821e4f06ed`; public Core regression `e6080fff29d089bea9861cfcaffa7eb832f5d201985bc1e89fe3a80f837a1be0`; selected requested-allocation probe `f739f861f71730ecbbbde710e3bdce00f56c5840752408f088c9d7a84e4c5b19`; count verifier `39c701253168297bed81a179107480a67fb66fe79db25c5a5ad9415f058b650c`.
