# Independent P1 acquisition-custody successor review

2026-11-04. Independent Code/Rust/Security delta review by `/root/closed_recovery_review`. The reviewer authored neither the initial implementation nor its successor. Scope is the changed acquisition/arming boundary and its retained records. No product edits, builds, test executions, guests, publication, issue comments or new agents were performed. Regulatory/compliance observations remain chat-only.

**Verdict: PASS for the narrowly defined P1 acquisition-custody correction.** No remaining blocker was found within that reviewed delta. This accepts the repair of the prior descriptor/holders failure paths; it does not accept the full recovery phase/fault matrix, P2 operations API, Stage B, useful restart, native/customer service, WAN/platform/capacity or upstream delivery.

The original HOLD report remains unchanged at `/tmp/spiralcore-retained-catalog-20261104/recovery-independent-review.md`, SHA `2115b84b73b7375ae4652b973f7e1ad6002fc8e885932a73ac93173f95946f09`. Its failure evidence is preserved. This separate successor disposition supersedes its P1 HOLD only for the source identified below.

## Exact reviewed identity

Freeze directory: `/tmp/spiralcore-retained-catalog-20261104/recovery-stage-a/p1-freeze/`.

| Item | SHA-256 |
|---|---|
| `source.tar.gz` | `dc42704cdd6d2a5a799a87cc05ea226bcd648a42182fedb1f89ed7083c7ada7f` |
| `source.sha256.json` | `104bcf5ce0750295c65c7d87ee8f6771ca78cba5f2fefe650ba1c62e8a417863` |
| `compiled-inputs.json` | `8e32fa8f16eead8766c472d4769747f00d718322379bd0437d45db49685e5600` |
| `author-response.md` | `afef221ce0a495e4a6880795073bc21190d2b0b0e95021e30427adda3892c95f` |
| Recovery leaf `crates/spiralcore-mvm-live/src/installed_owner/retained_recovery.rs` | `c924391854633195876811116f634742b1e9a1916060200a4ff4e03a38fa968a` |

I independently verified all five archive members against the frozen map, all five current source files against that map, and all 135 current compiled-input entries against the frozen compiled closure: no mismatches. Only the recovery leaf differs in the five-file map from the original reviewed checkpoint; the other four hashes remain:

- `installed_owner/retained_telemetry.rs`: `1d373ea54261dbe35783c0c10837dccc471a454609887176230956aec613278a`.
- `installed_resources.rs`: `712905e7dc0fdd5726a8d41d7bc35c096e0731bccd2412b46099e367e822af7b`.
- `installed_supervisor.rs`: `82053f23173b6384fdf63b1ed3de4229fbc6a9679640529327bf8e455a1cd176`.
- `installed_supervisor/retained_tests.rs`: `1d39088f13f719d7bb96f9f55c5713faafcc1c7c1c7e6a588f2a8274a9547018`.

These four paths are also relative to `crates/spiralcore-mvm-live/src/`. The unchanged external dependency/old-profile identities and ADR remain those recorded by the original review. This delta introduces no package/manifest/lock update. Roster `5e31d33c344d39b0` and the exact Code/Rust/Security/Compliance role eligibility checks were revalidated as active.

## Disposition of the original P1 paths

Source references below are in the frozen successor recovery leaf.

| Prior hole | Successor mechanism | Review disposition |
|---|---|---|
| Already-acquired holders EX dropped after late current validation | `Preparation` owns the original owner from function entry (99-137); raw holders observation enters its slot before private/ID/lock checks (151-168). Both returned failure and guard Drop retain its actual owner/holders. | Fixed in source. Late-current returned error and explicit post-holder unwind have causal executed negatives/positives. |
| Catalog File local across read/decode/refusal | `observe_at` stores the successful fresh FD immediately into an empty private slot (371-390); `catalog_observation` is owned before private/filesystem/cap/decode/nonempty/final binding validation (441-479). Adoption moves it to accepted catalog only after success (482-484). | Fixed in source. Actual CRC refusal observation is retained across blocked-object Drop in the selected test. |
| Optional-file File dropped on cap/privacy failure | `capture_optional` stores into `FileObservations` before post-open checks (415-430). Marker/records/key/status all use this same bounded helper. | Fixed in source. Oversized marker supplies the executed representative cap case; other names/caps are source coverage, not separately executed cases. |
| State directory dropped by checked-open/post-open refusal | `observe_state` stores the raw fresh FD before metadata validation (392-413). `capture_state` validates that owned observation (524-545). Qualified state moves separately into accepted entry before later file observations (496-520). | Fixed in source. Actual post-open state privacy refusal is the selected executed case; separate identity/protection fault combinations remain broader-matrix work. |

