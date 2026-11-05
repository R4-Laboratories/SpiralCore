# Independent B1 pure-codec review

2026-11-04. Reviewer `/root/closed_recovery_review`, independent of the B1 author. Code/Rust/Security review of the exact frozen pure-codec checkpoint, its contract, shared v1 row extraction and retained evidence. No source edits, builds, test executions, guests, new agents, publication or B2 implementation were performed. The original snapshot is preserved.

**Disposition: canonical-format and inverse semantics supported within the reviewed producer paths; CONDITIONAL HOLD on complete bounded B1 acceptance pending the identified P2 pre-clone ordering fix and its focused causal regression.** No additional canonical/inverse/legacy-compatibility blocker was found in this scope. This is not a demonstrated external security or admission bypass. B2 activation remains separately blocked on its own constructor/capacity and lifecycle gates.

## Reviewed identities

Freeze: `/tmp/spiralcore-retained-catalog-20261104/stage-b/b1-first-freeze/`.

| Artifact | SHA-256 |
|---|---|
| `source.tar.gz` | `502620604f55099765737b73ec35db8b4e95fc0e152f7bae749e0995e86b4ed2` |
| `source.sha256.json` | `77afd5670f44b6641d4a0bd1356b11214c7f61976635aa86356215d0dfcba25f` |
| `compiled-inputs.json` | `0a0b976bf95fc6afe4422f338a29b1bc39d79cd63342e1d1eb92db79c8f1da3b` |
| `contract.md` | `1e504e444d6554b587b4c653f2de413f1034b82b984c2866badd43aacbab6ea1` |
| `pre-clone-note.md` | `c72705a6b4d6db49ba3e8f42710caf2d4374bbea3f3249395e44eadc683516b4` |
| `metadata-note.md` | `cc786530131070900a5efaa872a10e354e1e08d8b4c4a481f7785b85ca0bfa90` |
| `golden-provenance.json` | `bcf2d066d2ae42136821d01f347c83ccdd265e910bf0bfcfc186d9edc4a53b6b` |

All five current mapped source/golden files match the frozen map; all five archive members match those hashes. All 138 current compiled-input entries match the frozen closure. The final Wick source-before/after maps are byte-identical to that same compiled-input JSON. Source hashes, relative to `crates/spiralcore-mvm-live/src/`:

| Input | SHA-256 |
|---|---|
| `installed_owner/retained_telemetry.rs` | `344856b284749d2743a883d93da3b535d24d1ba1a0769b6a62b554f84566b1c4` |
| `installed_owner/retained_transfer.rs` | `374aeeacc23a60a6dfd531797619109f6996b3a881251a4c53063904ffd9a426` |
| `installed_owner/retained_transfer_tests.rs` | `52aca789989484a87b8ea746951f9667731cd84ec6e759d96f389f36ee64a6bf` |
| `installed_owner/retained_v1_golden.bin` | `19aae811031be4032e46973677b534741c070f536c80095e7e9be8347f546111` |
| `installed_supervisor.rs` | `f87e52a5ea79430353d917ae7b97605a3f812a6d45dc3db717d8f63fb583f397` |

A Team roster `5e31d33c344d39b0` and exact code-reviewer/rust-reviewer/security-reviewer eligibility checks were independently refreshed successfully. The review uses the frozen closure rather than attributing concurrent worktree state to a single Git HEAD. No package/version/lock/dependency graph change is introduced by these five inputs.

## P2: validate the private catalog before reverse cloning

**File:** `installed_owner/retained_transfer.rs:186-217`.

The author note is correct. `reverse_source` initially checks only installation equality. Its v1 branch then clones every RetainedRecord/PathBuf into a fresh Vec before the destination v1 encoder rejects excessive row count, malformed rows or duplicate membership. Its v2 branch clones the entire entries Vec before the destination v2 encoder validates the restored header/origins/rows. A directly constructed private CatalogV2 with more than 16 entries, or a row path exceeding 4,096 bytes, can therefore request clone storage before rejection. `encode` itself bounds row count before allocating its raw output but checks individual row/origin/duplicate validity after that allocation.

This is a real input-order/resource-contract violation, **P2** in the present private, unconnected codec. `decode` and `from_v1` produce bounded rows, and the current `verify_reverse` first decodes bounded target bytes, so those reviewed producer paths do not expose an unbounded decoded-input clone. No grant, file authority, native receipt or open admission follows from this behavior. The ten retained tests do not establish the missing ordering guard. The pure-format checkpoint may be retained as evidence; it must not be called fully bounded/ready or consumed by B2 on the assumption that every private value is structurally qualified.

