# Owned PII split NFA controls: Set512 / Rule128

2026-11-04 bounded component implementation under Architect ADR454f22e674cdacb72a13df7e132b1d0afe1354cea546530cc7b322ba3967c4a6. **Functional and scoped strict qualification is complete in the isolated intermediate source context; independent implementation review, current SDK/consumer integration and native acceptance remain required.** No primary checkout, SDK or GitHub publication was changed by this author.

Core now reports and enforces two source-coupled construction controls: the existing `nfa_size_limit_bytes=524288` belongs to the one combined RegexSet; the appended public `rule_nfa_size_limit_bytes=131072` belongs uniformly to each canonical individual Regex. Unicode, patterns/order/validators, DFA262144, nesting64, concurrency1, legacy/default constructors and refusal semantics remain unchanged. Public curated_owned obtains its own actual facts; callers still cannot inject mutable controls into that production constructor.

Hostd queries those same facts and accepts BOTH exact caps. Its one private checked formula prices one Set+kRules plus compiler overlap using max(Set,Rule), preserving every8x multiplier, each2xDFA term and all fixed terms. No public arbitrary-profile/quote/quota constructor or permit repricing was added. All original same-pool admission and actual permit lifetimes remain unchanged.

## Exact production entry points and visibility

Paths below are relative to the frozen source root `/var/tmp/spiralcore-pii-dual-20261104/source`:

- PUBLIC Core facts: `mvm_core::pii::PiiRedactor::owned_pii_requirements(&PiiPolicy) -> Result<OwnedPiiRequirements,PiiPolicyError>`, `crates/mvm-core/src/pii.rs:197`. Both facts are emitted by the actual query.
- Actual Core enforcement: private `PiiRedactor::build_owned`, same file264. The original SetBuilder receives the Set field; original per-rule RegexBuilders receive the new rule field.
- Hostd checked scanner quote: `telemetry_collector::owned_scanner_requested_bytes(&RedactionPolicy) -> anyhow::Result<u64>`, `crates/mvm-hostd/src/telemetry_collector/owned.rs:216`, **pub(crate)**. It delegates its actual StreamRedaction/Core query to the SAME private `checked_owned_scanner_facts(OwnedPiiRequirements)` at223.
- PUBLIC `mvm_hostd::telemetry_collector::owned_collector_requirements()` (owned.rs32, reexported by mod.rs) still returns separate non-scanner metadata27,394,048 and existing worker/stack facts.
- The actual combined checked collector metadata charge remains in original `InstalledEndpointResources::start_owned_collector` at `crates/mvm-hostd/src/installed_endpoint.rs:261-262`: `facts.known_metadata_bytes.checked_add(owned_scanner_requested_bytes(redaction)?)`, before original component claim/construction. **There is no new public standalone combined collector-quote API.** An external Root composer cannot call the crate-private quote directly; a later separately reviewed readonly wrapper could delegate to this SAME checked path rather than copy the model. This packet introduces no such API.

Adding a public Rust struct field can break exhaustive initializers/destructuring. This is not universally source-compatible, a stable ABI extension or a serialized wire/envelope change. The scoped actual consumer inventory and compiled closure are retained; a coherent all-consumer rebuild against one pinned Core is mandatory before final SDK integration. No gratuitous wire/runtime version field, public controls constructor or policy field was added.

## Genuine causal failures before implementation

1. Actual prior512 Core public query contract produced exactlyE0609 for absent rule_nfa_size_limit_bytes; source/rlib hashes match. This is API absence, not enforcement evidence.
2. Before any product edit, the actual AuxiliaryLedger60MiB fixture compiled and failedWouldBlock: prepaid62,914,560, actual old scanner40,091,648 and collector67,485,696. It used the actual original Hostd query/quote and same ledger, not a shadow price.
3. New guard/model tests then produced3E0425(private helper absent)+4E0609(new field absent), with no unrelated compiler errors. Only test wiring existed at this stage. The original raw diagnostics are preserved.

Both NEW Aux RED and GREEN arms deliberately use matched release/LTOoff, jobs1/incremental0. They do not retag the predecessor's LTO qualification or create a cross-profile performance comparison. The local staged probe's first rustfmt invocation could not resolve sibling accounting.rs because it was outside its harness; formatting with skip_children corrected that staging prerequisite without a product/build/measurement failure. No measurement was silently rerun.

## Fresh split-source results

