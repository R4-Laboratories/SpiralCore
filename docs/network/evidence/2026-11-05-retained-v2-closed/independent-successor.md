# Independent B2 review-fix successor

2026-11-04. Independent Code/Rust/Security delta review by `/root/closed_recovery_review`, who authored neither this transaction nor the corrections. Scope is the two P1s and late-STATE observation gap in the original independent HOLD. No product edits, builds, new tests, guests, publication or broader audit were performed.

**PASS for these fixes in the exact CLOSED transaction component.** Both P1s and the identified late-state observation gap resolve in this successor. This accepts the corrected closed transaction boundary, not live inventory integration, Open authority, holder conversion, interrupted retained-stop recovery or native/customer restart.

Original HOLD remains unchanged: `/tmp/spiralcore-retained-catalog-20261104/stage-b/b2-independent-review.md`, SHA `cbf0b8cb840e95aaca6f4ac90c72290feef53a1ce83f38ab7dc9a7da368d6ed2`. Original source snapshot and causal failures are preserved. This separate disposition supersedes the two P1s/observation gap only for the source identified below.

## Exact frozen identity

Directory: `/tmp/spiralcore-retained-catalog-20261104/stage-b/b2-review-successor-freeze/`.

| Artifact | SHA-256 |
|---|---|
| eight-file `source.tar.gz` | `2a20e24daa7cb8ffca9940b35d0768501223bc1957b9d511cfaaff6d99ab5fec` |
| `successor.diff` | `09fd2aa6ff2b892be0a275a502ade9cc0ebc3da3d84acd2a13834b745c34b916` |
| `author-response.md` | `7ec11d6d3c5633bb111533e9748ad9713425fba688b1705fe3c5ade61d30d2cf` |
| `source.sha256.json` | `1c0adb166d8fc349fdce9c108e68daa74aeb677189e365e031f7802a5470806f` |
| `compiled-source139.json` | `8748d05998d4c4d3e4a1274671da8904ced298a3683a896e6a84cfdacfd82284` |
| `receipts.tar.gz` | `3f3544247d8b884b9c3788cee1aad76834a0395e188fcddf4d4b35e331bf2395` |
| `dependency-change.json` | `744706e4f60633dcb5c3a0481e7ac1bcd1d0403a8f468f6c55720f0c00bc08fb` |

All eight current mapped sources and eight archive members match the frozen map; all 139 current compiled inputs match the frozen closure. All 117 indexed receipt members match their hashes; the receipt archive has 118 members including index. Roster `5e31d33c344d39b0` was refreshed successfully.

Frozen inputs, relative to `crates/spiralcore-mvm-live/`:

| File | SHA-256 |
|---|---|
| `src/installed_owner.rs` | `0f773411912745414c44ca6229bb237cd7530f59a32f3b266b9c728783b8873a` |
| `src/installed_owner/retained_telemetry.rs` | `f7f064df1835f632da88538ff9db88c251da4cd1ae9699263f1cad3ee1f35011` |
| `src/installed_owner/retained_recovery.rs` | `1772a11d3aa0234c39ab132a0e5208ffa475cbbec8101962d46e027bcc0bc5fc` |
| `src/installed_owner/retained_transition.rs` | `203242fcfb99897adf50830bb3ba6f418b8a638a8140e9b94dd85c3db8f15f2c` |
| `src/installed_resources.rs` | `378ee1f09665c42857b0ee4f8b13643d0090aa9776e70917c50edf651424bd15` |
| `src/installed_supervisor.rs` | `047033f4ebbffaeaa78aae7932083265157cd338dfe76c6963a6ff7a32df98ea` |
| `Cargo.toml` | `dba5182ebf8470e83af04e80b6cd46ab439b4039dcb22a9ca232f940c64c8cb1` |
| `Cargo.lock` | `b0e54aa2d3c3f144e5e71af269622f681c3c99b0c04b5e3f31b32b0d822e5d22` |

## Resolution of P1: immutable canonical intent publication

`retained_transition.rs:200-223` uses rustix `renameat_with(..., RenameFlags::NOREPLACE)` for immutable transfer intent publication. Source-name identity is checked, source staging remains armed, and unsupported filesystem/operation errors propagate into the same closed custody. There is no replacing-rename or hard-link fallback. Ordinary replacing catalog/state publication remains separate.

Fresh commit repeats canonical TRANSFER_INTENT/TRANSFER_NEXT/CATALOG_NEXT absence at 369-370, then publishes through the no-replace primitive at 391. Thus a collision already present after preparation refuses early, while a collision introduced at the final syscall boundary cannot be overwritten. The selected hook demonstrates the latter boundary was reached and prevents an absence-check-only fix from satisfying the case.

Linux execution supports the actual no-replace refusal. I also inspected exact local rustix 1.1.6 source: public fs/at.rs routes renameat_with through backend renameat2; Linux NOREPLACE maps to RENAME_NOREPLACE; Apple maps to RENAME_EXCL and renameatx_np. The Apple backend returns NOSYS with nonempty flags when the API is unavailable, rather than falling back to replacing rename. **Apple evidence is source inspection only; no macOS execution or platform qualification is accepted.**

## Resolution of P1: exact shared namespace exclusions

`require_eligible_namespace` at 187-199 rejects the same CATALOG_INTENT and three endpoint-disks metadata names as the original fresh path and invokes the existing bounded disposable-namespace guard. It grants no cleanup/reconciliation permission.