**Minimal correction:** extract a single nonallocating structural validator from the v2 encoder. Check the current key/generation, <=16 rows, supported mode, origin installation/profile/generation constraints, `record.valid()` and existing duplicate origin-tuple/session/path/state-inode rules. Invoke it before encoder allocation and before either reverse cloning branch. Preserve the separate reverse source-version/mode/origin and restored-old-header checks; destination encoders still provide their existing semantics. Preserve valid empty-catalog behavior if currently supported; the intent's separate 1..16 row constraint must not silently redefine the catalog codec. Do not change quotas, layouts, domains, canonical bytes, v1 rules, I/O or authority.

The proposed test-only clone-boundary hook is appropriate if placed immediately before each actual cloning operation, scoped thread-locally and unreachable in production. For both reverse branches, an oversized row count and oversized path should refuse before reaching that boundary. A valid control must reach each boundary so a vacuous hook cannot pass. Retain causal RED against this frozen implementation, then minimal fix/GREEN plus the existing ten selected format regressions and scoped Clippy under a separately authorized lane. Such a hook observes control ordering; it does not measure allocator requests, RSS or native exploitability.

## Canonical v2 and intent semantics

Source inspection supports the following, subject to the ordering finding above:

- SCRETN02 uses the 88-byte header, 4,352-byte rows, <=16 total rows and maximum 69,752 bytes. The decoder checks short/oversize input, exact count/length/header identity/padding before row allocation. Count arithmetic is safe after the <=16 guard. The mode tag has only Active/StoppedRetained. The exact v2 domain and final canonical re-encoding prevent alternate encodings accepted under the same expected epoch.
- Base row bytes 0..4296 preserve the v1 field order, endianness, phases, exact R/K limits, path rules, File IDs and records seal. Origins occupy 4296..4336; remaining 16 row bytes must be zero. Origins must share installation, have nonzero profile/generation and not be newer than the activation generation. Duplicate origin/profile/generation/serial tuples refuse, while serial reuse across distinct origins is allowed. Session/path/state-inode uniqueness stays global.
- SCRETX01 is exactly 768 bytes, with fixed source and target versions, kind/mode tags, zero regions, SHA-256 domain, exact embedded 176-byte owner records, catalog lengths/count and three original object-ID pairs. Decode guards all fixed slices before parsing; reserved bytes and trailing/short data refuse. Embedded owner records canonical-decode and bind the same installation, profiles/generations and nonzero original object IDs, with both phases Active.
- Restart requires checked old+1 generation and same installation. RetainedStop requires identical epochs; canonical owner records and equal object IDs then imply identical old/target owner record bytes. Target mode follows the kind. A numeric StoppedRetained value is only format metadata; it is not proof that a backend or collector stopped.
- `verify_reverse` validates intent shape, target file length/hash, exact target epoch/mode/count, then reconstructs the source version and checks its exact full-file length/hash. Ordered membership and all base/origin fields are preserved by inverse encoding. V1 reconstruction refuses mixed origins and restores the v1 zero tail/domain; v2 reconstruction restores the old header/mode while retaining origins. A changed target row with an updated target hash still fails the old-catalog digest proof, as the selected test records.
- The checksum/intent is not authentication or authorization. `verify_reverse` has no pinned old source/beacon File argument and does not establish provenance of the intent's old digest or beacon digest. B2 must compare those facts with actual pinned source/beacon/objects under original exclusions and compiled compatibility. That deliberate ownership boundary is not filled by this pure codec.

## Legacy compatibility and arithmetic

The shared `encode_base`/`decode_base` extraction preserves v1 layout and acceptance semantics inspected against the Stage A source. V1 still uses SCRETN01, the v1 domain, six zero header bytes, all 56 zero row-tail bytes and the original stricter global serial rule. Shared decoding now performs row validity before final whole-catalog canonical comparison; it does not admit nonzero v1 tail data. No successful v1 field/byte reinterpretation was found.

The 4,472-byte golden is independently anchored to the old executable's synthetic phase-2 catalog, not generated by the refactored encoder. I recomputed the actual Wick baseline catalog SHA as `19aae811031be4032e46973677b534741c070f536c80095e7e9be8347f546111`, matching the mapped golden and provenance; old executable identity remains `45ddf9d61f3bdb853f554788645aa3a176aac9294712f7da482755d33e996659`. The selected test verifies direct v1 decode/encode equality and v1→v2→v1 equality. This is one executed real-old metadata golden; broader phase compatibility is source support, not an unrun old-native matrix.