- SAME actual Aux60 fixture GREEN: scanner24,363,008, collector51,757,056 admitted; remainder11,157,504 and actual remainder+1 refusal; duplicate Collector claim refuses. Original source/protected binaries verify. Nine auxiliary regression entries pass, including this new case; its standalone invocation overlaps those9.
- Actual SAME production guard/model:2 tests pass. Thirteen independent unsupported facts refuse, including wrong Set, wrong Rule, cache/nesting/concurrency/counts and overflow-shaped inputs. Actual query/model quotes for k1..5 are13,713,408 /16,375,808 /19,038,208 /21,700,608 /24,363,008. Eleven existing owned-collector tests also pass.
- Actual Core17 PII entries pass. New independent stage tests force only Set to1 with Rule128 unchanged, then only Rule to1 with Set512 unchanged; BOTH refuse with static canonical errors. Fresh public split defaults construct all325 ordered policies and match exact legacy byte/name tuples, plus54 first/repeat searches and5 default/mode cases. Legacy/default/disabled/unknown/duplicate/oversize/Luhn/IBAN regressions pass.
- Historical uniform private candidate helpers in this NEW context explicitly set BOTH fields; they retain the same uniform512 success and256/128 refusals. Original predecessor and singleton packets/binaries remain unchanged. These new test runs do not relabel old treatments as split defaults.
- SAME public field contract now builds and executes against the NEW actual Core rlib. Seventeen Core checks have584 supplied source inputs stable before/after. Final Hostd scope has5,237 supplied source inputs stable.
- Scoped release/LTOoff Clippy -Dclippy::all for Core(host-state library) and Hostd library BOTH exit0: zero emitted compiler warnings, zero ordinary `warning:` lines in stderr. Aux RED/GREEN builds also have zero ordinary warning lines. This is not an all-targets/current-SDK/native/platform acceptance claim.

## Exactly one fresh public allocation treatment

The reviewed Rust/System allocator witness retains its exact generator/order,325 policies,32KiB body, original counter logic and two-policy/three-input first+8-repeat sample. The minimal reporting update emits BOTH actual queried caps outside every measured construction window (325 control rows) and both sample policy windows. It uses unmodified public curated_owned defaults, never private test controls.

Exactly325 unique constructors and325 exact legacy tuples complete. Every individual postdrop delta is0. Both sample policies return99,980->99,980 requested live bytes. Query rows show Set524288/Rule131072/DFA262144 and counts k/k+1; sample concurrency1. New protected probe SHAb25d5004b1a01f5b7c0e18f703a3c3a8b6c2c325752f27d6224228b52f5ce778. All598 supplied source inputs and binary verify. The semantic325, historical-uniform325 and allocator325 overlap constructor space; they are not975 distinct policies or VM tests.

| Term | Uniform512 predecessor | Split512/128 |
|---|---:|---:|
|Maximum declared scanner workspace|40,091,648B|24,363,008B|
|Maximum declared collector metadata|67,485,696B|51,757,056B|
|Actual constructor requested peak increment|2,254,054B|2,254,054B|
|Actual conservative realloc overlap|2,391,826B|2,391,826B|
|Actual retained compiled allocation|1,938,125B|1,938,125B|
|Actual sample first-search peak increment|637,807B|637,807B|
|Actual sample repeated-search peak increment|65,623B|65,623B|

The declaration tightens15MiB per maximal scanner. **Observed allocation peaks remain unchanged; this is not15MiB of measured RAM reclaimed.** No live reservation is repriced/refunded.

Historical/new325 constructor medians: instrumented CPU1.639907/1.626602ms, wall1.640054/1.626696ms. Losses remain: the two-policy sample constructor CPU median increases1.448393->1.546166ms; first-search median320.6415->325.9965us. Raw observations/comparison preserve every phase. These are historical shared-host diagnostic observations, not randomized causal speed/energy results. No transfer, WAN or send-only measurement ran here.

## Formula and remaining capacity gate

Using actual queried S=524288,R=131072,D=262144,k<=5:

`Engine(N)=8*N+2*D+1048576+40960`

`Scanner=Engine(S)+k*Engine(R)+8*max(S,R)+1048576`

At k5: Set5,808,128 +rules13,312,000 +compiler5,242,880 =24,363,008. Non-scanner27,394,048 and every queue/helper/main/worker/native stack, carrier/TLS, reader/relay/bootstrap/session term remain separate and unchanged. These approximate engine controls and source-model coefficients are not a formal whole-heap/RSS bound.

The unchanged PARTIAL worksheet gives one endpoint162,867,132B (105,568,324B remainder), two endpoints237,653,880B (30,781,576B remainder) under268,435,456B. Missing positive terms are not zero; this proves neither profile fit nor native capacity. Root's next budget composition must use source-coupled supplements and actual native execution. The complete two-endpoint, sustained/fairness/recovery/platform/WAN/upstream/FIPS gates remain OPEN.

## Frozen handoff and integration boundaries

`handoff/dual-source.patch` is based on the immutable selected512 leaf and contains only the Core facts/builder/tests, Hostd exact guard/checked model/test wiring and appended Aux60 test. Hostd whole before/after bodies are CUSTODY REFERENCE ONLY. Apply narrow guard/model and test-declaration hunks to Session's actual newer producer; never overwrite its entire owned.rs. The exact source/model must be reviewed afresh by Code/Rust/Security/Compliance and rebuilt in the current coherent SDK/consumer graph.

No registry dependency/version refresh occurred. This inherited canonical intermediate source/pure1025 closure is not the newer upstream integration base; old qualifications do not transfer. Source snapshots, original binaries, libraries, normal Cargo targets and raw records remain on Wick. Only compact source/receipts were copied locally; no Mac Cargo/raw VM/cache store was created. No SDK push, primary merge or full-goal completion is claimed.
