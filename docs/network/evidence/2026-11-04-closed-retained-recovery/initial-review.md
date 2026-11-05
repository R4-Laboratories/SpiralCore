# Independent closed retained debt adoption review

2026-11-04. Independent source and retained-record review by `/root/closed_recovery_review`; reviewer did not author the implementation. Scope: Code, Rust and Security review of the five frozen Stage A inputs and their supporting original-owner, catalog, ledger and Core protection paths. No source edits, builds, test executions, native guests, issue comments, publication or new agents were performed. This file is the requested review artifact. Regulatory/compliance observations are delivered in chat only.

**Verdict: CHANGES REQUIRED. One P1 failure-custody finding blocks acceptance of the current source; one P2 diagnostic/snapshot API gap is informational and is not an additional authority blocker.** The executed old-profile phase-2 and malformed-marker tests establish two bounded component observations. They do not qualify the full phase/failure matrix, useful restart or the complete installed MVM contribution.

## Identities and record verification

The actual worktree HEAD is `d6ac5d1b11492ab710012abc1f68fd2ccb3a8e03`; it contains extensive concurrent/untracked changes. Review identity is the frozen five-file SHA map plus the compiled closure, not HEAD alone. The stated published comparison base is `8476d7583e083e02d8b6bc571fc505815b85de90`. I verified every current byte in the five-file map and the 135-entry compiled map against the frozen values, with no mismatch; the five archive members match the map too. No fetch or Git mutation was performed during this read-only review.

| Frozen input | SHA-256 |
|---|---|
| `installed_owner/retained_recovery.rs` | `80008c381cd1c4188609fd05d57c3549edf5063ff53e8138153c41bb05c81f14` |
| `installed_owner/retained_telemetry.rs` | `1d373ea54261dbe35783c0c10837dccc471a454609887176230956aec613278a` |
| `installed_resources.rs` | `712905e7dc0fdd5726a8d41d7bc35c096e0731bccd2412b46099e367e822af7b` |
| `installed_supervisor.rs` | `82053f23173b6384fdf63b1ed3de4229fbc6a9679640529327bf8e455a1cd176` |
| `installed_supervisor/retained_tests.rs` | `1d39088f13f719d7bb96f9f55c5713faafcc1c7c1c7e6a588f2a8274a9547018` |

All source paths above are under `crates/spiralcore-mvm-live/src/`.

Other exact identities:

- Frozen `/tmp/spiralcore-retained-catalog-20261104/recovery-stage-a/freeze/source.tar.gz`: `f9b184c92aa88f5c2efbc334c58b36d8668ff64513b3f4b6e25dbfb734d8f265`.
- Frozen `source.sha256.json`: `dbb9d162a2a2437a43124da0bce7650c035984ea73528670f98810ce6830520b`.
- Frozen `compiled-inputs.json`, and final Wick `source-before.json`/`source-after.json`: all byte-identical SHA `8cf0e230f2d814567f629b4ca177de860f6aecc1ed2a2f062711397b88edb9bc`, 135 entries.
- ADR `/tmp/spiralcore-retained-catalog-20261104/recovery-adoption-plan.md`: `de3eb6916662cfa77e4c8aabd781816ad92f1e3dc688c2bb4b99e349bf057d5f`.
- Full goal `/Users/casey.allard/.codex/attachments/2bb12934-0052-415e-a820-7cd9b48dd1dd/goal-objective.md`: `9f611438b6aaa87bcf25b9f948ccf9dda43cadaf82ba047110f86628b45c53a1`.
- Actual old protected executable: `45ddf9d61f3bdb853f554788645aa3a176aac9294712f7da482755d33e996659`.
- Compiled supported OLD profile: `fca2af7e09a0b8f0782b323d09334c9683d909796b918eff495227b6f27c4f0f` (Linux/x86_64/64-bit only).
- Actual final Wick protected executable: `f182f4750491f7bb65078c9461ac7e9e0852b045d7c2120f68d81e576725a998`.
- Actual pinned SDK dependency: `Casey-allard/mvm@9a3a964bd540d5e74c71f9b7df93cc5593295546`; adapter revision `9153dfe20be9518a2277519c312dd0709664610d`.

A Team roster verification returned `5e31d33c344d39b0`. Exact `Agent code-reviewer`, `rust-reviewer`, `security-reviewer`, and `compliance-reviewer` checks all returned active. The File Claims table contains no active claim rows. Project role guidance was read; its build suggestions were overridden by the explicit source-only/no-build assignment.

## P1 — Arm acquired descriptors before fallible validation

**File:** `installed_owner/retained_recovery.rs:138-139, 344-375, 324-333, 404-414` (existing checked-open helpers also matter).

The early-arm fix at lines 389-396 correctly preserves a qualified state directory across a later malformed-marker error. The same invariant is still violated before that assignment and for other acquired objects:

