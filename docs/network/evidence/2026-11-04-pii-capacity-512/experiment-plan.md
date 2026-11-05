# Proposed bounded capacity experiment: owned scanner NFA caps512/256/128KiB

2026-11-04. Source-only implementation/experiment plan for #49. **No product edit, Cargo/build, measurement, dependency change or512KiB acceptance result was performed.** Current owned1MiB semantics and all historical constructor/allocator/strict records remain unchanged. The recommendation is to test the fixed three-candidate set512/256/128KiB, not lower accounting independently of enforcement. No candidate is accepted by this plan.

## Decision and limits

Proceed later with a preregistered candidate set **524,288 /262,144 /131,072 bytes** for the owned NFA cap (current1,048,576), preserving the256KiB DFA cache cap, Unicode, nesting64, one concurrent scanner/search, all canonical patterns/order/modes/validators, and all refusal behavior. Couple Hostd's exact accepted-control predicate to the ONE selected qualified cap and derive its requested charge from the actual returned facts; do not admit a generic range. Legacy `with_rules`, `from_policy`, transcript/default fallback semantics and regex/automata versions remain unchanged.

512KiB can materially reduce the conservative DECLARED requested envelope by28MiB per maximum scanner;256/128KiB have larger conditional reductions below. It does not prove actual allocation will shrink: successful fixed machines may allocate the same bytes under either cap. It also does **not alone make two active same-tenant object endpoints fit256MiB**, as the current disjoint-term calculation below shows. A single failed admitted constructor rejects that candidate; no dropping/reordering rules, disabling Unicode, fallback to the old builder, silently accepting fewer selections or raising a different ceiling.

## Actual current source

Core `pii.rs` SHA `801446245c7121c15e845c7634a9002c76eb4f93feeb653f2f43a0dcb8873bee`; Hostd redaction wrapper `dcd47b70b14ff08dc5ce1a9eba79fddb243ed62ff1e65452da1ab02ab1920930`. The current collector/model and controller observation hashes are in source-observation.json; those files contain other owners' WIP and are not proposed wholesale sync inputs.

The private constructor is named **build_owned**, not from_controls. It receives the owned selection and requirements, passes size_limit to BOTH RegexSetBuilder and each RegexBuilder, and maps any build error into a static owned-policy refusal. The public curated_owned obtains the actual requirements then calls that private builder. Tests can exercise512KiB through the existing private unit-test access without a new public arbitrary-control constructor. An existing1-byte control test already proves the same builder refuses too-small limits.

The query's max selection has five rules plus one RegexSet: six regex objects. Empty/disabled safety cases compile the full canonical set; detect/redact/refuse mode does not alter the selected regex compilation machinery. Unknown modes/categories, duplicates and oversize category lists refuse. These admission/default semantics must remain unchanged.

Pinned primary regex1.12.4 source inspected on Wick:
`/home/callard/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/regex-1.12.4/src/builders.rs`.
Byte RegexBuilder size_limit is at1869 and byte RegexSetBuilder at2447; their documented oversized examples fail build. These approximate compile limits are not aggregate allocator/RSS caps. Public versioned docs endpoints were unavailable to the browser during this check; the actual pinned local source was used instead. No regex algorithm or source is copied/modified.

## Exact model arithmetic

Keep the existing conservative terms unchanged, with N=NFA bytes, D=262,144 and E=rule_count+1:

`per_engine = 8*N + 2*D + 1,048,576 + 40,960`

`compiler_overlap = 8*N + 1,048,576`

`scanner(E) = E*per_engine + compiler_overlap`

Use checked arithmetic and keep the exact-cap whitelist, not a permissive cap range. For E=6:

| Term | Current N=1MiB | Candidate N=512KiB |
|---|---:|---:|
| Per engine |10,002,432B|5,808,128B|
| Six engines |60,014,592B|34,848,768B|
| Compiler overlap |9,437,184B|5,242,880B|
| Scanner requested declaration |69,451,776B (66.234375MiB)|40,091,648B (38.234375MiB)|
| Reduction |—|29,360,128B =28MiB|

For fewer rules derive E from the same actual query. In general savings are `(E+1)*8*(1,048,576−N)`;512KiB savings are `(E+1)*4MiB`. Never separately mint a replacement or release live objects against a newly lowered value.

| NFA candidate | Per engine | Compiler overlap | Scanner6 declaration | Saving/scanner vs1MiB |
|---|---:|---:|---:|---:|
|512KiB|5,808,128B|5,242,880B|40,091,648B (38.234375MiB)|28MiB|
|256KiB|3,710,976B|3,145,728B|25,411,584B (24.234375MiB)|42MiB|
|128KiB|2,662,400B|2,097,152B|18,071,552B (17.234375MiB)|49MiB|

