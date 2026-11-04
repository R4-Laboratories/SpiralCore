# Resident original-helper consumer: independent source review

Verdict: **scoped Code/Rust/Security SOURCE PASS**, with the ordering wording note below. This does not accept a numeric helper profile, activate production/native roles, or establish an executed helper/guest lifecycle. No blocker found in the reviewed custody delta.

Reviewer: session_debug. I did not implement this consumer or its new tests. I previously authored parts of the resident API contract and helper privacy diagnostics; those are not independently approved by this record. The current A Team roster verified `5e31d33c344d39b0`, 14/14; code-reviewer, rust-reviewer and security-reviewer each checked active. No Cargo, process tests, guest boots, or product edits were performed for this review.

## Exact scope

Base immutable SpiralCore revision: `56db5291`. Compared its Git blobs directly with the hashed filesystem bodies; reviewed the consumer delta and relevant existing lifecycle/permit helpers. The selected manifest currently pins canonical MVM `bef4146c0de18b22ac392118acfd898a369cc64c`; its nonconsuming client install, runner install-once/no-active-instance guard and original-owner shutdown were inspected from those immutable Git blobs. This is not approval of unrelated shared MVM WIP.

| Reviewed body | SHA-256 |
|---|---|
| `crates/spiralcore-mvm-live/src/installed_parent/resident.rs` | `1984e9f9e25a34879c8a48af7aa1819a7731d3ee8c744ec699538c4fc4faeaf6` |
| `crates/spiralcore-mvm-live/src/installed_parent/host_agent.rs` | `d90529eb4843d4327bcdf6ea3d76b2fa6ff75e23b0ede3c7bbeae7afc611a110` |
| `crates/spiralcore-mvm-live/src/installed_parent/resident_tests.rs` | `93f403b625a1cb2d95bb1044536fa99cc7a6426c1c7ad1cb6b87688a28889483` |
| `docs/api/installed-resident-service-v1.md` | `6565a8791990cc363236c60a88580a7ff824a8b467c4c3a92c727b43a1eebba9` |

All four hashes were rechecked unchanged immediately before writing this review.

## Custody and ordering findings

- `Live` retains the SAME backend/provider and actual `RootHostAgent`. The successful preparation result is stored before fallible registrar acquisition/install. Registrar installation borrows that backend; it does not replace or consume the original backend. Installation refusal closes admission and preserves the stored helper for explicit shutdown.
- A private consuming `StartFailure` downcast transfers its original owner into `Live`; it does not reconstruct a cleanup handle from metadata. The late activation-lock/phase failure now also returns this typed owner. An unrecognized failure is treated as no-effects only when the provider mutex positively observes its private Unused phase; poison and every other phase retain the error plus whole resident and refuse completion.
- Both start and actual helper shutdown execute inside catch-unwind boundaries. An unwind invokes the existing whole-table quarantine before resuming the original panic. Unknown-startup errors set quarantine/close while leaving custody in `Live`; subsequent Drop detects the retained error/helper even with zero VM slots and forgets the entire owner table and supervisor together. No new receipt, PID authority or refund-on-error path was introduced.
- Global finish first closes admission and requires every VM slot empty. It then calls `RootHostAgent::shutdown` without holding parent registry/helper-state/managed-resource locks. Root shutdown uses cleanup-safe retained identity, while registration continues to require fresh admission. Canonical shutdown checks its registration ledger is empty, drops that lock before stopping the original child, and succeeds only on its existing positive cleanup path. A shutdown error keeps `Live` intact; a panic quarantines it.
- Only successful helper shutdown permits dropping the entire table, including the backend's retained registrar. The original supervisor then performs the same global zero-accounting and holder-exclusion checks. External aliases still retain the original permit/reservation; a Busy result leaves the same supervisor available for a later retry after final alias Drop. No synthetic release or fresh owner is used.
- The existing `size_of::<Live>()` reservation is computed before its Box allocation, so its new inline option storage participates in that limited metadata accounting. This does not cover captured heaps, native allocations, the full helper model or RSS; the qualified profile still has no production constructor.

## Test fitness and remaining execution

The added ordinary tests target missing-profile no-effects/unchanged ledger, refusal with a live VM slot and after close, and poisoned-helper-state whole-resident retention. They do not simulate a positive native receipt. The ignored canonical helper case uses the real canonical signing-key bootstrap and original helper/registrar/backend, checks install-once, closes/drains, verifies backend Drop, and tests external-alias credit retention before the same supervisor finally finishes. Its candidate numbers are explicitly a functional fixture, not a qualified shipping model. It must be separately selected with protected same-revision binaries and a fresh private synthetic home; an ignored case is not executed evidence.