Raw helpers use nofollow/CLOEXEC/nonblock and adopt only the successful syscall's fresh descriptor. They accept no caller-supplied raw FD and operate only on empty internal observation slots. Record validation and canonical-parent checks still precede the state open. A stored refused observation is private evidence/custody, not authority: it cannot become a successful ClosedRetainedDebt, scope, grant, sealed receipt, native issuer or new admission.

Function-entry `Preparation::Drop` retains already-owned values on unwind, while a returned `RecoveryStartFailure` owns that same guard. After the infallible owner/holders move into closed authority, funding is installed in the guard before `fund_metadata` (185-200). Therefore an ordinary early metadata-funding error returns the guard owning the same funding/closed authority, which retains the owner and holders; a later layout-duplication error does likewise (202-209). This funding-error conclusion is SOURCE inspection, not an extra executed test claim. The five selected cases do not separately inject an early funding/ledger failure. The short move/construction gaps contain only invariant-preserving private moves and fixed construction; ordinary allocator exhaustion aborts rather than providing an unwind recovery path. No public mutation can invalidate the four private `expect` invariants inspected here.

Accepted entries and fresh revalidation observations remain separate. Revalidation observes a fresh catalog and fresh row Files, completes identity/content comparisons against the accepted originals, then clears only duplicates at the successful end (624-668). Any returned failure retains observations plus originals; failed normal revalidation remains sticky through the existing caller. Adoption owns the full recovery object, whose Drop quarantines the whole bounded controller on unwind. No originals are replaced by a fresh candidate before comparison.

The same closed funding, fixed `1 MiB + 8 KiB` metadata allowance, FULL `8 MiB + 32 KiB` per validated row, fixed 16-row bounds and refusal/admission rules remain unchanged. No quota increase, refund, deletion, beacon rewrite, old-scope reconstruction, new original authority or ordinary-admission reopening was found in this correction. A corrupt catalog lacks a trusted decoded row count; retaining fixed bookkeeping and closed exclusion is not evidence that historical debt was zero or refunded.

## Independent record verification

Read-only SSH inspection used `/var/tmp/spiralcore-retained-catalog-20261104/validation/recovery-acquisition-{red,green}/`. I recomputed the actual protected executable hashes, read commands/results/diagnostics, compared source and fixture maps, and inspected the test modules from the retained source overlays. I did not rerun the tests or compiler.

The RED and GREEN overlays contain **byte-identical cfg(test) modules**, SHA `83fdc4d493d3de0310b2220d1e70b4142661c4bce6fcc3634a793ce51062e9fb`. This supports causal attribution to the implementation correction rather than different assertion bodies.

| Record | Actual retained result |
|---|---|
| Acquisition RED | Release build 0; the same five selected parent tests fail, test/batch 101. Stderr identifies CRC observation loss, released holders EX, cap observation loss, state observation loss, and owner EX released after explicit unwind. Actual RED binary SHA `0db79ea5aca4433ebcb438f4ad12d135f00a80bd204be78bc841c0e56ebe2803`. |
| Acquisition GREEN | Release build 0; same five selected parent tests 5 passed / 0 failed, 0.16s, test 0. Explicit injected unwind appears in stderr but is caught by the successful test, not an unhandled run failure. |
| Previous malformed-marker regression | 1 selected parent test passed, marker exit 0, 0.03s. |
| Fresh actual OLD-profile phase 2 | 1 selected parent test passed, positive exit 0, against a newly generated private fixture from the same protected old executable. |
| Same protected executable, ordinary library | 153 passed / 0 failed / 9 ignored, broad exit 0, 12.46s. Seven ignored cases were separately selected above; two others remain unrun by this packet. Helper tests returning early without their environment do not establish native execution. |
| Scoped release Clippy | `--lib --tests -- -D clippy::all`, exit 0, errors JSON count 0. Warnings JSON count **38** ordinary unused-import/dead-code messages. This is not `-D warnings`, zero-warning or all-target qualification. Test build JSON separately has **2** ordinary unread-error-field warnings. |