These are exact formula values conditional on unchanged source-model assumptions and actual successful construction. They are not measured candidate allocations or accepted scanner/whole-process bounds. Existing objects retain their original actual permit until Drop; no reprice/refund-in-place during service operation.

Provided per-collector non-scanner metadata is27,394,048B. At the maximum new scanner that is67,485,696B per collector before its threads/main/controller/carrier/codec, rather than96,845,824B. Two such metadata envelopes plus helper52MiB and its supplied Linux32MiB stack declaration sum223,051,776B. Relative to tenant256MiB, remaining headroom is45,383,680B =43.28125MiB. Old subtotal was281,772,032B, already13,336,576B above the tenant cap. The two-scanner saving is56MiB.

### Current disjoint terms still prevent a fit claim

Source `owned_collector_requirements` separately declares three2MiB usable worker stacks per VM; controller source separately declares an8MiB main envelope per endpoint. Current same-source Small/Ordered worksheet declares8,349,628B for a local sender+receiver pair, separate from TLS/carrier and reader stacks. If each of two selected endpoints has its collector and one pair:

| Remaining term | Bytes |
|---|---:|
| Two collectors' usable stacks only |12,582,912|
| Two endpoint main envelopes only |16,777,216|
| Two Small local pairs only |16,699,256|
| These three terms total |46,059,384|
| Headroom after those terms |**−675,704** (−0.64440155MiB)|

The256/128KiB candidates retain this same accounting structure:

| Candidate | Two collector metadata+scanner, helper52MiB+32MiB subtotal | Headroom before known46,059,384B terms | Remainder after those terms |
|---|---:|---:|---:|
|512KiB|223,051,776B|45,383,680B (43.28125MiB)|−675,704B|
|256KiB|193,691,648B|74,743,808B (71.28125MiB)|28,684,424B (27.35559845MiB)|
|128KiB|179,011,584B|89,423,872B (85.28125MiB)|43,364,488B (41.35559845MiB)|

Thus256/128KiB may leave materially useful room for remaining source-qualified terms if all admitted cases compile; neither establishes a fit. Do not subtract TLS/carrier, readers, relays or other positive terms as zero simply because the table stops there.

This is a partial DECLARED model counterexample to “512KiB alone fits two concurrent endpoints,” not observed physical use. It omits positive signal/guard supplements, TLS/carrier, codec reader/relay stacks and metadata, controller/bootstrap bookkeeping, Root/SDK state and other still-separate terms. Do not subtract those as zero or double-count an existing term: the supplied32MiB is the helper's usable-stack term, not either endpoint's main/collector stacks. Guest RAM remains its separately admitted domain. If actual supported concurrency uses fewer active components, state that envelope explicitly; do not call it two simultaneous mapped transfers. Do not raise256MiB to hide the gap.

## Existing finite evidence to preserve

Public packet: `docs/network/evidence/2026-11-04-owned-pii-requested/requested-allocation-receipts.tar.gz`, SHA `4528b175326b3e74e6b8e7773aebe3eeeeb1373728c239058f33149f67fe8417`,163638B,48hashed members+index. It pins regex1.12.4/automata0.4.14 and the Core1MiB source above. The external Rust witness lives at `/tmp/spiralcore-owned-pii-20261104/requested-witness` locally and corresponding owned Wick directory. Its harness lock SHA is `049a7bd0c2fa5fff09e92fdae4fa32d6f17b893a8ed737a32cddb7e27dbf82ec`.

All325 nonempty ordered selections of the five canonical rules (`5+20+60+120+120`) actually compiled under1MiB and compared exact masked output bytes AND rule-name sequence on the same32KiB dense synthetic input. Every postdrop live delta was0. Peak constructor requested increment2,254,054B occurred at selection191/order[2,4,3]; independent max conservative realloc overlap2,391,826B; max retained1,938,125B. These are whole-scanner aggregate observations, **not** the size of any individual NFA/RegexSet, and therefore cannot establish that512KiB will compile all325. In particular, the largest ordering cannot be replaced by the canonical full-five order alone.

The original exactly325-constructor run and the separate two-policy first/repeat sample retain their precise original identities. There were325 measured constructors, not all search inputs, modes-by-order executions, all architectures or a whole-process proof. The earlier cached build receipt remains explicitly cached; new source must emit a new protected object. No historical receipt is retagged512KiB.

## Sharp next bounded experiment and minimal implementation

