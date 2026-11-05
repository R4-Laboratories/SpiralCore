# Independent review: owned PII split NFA implementation

2026-11-04. Reviewer: independent Code/Rust/Security/privacy-compliance review task `pii_512_review`; separate narrow Architect judgment on a proposed visibility-only API delta.

**Verdict: PASS for the frozen split-control component source and bounded execution evidence; no blocking code/security/privacy defect found. One nonblocking evidence-description finding is recorded below.** Current SDK/consumer integration, native activation and full endpoint capacity remain unqualified. This review does not close platform/WAN/recovery/FIPS or broader delivery gates.

Read-only review of `/tmp/spiralcore-pii-dual-20261104/README.md`, handoff before/after/patch/pins, the full compact archive member index and relevant raw source/results/maps/scripts, against ADR `454f22e674cdacb72a13df7e132b1d0afe1354cea546530cc7b322ba3967c4a6`. No builds, test execution, source/snapshot mutation, new agents or external comments. Original remote executable/source custody records were checked as records; the remote originals were not independently reread or executed. Only this review report was authored.

Roster `5e31d33c344d39b0` revalidated; Code/Rust/Security/Compliance role checks all active, empty File Claims table. Compliance role was authorized for this chat. Privacy-engineering review is limited to this sensitive-data-scanner delta and synthetic fixtures, without legal applicability rulings or regulatory certification. Previously read INIT scope does not authorize production/CUI/FIPS claims.

## Source verdict and ADR agreement

- Core appends the one documented `rule_nfa_size_limit_bytes` fact and returns131072. Existing `nfa_size_limit_bytes` stays524288 and is explicitly documented as the combined Set cap/maximum cap of this profile, not aggregate heap/RSS. Public `curated_owned` still gets its own actual requirements; no public caller-injected controls path was added.
- The actual original RegexSetBuilder retains `.size_limit(requirements.nfa_size_limit_bytes)`; every original individual RegexBuilder now consumes `.size_limit(requirements.rule_nfa_size_limit_bytes)`. Canonical rule definitions/order/validators, Unicode true, DFA262144, nesting64, concurrency1, default/disabled/detect/redact/refuse selection and legacy constructors are unchanged. Build failure remains a static owned-policy error without fallback, category omission, input echo or rule reordering.
- Hostd's existing policy entry queries the same Core facts through StreamRedaction and delegates to one private checked helper. The exact guard requires both selected caps, original cache/nest/concurrency and k in1..=5 with checked k+1 engine count. Checked model arithmetic prices one Set, k individual Regex objects and compiler overlap at max(Set,Rule), preserving8x NFA,2xDFA per engine,1MiB+40960 fixed terms and1MiB compiler fixed term. No duplicate cheaper quote, new resource authority or live-object repricing was introduced.
- The unchanged non-scanner collector query and resource/permit custody remain separate. The narrow code patch does not change the admission implementation or physical worker lifecycle. Whole Hostd owned.rs before/after files remain custody references ONLY: apply relevant scanner/test hunks onto the current owner's newer producer; never overwrite that owner's whole file.
- The public split-default tests call `curated_owned` with unmodified actual query defaults. Distinct tests mutate only Set to1, then only Rule to1 and require static refusal. Historical private uniform-cap helper now explicitly changes BOTH fields, so old candidate treatments retain their meaning. Historical packets and prior review/ADR are not relabeled.
- New production arithmetic is checked; no changed unsafe code, new production unwrap/panic, payload logging, network envelope, cryptographic primitive, capability/DNS/rate/audit bypass or persistence policy appears in the delta. Test assertions use bounded synthetic inputs and static labels. This is a source-coupled model revision, not a memory proof or compliance/security attestation.

## Independently verified execution evidence

Archive `dual-nfa-receipts.tar.gz`:1,842,162 bytes,133 members,132 hash entries. Independently recomputed archive SHA and every member hash; all match. Every handoff before/after source hash matches pins.json and its archive member. ADR member hash matches the approved ADR.

**Causal RED/GREEN:**

- Actual prior512 public contract fails exactlyE0609 for the absent rule field; recorded source check succeeds. New actual Core contract builds and executes with both fields. This is API-shape evidence, distinct from builder enforcement.
- Actual Aux60MiB RED build succeeds and test exits101 with WouldBlock from the actual ledger: prepaid62,914,560, scanner40,091,648, collector67,485,696. New split source admits the SAME fixture at scanner24,363,008 / collector51,757,056. Its code then refuses actual remainder+1 and duplicate Collector claim. Remainder is11,157,504. No shadow quote, extra ledger or test refund exists.
- Aux RED before/after maps have5,235 equal inputs; GREEN has5,237 equal inputs. Comparing RED to GREEN shows only the planned Core source/helper-test and Hostd scanner/helper-test changes. The actual Aux60 test source itself is identical between those arms, as are dependency/lock inputs. Both retained executed wrappers set release/LTOoff, jobs1/incremental0; this is not transferred predecessor LTO acceptance.
- Additional guard API RED has only the expected3E0425 and4E0609 compiler errors. The real private production guard/model's2 tests pass after implementation. Thirteen unsupported facts are rejected; overflow-shaped facts fail the guard, so this is not claimed as dynamic exercise of arithmetic-overflow branches. Checked arithmetic is verified by source inspection. Actual query/entrypoint values for k1..5 match13,713,408 /16,375,808 /19,038,208 /21,700,608 /24,363,008.