The helper is now used by fresh preparation, resumed preparation after full row funding/before row data acquisition, and commit at the final effect boundary before intent/catalog/state mutation. Fresh-only staging absence remains separate; interrupted staging still requires exact target proof through the existing observer. The three introduced excluded-namespace fixtures now refuse before changing source state/catalog/owner/intent, preserve the unknown inode/bytes, and retain original exclusions across failure Drop.

## Resolution of late STATE observation gap

The fixed Transaction field `current_state_observation` is armed by the raw observing helper before fixed-size read/classification at 472-482. A validated target moves into target custody. A source observation either becomes the retained source slot or is released after the named source positively matches its retained source pin. Read/classification errors leave the candidate in the transaction; whole-transaction quarantine preserves it across failure Drop/unwind.

The explicit post-state-intent malformed replacement test inspects this-process Linux FD dev/ino observations after refusal and again after Drop. It now retains the actual candidate descriptor. This establishes the selected error path, not every possible namespace race or an allocator/native-process authority claim. The extra fixed File slot is covered by the existing Transaction/observer ABI envelope test; no allowance was increased.

## Dependency identity

I independently parsed the pre-edge Cargo.lock from B1 base `8856373d45ade643194d130e2f115fe817315dbd` and the successor lock, and compared all package `(name,version,source,checksum)` identities. Both contain **419 packages**, with identical identities. Comparing complete package rows found one dependency-row change only: `spiralcore-mvm-live` adds existing `rustix`; no dependency is removed.

Manifest pins `rustix = { version = "=1.1.6", features = ["fs"] }`. Rustix 1.1.6 was already in the lock, registry checksum `e5e76742e5294153d4dde1207dbec5bdd81eef6fabdc0d2118babadabfd858aa`. Old lock SHA is `65bf2744b0f5c90a05f2abb5af9eedf3b83e689588840a637a58819887a955ad`; successor lock SHA is `b0e54aa2d3c3f144e5e71af269622f681c3c99b0c04b5e3f31b32b0d822e5d22`. This review does not substitute the different MVM-context rustix 1.1.4 identity.

## Independently checked causal records

Wick raw prefix: `/var/tmp/spiralcore-retained-catalog-20261104/validation/retained-v2-b2-review-*`. I read raw records and recomputed the actual final executable hash; no tests/builds were rerun.

- RED: build0; **seven selected new cases fail**, test/batch101, while valid commit and valid four-barrier crash controls each pass. Failures explicitly demonstrate unknown and identical canonical intent overwrite, final-boundary collision overwrite, three excluded-resume source mutations, and lost late-candidate descriptor. RED actual executable SHA `36d49d2f47f9e02c7c8146599a0915327a52f2f2f8e07d1bd0cf59167cbb7c83`.
- Final lint-GREEN: build0, **13 selected B2 cases PASS**, ABI1PASS, strict/batch0. This combines original six B2 cases and seven new regression cases; child process helpers are not MVM/native counts.
- Final protected library: **172 PASS / 22 ignored**, 12.53s. The ABI and B1 codec cases overlap this count. Explicit ignored StageA acquisition5, malformed-marker1, old phase2 case1 all pass separately.
- Scoped release Clippy `--lib --tests -- -D clippy::all`: exit0 with **75 ordinary diagnostic messages** (74 dead_code, one unused_imports); build JSON has two ordinary warnings. Initial successor `items_after_test_module`101 is preserved; the test-only getter moved before the size-test module without behavioral change. Not zero warnings, `-D warnings` or all-target acceptance.

RED→GREEN recovery test-source delta changes only the excluded-source equality assertion from `assert_eq!(before, after)` to `assert!(before == after)`, preserving its predicate while reducing failure-byte dumping. Test scaffolding/fault hooks remain cfg(test); this does not weaken the causal refusal assertion.

Final actual protected executable SHA: `4b733f949ee531c91e5f43141d15bb5848845fdb57f9343d1099c76d0f7c9a4a`. Source-before/after maps for final selected and full runs match frozen 139-input SHA `8748d05998d4c4d3e4a1274671da8904ced298a3683a896e6a84cfdacfd82284`. Historical four-file fixture maps remain SHA `3ac4ac70557be27b85dcafb4e07a73154a557a54c091972456a9278d1d17d81c`.

Selected final lint-GREEN raw hashes:

| File | SHA-256 |
|---|---|
| `test.stdout` | `50052070b6a3e803a9ef4e728eac8f746237b803d22d35f6d0d66e3ac139201c` |
| `size.stdout` | `01bde8ecd07adf1f96c703eac673413acdf672da121534233d31165459278752` |
| `build.jsonl` | `cea20cbfd4feda2fbaa9fcd643eef16c61f7a735c83962f108421de7dbc0ceff` |
| `strict.jsonl` | `b2109bc72c60f99fc4295e9f1a2d5f2b6f3127e7181e464d539fd9ce3aeae3de` |

## Corrected claims and unchanged gates

The author response explicitly corrects funding order without altering the historical freeze: fixed read-only constructor observations; fixed metadata funding; heap catalog decode/reverse proof; FULL row R+K funding; namespace eligibility; row data acquisition; effect-boundary revalidation; mutation. No predecode row-funding claim, cap increase or refund is accepted.

The existing actual ledger move, original owner/holders/pin custody and CLOSED admission boundary are not broadened by these fixes. Metadata phase4 fixtures still do not establish actual EndpointRetirement/backend finality. Live V2 inventory/history capacity, private one-use activation, EX→SH holder conversion, interrupted retained-stop opener, full fault/I/O retry/unwind and capacity/native allocator/kernel/RSS qualification, genuine native producer/receipt, useful restart, platform/WAN and complete customer/upstream delivery remain open. The narrow PASS here supplies none of those authorities or measurements.