The contract's logical arithmetic recomputes correctly:

- Per-row sidecar overlap: 2*4352 + 2*4096 + 2*1024 + 1024 = **19,968**, below unchanged K=32,768 by **12,800**.
- Fixed global metadata files: 2*120 + 2*768 + 2*176 + 48 + 112 = **2,288**, below unchanged 8,192 by **5,904**.
- Corrected provisional requested-Rust worksheet: 860,400 + one extra maximum raw buffer 69,752 = **930,152**, leaving **118,424** below 1 MiB.

The metadata note correctly distinguishes the third transient raw in-memory buffer from the two-image on-disk limit. Decoder canonical re-encoding can allocate a temporary raw buffer; the inverse output is another transient at a later step. The worksheet is an allocation/capacity envelope proposal, not an executed capacity model, peak RSS measurement or native allocator/kernel/FD/TLS proof. Source-specific container/constructor/object-layout guards and lifetime scheduling remain required before B2 activation; no increase in allowance follows from this arithmetic. No disk transaction is implemented here, so the two-image disk limit is a contract requiring later enforcement, not a verified I/O result.

## Retained execution checked independently

Raw prefix: `/var/tmp/spiralcore-retained-catalog-20261104/validation/`.

| Packet | Actual retained result |
|---|---|
| `retained-v2-api-red` | Build/batch 101, exactly one E0432 unresolved `super::transfer` in the test API probe. |
| `retained-v2-b1-green` | Release build 0; ten selected pure tests pass; scoped Clippy/batch 101 on one test-only `clippy::clone_on_copy` for `[u8;768]`. Protected executable SHA `e7d5a5b2dd49ee2fd59efa027d88953b65c9b4b8f8720ae2616f9353e0e531eb`. |
| `retained-v2-b1-lint-green` | Release build 0; ten selected tests pass, zero ignored, 162 filtered, test 0; scoped release Clippy and batch 0. Final actual protected executable SHA `e3bc946ce64a15131062ca9ca547933ac16a72730c391c80ae741712b3c507d0`. |

I compared retained first-green/lint-green test overlays: the only test-source difference is `let mut bad = raw.clone()` → `let mut bad = raw` for the Copy intent array. It changes no codec or Vec clone. The first lint failure remains real evidence, separate from the final pass.

Final source before/after maps contain 138 identical entries and equal frozen `compiled-inputs.json` SHA `0a0b976bf95fc6afe4422f338a29b1bc39d79cd63342e1d1eb92db79c8f1da3b`. Binary after-test and after-Clippy hashes match the actual final executable. Historical four-file fixture maps match SHA `3ac4ac70557be27b85dcafb4e07a73154a557a54c091972456a9278d1d17d81c`.

Final Clippy command is scoped `cargo +1.97.1 clippy --release --offline --locked --features linux-host,canonical-host --lib --tests --message-format=json -- -D clippy::all`. Its JSON has **52 ordinary unused-import/dead-code messages**, no errors. Test-build JSON has **2 ordinary unread-error-field warnings**. This is not zero warnings, `-D warnings`, all-target or broad-suite acceptance. No new broad library run was claimed by this B1 packet.

Selected final record hashes:

| `retained-v2-b1-lint-green/` file | SHA-256 |
|---|---|
| `test.stdout` | `79500df4b5603770330a077272b2e90f42864e0346f06a664739a867e41c6754` |
| `build.jsonl` | `9f7c6e8ea2a0d319600311f85dc87d0988b972fc739ca9f6afc5704b3c8cabad` |
| `strict.jsonl` | `ba6c65ce27c4e70b68745b3ad34acfd6a3209173dd83502cf5dc270b7ada2a31` |
| `strict-warnings.json` | `2e51211cfea114a248271db53a3f9036ba2b82684d36b0d13d7efe1dfe18d2c2` |

## Scope remaining open

Required next B1 work is the focused pre-clone ordering RED/minimal fix/GREEN and source review of that successor. Other malformed format values are supported by source checks but ten selected examples do not establish a complete mutation/fuzz matrix.

B2 is not implemented: no durable transfer-intent/canonical-name publication, crash recovery, owner Record/Storage-key transition, actual same-ledger activation, native holders handoff, stopped-controller lifecycle or existing finish semantics change occurred. The profile include changes the source-derived current profile but does not accept it as a customer/native migration target. Genuine native phase-4 producer/EndpointRetirement evidence, useful new transfer after restart with old full debt charged, two-guest recovery, long sessions, platform/WAN/capacity and complete upstream delivery remain open. No compliance qualification is implied by this source-only format review.