1. Preparation acquires the independent holders EX File at line 138, then calls `self.current()?` at 139 while the holders File is a closure-local value. If that second current-epoch/storage validation fails, closure unwind drops the actual already-acquired holders exclusion. `RecoveryStartFailure` stores only the owner (and no funding at this stage), so its quarantine cannot retain both original exclusions. This is a narrow concurrent namespace/state failure path, not the normal Busy case where exclusion was never obtained.
2. `read_original` opens the actual catalog at 363 but retains it only in a local variable until bounded read, decode, nonempty check and final active binding all pass. A corrupt checksum/encoding, oversize/short read, empty catalog with latched beacon, or post-read active-binding error drops that File. `adopt` installs `self.catalog` only at 375 after the helper returns. The returned blocked controller therefore does not own the descriptor of the actual catalog that caused the refusal.
3. `optional_file` opens a marker/records/key/status File at 328, then performs filesystem/privacy and cap checks before returning it. For example, a private records file of `8 MiB + 1` bytes is successfully opened and then dropped at the cap refusal (330-331). The assignment to `pin.records` at 438 never occurs. The row remains fully charged and its state/protection pins survive, but the exact observed data File does not. Equivalent cases exist for an oversized marker, key or status. `open_at` itself checks privacy after opening and can also drop its File before the caller receives it.
4. `begin_pin` opens a state directory at 404, then performs identity and Core protection checks before returning the `PinnedEntry`. An original matching state descriptor followed by a failing `protection.check_state` is dropped before `self.entries[at]` can be armed. `open_root` also performs checks after the raw open. This is distinct from preserving an original entry during later fresh revalidation, which the source does correctly.

This is a source-proven custody contract failure. I did not execute a new reproducer. No path above was found to reopen admission, erase files or refund a successfully imported row; those protections do not satisfy the separate requirement to preserve the actual acquired exclusion and File observations. ADR line 64 expressly requires all already-acquired exclusion, actual File pins and charged candidate state in the same blocked controller. Reopening a path later cannot replace the original observation.

**Required change:** Introduce bounded staged custody for every acquired descriptor immediately upon acquisition, before fallible validation. Separate an unqualified observed candidate from a validated original authority; keeping a refused observation must not authorize it. Retain the already-acquired holders EX in preparation failure custody. Keep catalog and per-row observed descriptors in the controller on refusal, including failures currently hidden in checked-open helpers. Fresh revalidation candidates must remain separate from, and must never replace, old accepted pins before identity comparison.

**Required focused negatives before acceptance:** corruption/oversize/late-current catalog failure with actual catalog fstat identity retained; oversized records/key/marker/status with FULL R+K and the exact offending File retained; post-open exact-state validation failure; post-holders-acquisition epoch failure. Check owner/holders/protection contention, same-pool usage and no file mutation before and after dropping the returned failed object. Preserve the existing malformed-marker RED and final GREEN as historical evidence.

## P2 — Expose bounded diagnosis and the specified snapshot facts

**File:** `installed_owner/retained_recovery.rs:97-105, 250-277`.

`RecoveryStartFailure.error` and `RecoveryBlocked.error` are private, have no accessor, and are omitted by the constant Debug implementations. Callers outside this leaf cannot distinguish profile refusal, Busy, missing/corrupt catalog, identity mismatch or I/O refusal. The actual build JSON confirms both fields are unread. `RetainedDebtSnapshot` reports only entry/unresolved counts and aggregate managed usage; it omits observed lengths and per-row unresolved reasons requested by ADR line 78. There is also no retry entry on the same blocked controller, although the ADR requires any retry to reuse bounded custody.

These omissions do not create admission authority and are informational for this intermediate checkpoint. They leave the fuller proposed Stage A inspection/recovery API unable to communicate the facts needed for a meaningful operational decision and prevent the proposed same-controller retry from being used. They must remain labeled unfinished if that fuller API is not being claimed. Add a bounded error classification/accessor, observed-byte and unresolved-reason snapshot data, and either implement the scoped same-controller retry semantics or explicitly retain that part as unfinished. Do not add a cleanup/refund shortcut to resolve this gap.

## Properties supported by source inspection