**Semantic/control checks:**

- Actual Core17 PII test entries pass, including fresh public split-default325 exact byte/name comparisons,54 representative searches, five mode/default cases, separate builder-stage refusals and existing validation/Luhn/IBAN/default/disabled/Unicode/refusal cases. Historical uniform512 success and256/128 actual rejections remain separately labeled. Core's584 source inputs match before/after.
- Actual selected Hostd9 AuxiliaryLedger entries,2 guard/model entries and11 owned-collector entries pass. The standalone Aux60 invocation overlaps the9, not an extra unique regression.
- Scoped Core(host-state library) and Hostd-library Clippy commands exit0. Both JSONL files have no compiler-message diagnostics; both stderr files have zero ordinary warning lines. This is scoped release/LTOoff qualification, not all-target/current-SDK/native qualification.

**Fresh public allocator treatment:**

- Independently parsed all raw selected JSONL:325 selection indices exactly1..325,325 unique orders equal the entire nonempty permutation set of five canonical rules,325 constructor rows and325 verified exact legacy tuple rows. Every postdrop delta is individually0; completion exists.
- All325 control rows have actual Set524288/Rule131072/DFA262144 and k/k+1 counts consistent with their selections. They are emitted outside measured constructor windows. The witness uses original public defaults without private control injection.
- Recomputed maxima: requested constructor peak increment2,254,054 bytes; conservative realloc overlap2,391,826; retained compiled allocation1,938,125. Both sample policies finish at99,980->99,980 live requested bytes. Samples report concurrency1 and both caps. These finite observations do not bound all input/cache/native allocator/process states.
- The actual probe change only adds/asserts/reports the second query fact and per-selection control rows outside measurement. Its allocator/accounting/permutation/lock/PLAN/count-verifier sources match the prior reviewed witness byte-for-byte. Measurement phases and legacy comparison placement remain unchanged. New public source-before598 inputs all match source-after; only the two expected protected executable paths are added (600 after).
- Original protected selected probe hash is recorded and checked as `b25d5004b1a01f5b7c0e18f703a3c3a8b6c2c325752f27d6224228b52f5ce778`. It is distinct from prior uniform512. Constructor median CPU recomputed1,626,602ns. Same allocation maxima and retained losing sample timing observations prohibit a measured RAM-reclaimed or causal speed/energy claim.

The declared scanner reduction40,091,648->24,363,008 is15,728,640=15MiB per maximal scanner, not15MiB of measured heap released. Actual peaks remain unchanged. Semantic325, historical-uniform325 and allocator325 overlap the same policy space; they do not establish975 distinct policies or VM capacity.

## Nonblocking finding: command description drift

**LOW / evidence-description correction:** `validation/old-uniform-512-aux60-red/command.txt` and its GREEN counterpart describe the inherited pii-capacity config path and omit `CARGO_PROFILE_RELEASE_LTO=false`; the actual retained `run-aux.sh` invokes the dual-root config with that environment setting. `validation/dual-core-tests/command.txt` likewise omits that setting and contains an unexpanded literal `$root` in the descriptive config argument. These strings are printed descriptions, not the execution statement.

The retained actual scripts, source maps, package paths, source hashes, matched RED/GREEN results and configuration context resolve the executed scope. This does not block the bounded source/evidence conclusion. Preserve hashed original receipts; append a short correction naming the actual script/config/environment before publishing the packet. Future producers should generate exact argv/environment receipts from the actual invocation instead of maintaining a second descriptive string. Do not claim the existing command.txt files are exact executed argv, and do not rebuild solely to rewrite them.

## Consumer/source and remaining gates

The included inventory is scoped to the intermediate frozen source closure. A public Rust struct field addition may break exhaustive initializers/destructuring; the README acknowledges that. Recorded final source status identifies one actual Core package source, regex1.12.4/automata0.4.14/syntax0.8.11 and unchanged lock `61a991aac9a2bd748e2fbffb48ab863b7bf7b59ecf9447fd8ab18af6cbb9325b`. This does not inventory/rebuild every current SDK/external consumer. Current coherent consumer integration and exact release-source validation remain required; this intermediate pure1025 context cannot transfer qualification to the newer upstream/SDK graph.

The partial two-endpoint subtotal237,653,880 leaves30,781,576 under268,435,456 before still-separate positive native, Root/helper, carrier/TLS, reader/relay, bootstrap and session terms. This is conditional source-model arithmetic, not capacity or native RSS proof. Full budget composition, native signal/guard terms, two simultaneous endpoint execution, sustained fairness/backpressure/recovery/tails, Linux/macOS/Windows role coverage, WAN, upstream delivery and FIPS/CUI remain OPEN. No missing term is zero.