Final actual protected executable SHA: `d22faf6febf65a30564f8617d1bdf5f80a7f4effd614f75517065b779b1acc12`. All retained binary after-maps match, including after broad execution/Clippy. The 135-entry source before/after maps are byte-identical to frozen `compiled-inputs.json` SHA `8e32fa8f16eead8766c472d4769747f00d718322379bd0437d45db49685e5600`.

Original OLD executable remains SHA `45ddf9d61f3bdb853f554788645aa3a176aac9294712f7da482755d33e996659`; supported OLD profile remains `fca2af7e09a0b8f0782b323d09334c9683d909796b918eff495227b6f27c4f0f`. Each selected acquisition test invokes the actual old helper in an ordinary process and checks its success; these are synthetic filesystem/debt fixtures, not booted MVMs. The late-current error and post-holder unwind are explicitly cfg(test) injections. The descriptor assertions observe this-process Linux `/proc/self/fd` dev/ino counts, and lock assertions use independent actual kernel flock contention. They do not create PID-based recovery authority or adopt observed raw FDs.

Historical four-file fixture before/after map SHA is unchanged: `3ac4ac70557be27b85dcafb4e07a73154a557a54c091972456a9278d1d17d81c`. Fresh positive fixture before/after map SHA is `b00a4421ac14569aa276c7db76871914fbaff3981fc5ae204338f262a15e8aef` and matches on both sides.

Selected GREEN raw-file identities:

| File | SHA-256 |
|---|---|
| `test.stdout` | `efefc647f2c05e397430b1f4f2f42707f336dbafc58a874fce46c9cd135c6e9e` |
| `test.stderr` | `52d563042b751f0411f6d45145261546dad0d356872c2c56e2c90dc731d06d25` |
| `marker.stdout` | `3fc0b5d390323abe71cba9facb5bd20613040db13a1e415ed57c7b6aceb95f10` |
| `positive.stdout` | `3e462d18b7cef7766939c3eda866e68d6d4c656c60ded6b660d5f64d03bfbe41` |
| `broad.stdout` | `b75ff6b49e4d26a10899de3134a42dd9ac40f20ddbe9d7bc7f467c4a2e43649d` |
| `build.jsonl` | `25aa01944164ff0515377fb8df8e290665fa933d78873df34044dc6a594a9a4a` |
| `strict.jsonl` | `559e6b3bf497b96306ef8800ca1e1421c50e7aef115460e4f428e48b0ab25ae8` |
| `strict-warnings.json` | `0837e5cee607157a7dccb64de8ab8c13800b376097bda666251387e94156d7bf` |
| `strict-command.json` | `21b2c1f6e26f716a764781430ef0d771a2d03c85ed675cf45e17a335fded0a2d` |

## Explicit open gates

P2 bounded diagnostics, observed-byte/reason snapshot data and same-controller retry remain informational and unfinished. The complete phase1/2/3/4 and malformed metadata/tenant/table/overflow/replacement/privacy/filesystem/cap/poison/fault matrix has not been established by this packet. In particular the early funding-error retention path is source-supported only. The selected marker-cap and state-privacy witnesses do not imply every optional-file, post-open protection or failure permutation was executed.

Debt-preserving versioned generation/profile migration, transfer-intent recovery, same-ledger activation, native producer/original backend/holder/EndpointRetirement qualification, useful service restart and retention disposition remain open. So do final platform, actual WAN, capacity/fairness and upstream customer-delivery acceptance. The narrow PASS here authorizes no claim that those outcomes have occurred.