- Compatibility is a private compiled OLD-profile constant with explicit Linux/x86_64 qualification, separate from the new include-bytes-derived profile. Wrong current profile/phase/generation/supervisor state is refused. No caller-supplied quota DTO becomes authority.
- The original owner is consumed, holders EX is independently acquired, and `InstalledAuthority::from_retained_recovery` constructs `open=false`. No temporary open, public native issuer, reconstructed old VM grant, old retained scope, positive endpoint receipt, general allocation method or deletion method escapes the recovery object.
- `ClosedRetainedFunding` charges fixed metadata `1 MiB` and fixed disk headers `8 KiB` before catalog/table allocation. Row import uses source-fixed FULL `8 MiB + 32 KiB`, one ledger and checked host/tenant/slot arithmetic. It commits a bounded cloned ledger only after all candidate calculations pass; no partial row import or reservation Drop refund was found.
- The decoder is bounded to 16 rows / 69,752 bytes, checks canonical encoding, nonzero identities, exact R/K, old key/generation, valid phases, checksums, padding and row duplicates. Missing/corrupt/empty catalog with the retained beacon refuses closed. No directory-scan reconstruction or beacon clearing occurs.
- The trusted parent retains a read-only actual File and compares path identity. Core runtime-state protection uses an independent nonblocking OFD, checks parent/lock identity and validates direct-child state identity. Successful phase-1 adoption retains that protection guard even with no marker; blocked paths after guard installation also retain it. A read-only Stage A may create/sync Core's canonical protection lock through the existing acquire operation; it does not modify retained payload or catalog bytes.
- Phase 1/2 do not infer no debt from missing records/key. Phase 3 requires committed IDs and remains unresolved. Phase 4 checks exact committed IDs, bounded records length and digest with post-read length validation. This validates the historical records seal; it does not validate a historical wrapped-key content digest because v1 has none. Key bytes remain opaque.
- Revalidation opens a fresh candidate, checks it, compares original IDs and keeps the old table. A failure sticks `failed=true` and later snapshots refuse. Original complete custody is quarantined on RetainedRecovery Drop. This is source support for those completed steps, with P1's earlier acquisition gaps explicitly excluded.

## Recorded execution reviewed independently

I read the following retained records over the specified Wick SSH connection; I did not rerun them. Base: `/var/tmp/spiralcore-retained-catalog-20261104/validation/`.

| Record | Recorded result and limit |
|---|---|
| `recovery-old-fixture-baseline` | Build 0; old-product helper actual ordinary process exit 0; original protected binary SHA independently recomputed. This is synthetic phase-2 process-death state, not native MVM evidence. |
| `recovery-api-red` | Build 101; actual JSON E0432 missing recovery imports and E0599 missing preparation method. |
| `recovery-stage-a-green` | Build 0, selected phase-2 parent test 0; original four-file fixture before/after maps equal. Intermediate binary `d6ca08d22e9dc2123f80c3090d1c0751766ffc390230977df14abc9415679216`. |
| `recovery-pin-red` | Build 0, selected test 101; retained stderr says `late validation error dropped the original state pin`. Actual intermediate binary `2bf24ba88cc15129b7b91346909f04890d0a78398cf34f559ae7e6739f1a754d`. |
| `recovery-pin-green` | Build 0, malformed-marker selected parent test 0, fresh phase-2 selected parent test 0, batch 0. Final executable SHA above. Old fixture four-file and fresh positive four-file before/after maps each equal; source maps 135/135 equal. |

Final compilation command was retained as Cargo `+1.97.1 test --release --offline --locked --features linux-host,canonical-host --lib --no-run --message-format=json`, jobs=1 and incremental=0. Final `warnings.json` contains TWO ordinary `dead_code` diagnostics for the private `error` fields. Empty stderr is not zero warnings or strict Clippy evidence.

Selected final record hashes:

| Wick `recovery-pin-green/` file | SHA-256 |
|---|---|
| `build.jsonl` | `674b49e45ad864cc1ebea7eac3f31c299940c30dc01f388927e362065ced6fc3` |
| `test.stdout` | `a034f43837edd36a2aed7968b7c85316f7684973bab2a17b67d3c098297417eb` |
| `positive.stdout` | `baab7d7c9c62adf1e4ff719517644456d3fa30887c4c3dde27a633923588ff2d` |
| `warnings.json` | `b650827a70b42752fdfa1bf72ca92a8d55b4c675b6ec7127b7b896d528acf086` |
| old fixture before/after maps | `3ac4ac70557be27b85dcafb4e07a73154a557a54c091972456a9278d1d17d81c` |
| fresh positive fixture before/after maps | `7a04da40fe84dd11bffd4f9e44869c26dc82a267618d06147e0851064ea35aef` |

## Remaining acceptance gates

Besides P1-focused negatives, the full phase 1/2/3/4 table, 16-row/multi-tenant atomic funding, malformed catalog/beacon/temporary metadata, wrong old profile, namespace/parent/object replacement, privacy/filesystem/cap errors, active owner and holder aliases in this/another process, retained Core parent-lock contention, panic/unwind/Drop custody and sticky revalidation failures still need executed coverage. Full regression and scoped strict Clippy were not established by the reviewed record. A broader later run by Root would require its own exact executable/result identity.

Stage B debt-preserving versioned generation/profile transfer, durable transfer-intent recovery, same-ledger new activation, real original backend/holder qualification, useful service restart, actual canonical producer/guest transfer, original shutdown/retirement, two-guest recovery, long-lived capacity/fairness, supported platform matrix and final direct/relay WAN acceptance remain separate unfinished work. The current source and two selected tests do not close those gates or the Ari upstream delivery goal.