## Narrow Architect judgment: proposed public scanner query

**APPROVE the visibility-only design, subject to actual focused RED/GREEN and coherent source checks. No such exposure is implemented or accepted by this frozen packet.** Change the existing `owned_scanner_requested_bytes(&RedactionPolicy) -> anyhow::Result<u64>` from pub(crate) to pub and move the SAME item from the crate-private reexport group to a public reexport. Keep the implementation and private `checked_owned_scanner_facts` helper exactly on the existing checked policy-query path. No wrapper with a copied formula, alternate facts input, new grant/model or public arbitrary-controls API is needed.

This exposes read-only requested-workspace data so Root can compose its aggregate worksheet using the same policy/source revision that admission uses. It does not reserve credit, admit an endpoint, release/refund a permit, authorize construction, establish native heap use or replace later actual admission. Document those semantics and unsupported-policy/profile error behavior on the public function. Preserve Result propagation; an external aggregate composer must reject errors, use checked additions and retain all disjoint terms rather than substitute zero or a guessed cheaper value.

Root reports a separate copied snapshot for the actual external Rust caller regression, with baseline API-RED building and no visibility edit yet. The proposed default scanner24,363,008 plus separately queried collector27,394,048=51,757,056 check and unsupported-category refusal are appropriate narrow external caller checks. Compile/execute those against the actual new public reexport, verify final exact source/library identity and applicable consumer/library checks, then inspect the small visibility/reexport/docs diff. Existing private tests already cover k1..5/model mismatches. Pure visibility/docs changes do not by themselves require another expensive allocator325 run when the underlying implementation/source inputs remain otherwise identical; do not reuse this packet's source hashes as hashes of that future API delta.

Public exposure changes Rust API availability only, with no MVM wire/envelope/capability version or new authority. It is narrower and safer than an external copied accounting model. Root's aggregate result still requires complete terms and native qualification before a fit claim.

## Exact source/evidence identity

All values SHA-256. Handoff source paths relative to frozen source root.

| Item | Hash |
|---|---|
| dual-nfa-receipts.tar.gz | `935a55ae9409cf9de0e70676c88fcd165475d85bfc1bb7635dcd6c69879907a4` |
| README.md | `8ebecffa34e363178b6560606489fa318c5fc9f855970de5d8f2c214b6802c27` |
| handoff/dual-source.patch | `7d79d6e1eb0adb978c5f0b0835a76a571512887b4a68cf822ebee623a9a95387` |
| approved ADR | `454f22e674cdacb72a13df7e132b1d0afe1354cea546530cc7b322ba3967c4a6` |
| Core pii.rs before | `d5e850eb54be8c2313b934967fb30ca04649cff32105e5545a78c4a1fcb6b984` |
| Core pii.rs after | `8b8e5b1289abc1231e677b0a6dab7952d32d9263411480d9e84698360ec01547` |
| Core owned_capacity_tests.rs after | `b25d7fd99e8c1f1bf1f4cba1609c4963735306c1f6032402c03c3088615a4302` |
| Core owned_dual_tests.rs | `7491f4634c0ede85cdf8ad0d41793cc2c1f05986c6b3872c0523afba1eed1e47` |
| Hostd telemetry_collector/owned.rs before | `c882da894ac7c95033cefc632553dd980f24bd39ef932c2bee3d59bab09c72d8` |
| Hostd telemetry_collector/owned.rs after, CUSTODY REFERENCE | `60242a6563491a16a295d92fa4c7a594c3b0445fb4cda3bfc5331f9bada2d9e1` |
| Hostd owned_dual_model_tests.rs | `01c9901e02fbaa52d93ba23904434b25bdb34f9b1b959b227137d61f92f35280` |
| Hostd installed_endpoint/auxiliary_tests.rs after | `a6bc4951c4da0bdfd627f53f083c718a34700f7cd99afd29ea617c0b3179e1dc` |
| public allocator constructors.jsonl | `5c5d245d672da67c5af90d5dc882cbc0547f0c6773d099c7d525f1a10727931f` |
| public allocator sample.jsonl | `49365d27a7b8bf2d19bce7407f09aa8268aa4fff8625e90539fe68e504853500` |

Recorded protected objects: AuxRED `5f660dbcd59c002905e034f25c366b0e049c14f5979cec9815b44b2e97ed92e5`; AuxGREEN `f6b8b2930fa8af6318233a8d782f66d8e4b1a4f2448d0b147059140284843073`; Core17 `2d327621324aac9a422f23cfb62c7ec436fb8bb3d1d727a0edcf301616411dbe`; public allocator probe `b25d5004b1a01f5b7c0e18f703a3c3a8b6c2c325752f27d6224228b52f5ce778`; Core public field contract `7e432eb6bbe06ff117cc18ed1effa6d44c5b5850362f8081957f9fe989463daf`.