1. **Freeze/no input overlap.** After the current status/SDK coherent-source work completes, obtain an explicit free jobs1 lane and owner handoffs. Use an owned isolated source snapshot and existing cache; never sync Core/Collector while another command consumes that closure. Record current Core/Hostd/regex/automata/manifest/lock and witness hashes. Preserve1MiB baseline evidence/model untouched. No dependency refresh, VM or new dataset.
2. **One internal Core candidate test, no public controls API.** Add focused cfg(test) coverage calling the existing private build_owned with actual requirements copied and NFA set to each of512/256/128KiB. The SAME canonical selected patterns/options go through the actual RegexSetBuilder and per-rule RegexBuilder. For each candidate run all325 SAME ordered selections and exact legacy masked-byte/rule-name comparisons on the existing32KiB dense body. One compiled Core test object can exercise the three actual controls without exposing mutable controls publicly or recompiling three production snapshots. Record each candidate separately. Any selection build failure rejects that candidate; retain the exact first failure/partial count and never call it325PASS. No selection may be removed or reordered to pass.
3. **Representative bounded search for EACH passing candidate.** Use the existing three32KiB dense/Unicode-nearmiss/ASCII families and original two sample policies, same scanner/one receive thread, first+eight repeated searches, checking exact legacy output. Include existing default/disabled/refuse/detect semantic tests and tiny-cap/unknown/duplicate refusal. This is finite representative coverage, not every search input/cache state. The maximal constructor191/order[2,4,3] is already among the325 and cannot be substituted by the full-five case alone. The candidate sweep has at most975 constructor comparisons plus finite fixed search sample; report them as325policies×3controls, not975independent policies or VM capacity.
4. **Choose ONE source-coupled candidate.** Select the smallest of the fixed candidates that passes ALL325 and representative/default/refusal cases. If none passes, reject this route without raising another ceiling or narrowing redaction. Before production changes, record a genuine existing-Aux admission RED for the old scanner+collector charge using the SAME prepaid domain with a fixture budget between old and selected candidate totals (e.g80MiB is above67,485,696B and below96,845,824B). It is an actual component claim/refusal boundary, not an independent quota or two-VM witness; no sole literal-mirroring test.
5. **Minimal selected product delta only.** Change the owned requirements cap to the ONE selected value, adjust its existing exact-control assertion, and change only Hostd's accepted-NFA equality to that same value. Keep cache256KiB, all formulas/multipliers/fixed terms, patterns/options/modes/validators and refusal/legacy paths untouched. The actual Core builder already consumes the query. Preserve checks before allocation and actual same-pool permit lifetime; no reprice/refund of existing scanner objects. Record a managed-model source revision, not a new network envelope/capability.
6. **One instrumented325 public-constructor execution on the selected source.** Reuse the reviewed external Rust/System requested-allocator witness against actual candidate curated_owned, the same generator/order/input/features and pinned dependencies. Emit queried control facts outside measured windows; protect/hash the NEW binary. Run --all-constructors once on that selected default, require325unique completed rows and exact legacy tuple equality for all. Run the existing two-policy search sample once against that same object to observe retained first/repeat caches. Other controls were tested through the actual private builder; do not label them public-default deployments or measured allocator profiles. Reporting/legacy comparison stay outside measurement windows; no scanner clones/thread migration.
7. **Source/model/qualification checks.** Retain successful-allocation-only counters, sticky faults, requested live/peak, conservative realloc overlap, first/repeat coexistence and EACH postdrop delta==0 (not only a maximum that could hide a negative delta). Check original baseline plus increments, count/completion/byte results, old/new protected objects and unchanged source/lock. Compare actual requested witnesses with the selected formula proposal, preserving fixed compiler/cache terms and source rationale. Relevant Core/Hostd/gate regressions and scoped strict must pass; obtain independent source/security/privacy delta review. CPU/wall are separate observed diagnostics, with no speedup/energy/RSS claim. Final installed profile remains gated on remaining disjoint source terms and actual native/all-Git closure qualification.

## Falsifiers / stop conditions

- Any existing admitted selection/order, default/disabled safety case or mode fails at a candidate cap: reject THAT candidate; any category/rule is omitted, reordered to pass, or falls back.
- Any legacy/owned byte or rule-name sequence difference; altered validator, Unicode, cache cap, nesting or concurrency.
- Model guard/query disagree or charge is lowered without the actual builder enforcing the selected cap. Existing live objects are refunded/repriced before their true lifetime ends.
- Fewer/more/duplicate325 rows, absent completion, counter fault/nonzero postdrop delta, source/binary/lock drift or accidental multiple Core sources.
- Newly observed aggregate requested peaks/search retention violate the proposed declared model, or source multipliers/fixed compiler/cache terms no longer support the changed cap. Controls alone and finite observations are not a formal RSS/native-allocator bound.
- Any statement that this change alone establishes two active mapped VMs inside256MiB. The partial model above already falsifies that statement under current disjoint declarations.

**Recommendation:** after current actors finish their coherent closure, run the fixed512/256/128KiB private-builder sweep, choose the smallest fully compatible candidate, and measure ONE selected public constructor profile. This is a sharply bounded real next capacity change, not a permanent69MiB HOLD. Conditional per-endpoint savings28/42/49MiB are material, but neither constructor success nor the remaining partial headroom completes #49. No different optimization, parent-limit increase or arbitrary cap search is authorized by this plan.