The parent reported an actual preimplementation E0599 against protected normal library metadata. This review did not independently execute or relabel that receipt. Execution of this exact consumer source and the selected canonical helper case remains pending at review time. Existing helper-only results retain their earlier identities; no native VM, positive VM/endpoint receipt, telemetry integration, cross-platform/WAN or whole-resource acceptance follows from this source review. Literal quality-gate tooling was not available/invoked.

## Nonblocking ordering wording

`start_owned_host_agent_inner` checks that all **current** slots are absent. After an earlier operation reaches UnusedRetired/Retired and frees its slot, that condition can become true again while the helper phase remains Unused. The API addendum says refusal follows “any VM operation,” which reads as lifetime history. Clarify this to current outstanding operations, or add a monotonic first-VM-attempt flag plus a focused test if strict helper-before-any-attempt ordering is intended. The SDK independently refuses currently retained installed workloads, so this mismatch is not evidence of an unsafe custody transfer. The source PASS covers the actual current-slot semantics, not an unimplemented historical barrier.

## Monotonic first-VM barrier successor

**Scoped successor SOURCE PASS.** This addendum resolves the ordering wording finding above; the original review and source identity remain historical. Resident source is now `8fac9672a1d923c24b6a8b069180e7704d75ae0feaec2d8557467746a2ef0250`; tests are `add4cbaf300ff862538f9dc4759841d431d4765e84be0c3b30f3622c47af0eba`; helper source remains `d90529eb4843d4327bcdf6ea3d76b2fa6ff75e23b0ede3c7bbeae7afc611a110`.

Verified the source delta is exactly four monotonic-flag hunks and one test assertion: reversing those hunks reproduces the original reviewed resident/test SHA-256 values. `Live.has_issued_vm` starts false, becomes true immediately after original `prepare_vm` successfully returns, before the fallible operation-storage/future construction, and is required false by helper startup. There is no other assignment/reset. Thus retirement, unused cancellation, and later operation-construction failures cannot reopen helper installation. An unsuccessful preparation creates no history bit because it returned no original preparation. This is a local ordering restriction, not permission, a receipt or an extra resource allowance. Existing size-based Live metadata admission includes its changed inline layout. No ownership, shutdown, wire, deadline or quota-release logic changed.

The strengthened existing test first obtains the real canonical UnusedRetired result, then requires the helper-ordering detector before any helper profile lookup. This distinguishes the intended historical barrier from the old generic missing-profile error, without creating a helper or native proof.

Independently read the retained remote receipt files under `/var/tmp/spiralcore-resident-helper-20261104/validation/` (read-only; no new execution):

- `lifetime-order-red`: build exit 0; test exit 101; 0 passed / 1 failed / 0 ignored, with exact failure `completed original preparation was forgotten: owned helper resource model not qualified`. Its source manifest pairs original resident `1984e9f9...` with new tests `add4cbaf...`; before/after source JSONs agree.
- `lifetime-order-green`: build exit 0; protected serial `installed_parent::` execution exit 0, 22 passed / 0 failed / 2 ignored / 124 filtered. The reviewed successor resident/test/helper hashes appear in the source manifest. Source-before/source-after and binaries-before/binaries-after JSONs compare equal.
- The same successor protected test object was then explicitly invoked with the exact canonical-helper alias-retention test name and `--ignored --exact --test-threads=1`: exit 0, 1 passed / 0 failed / 0 ignored / 147 filtered. Its command names the retained canonical agent/signer objects in `validation/actual-helper/protected/` and a fresh synthetic MVM_HOME. The earlier `actual-helper` record is separately retained, also exit 0 with one selected helper case and matching source/binary JSONs; it is not substituted for the successor execution.

These receipt observations supersede the prior execution-pending statement only for the stated Linux functional component/helper scopes. I did not rerun tests, compile or independently regenerate those recorded build hashes. The canonical helper case boots no VM and its candidate profile remains a test-only fixture. Strict Clippy was still running when this source recheck was requested; no strict result is inferred here. Production profile acceptance, the full managed model, native VM/endpoint receipt consumption, cross-platform composition, fleet capacity and WAN gates remain open.
