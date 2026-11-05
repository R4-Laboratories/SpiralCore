# B2 focused successor — author response, independent review pending

Original independent HOLD: b2-independent-review.md SHA cbf0b8cb840e95aaca6f4ac90c72290feef53a1ce83f38ab7dc9a7da368d6ed2. Original six-file freeze 5a2c6250... and all earlier results remain unchanged.

## Causal execution before the fixes

The original product plus cfg(test)-only scaffolding compiled. All seven new filesystem regressions failed101, while both existing valid commit/crash controls passed. The failures establish:

* A canonical intent introduced after prepare was overwritten (both unknown bytes and identical expected bytes).
* A deterministic cfg(test) collision inserted immediately before the existing immutable-intent rename was overwritten. The hook-reached assertion passed, so a mere absence check cannot satisfy this case.
* Resuming with CATALOG_INTENT, disposable inventory metadata or a disposable namespace changed source state/catalog before refusal or returned successful closed commit.
* An explicitly injected malformed replacement STATE, introduced after the durable state-intent barrier, was not held in the current process's descriptor table after refusal. This is an explicit synthetic fault and observation-only current-process FD witness, not a natural exploit/native process authority claim.

## Minimal production delta

`require_eligible_namespace` is shared by fresh preparation, resumed preparation and the final effect boundary. It rejects the exact existing CATALOG_INTENT/disposable metadata set and invokes the existing bounded disposable-namespace guard. Fresh-only staging absence remains separate; interrupted catalog staging still requires the existing exact target proof. Unknown objects are neither reconciled nor deleted.

`publish_immutable` uses the safe existing rustix `renameat_with(..., RenameFlags::NOREPLACE)` only for the canonical immutable transfer intent. Normal catalog/state replacement helpers retain their prior behavior. Unsupported filesystem/operation errors retain the armed staging File and complete original closed custody. There is no replacing-rename fallback and no hard-link/link-count intermediate.

A separate fixed `current_state_observation` slot owns the late STATE candidate before fallible read/classification. It moves a validated target into target custody, releases only a positively matched source duplicate, and retains the actual malformed candidate on error/unwind. The seven new tests now pass, including preservation of unknown canonical bytes/inodes, original exclusions and actual late-candidate FD after failure Drop.

Root's actual lock resolves rustix 1.1.6, not the separately inspected MVM context's 1.1.4. The exact 1.1.6 fs edge was added. All 419 package name/version/source/checksum identities are unchanged; the only lock dependency difference is spiralcore-mvm-live -> existing rustix. The manifest adds exact=1.1.6 with fs. dependency-change.json records the old/new lock hashes; remote package-check.json independently checks the same condition.

Local exact rustix 1.1.6 source confirms the Linux NOREPLACE and Apple RENAME_EXCL mappings. Primary API semantics: https://man7.org/linux/man-pages/man2/rename.2.html and https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/man/man2/rename.2 . Actual no-replace execution here is Linux only; macOS execution/whole MVM qualification is not transferred from source compatibility.

## Funding description correction

The first frozen README's claim that BOTH fixed metadata and row funding precede heap catalog decoding was inaccurate. The implemented order is: bounded fixed read-only constructor observations; fixed metadata funding; heap catalog decode/reverse proof; FULL row R+K funding; excluded-namespace eligibility check; row data acquisition; effect-boundary revalidation; mutation. The old sentence remains in the immutable historical freeze. This successor makes no stronger predecode row-funding claim and introduces no new quota or refund path.

The added File slot is included by the existing fixed Transaction/observer ABI size guard; no cap increased. The provisional full memory/native/RSS gates remain open.

## Exact final execution

* Selected B2: 13 PASS (the original6 plus seven causal regression cases). These are parent tests with explicit original/synthetic child helpers; not 13 MVM/native tests.
* Fixed ABI-envelope test: 1 PASS, overlapping the full library suite.
* Full protected Root library: 172 PASS / 22 ignored, 12.53s. B1's 17 codec tests and ABI test overlap this count.
* Explicit ignored StageA acquisition 5, malformed-marker 1 and original old phase2 case 1:all PASS, separate from default full suite.
* Scoped release Clippy --lib --tests -Dclippy::all:exit0. Build JSON has 2 ordinary dead_code diagnostics. Strict JSON has 75 ordinary diagnostics (74 dead_code + 1 unused_imports), not zero warnings.
* Initial successor Clippy101 solely items_after_test_module is preserved. The test-only intent-byte getter was moved before size_tests with no behavior change, then all 13/ABI/strict were rerun.
* Final binary SHA256 4b733f949ee531c91e5f43141d15bb5848845fdb57f9343d1099c76d0f7c9a4a; 139 source inputs and binary before/after match. Six Rust bodies plus manifest/lock are frozen (8 files); no other input drift.

Source archive SHA256 2a20e24daa7cb8ffca9940b35d0768501223bc1957b9d511cfaaff6d99ab5fec. Focused successor diff versus first freeze (manifest/lock versus their pre-edge bytes) SHA256 09fd2aa6ff2b892be0a275a502ade9cc0ebc3da3d84acd2a13834b745c34b916.

receipts.tar.gz SHA256 3f3544247d8b884b9c3788cee1aad76834a0395e188fcddf4d4b35e331bf2395, 384,013 bytes, 117 hashed receipts plus index = 118 members, read back and hash-verified locally. Raw/protected objects remain on Wick under validation/retained-v2-b2-review-{red,green,lint-green,final-protected}.

No Git mutation, activation, new issuer, deletion authority, VM or WAN execution occurred. Live V2 inventory/one-use activation, holder conversion, interrupted retained-stop recovery, larger fault/IO retry matrix, genuine native phase4/finality, useful customer restart and the full delivery goal remain open. Lane 2 is returned; no Cargo remains active. Independent successor review is required before accepting these fixes or proceeding to activation/history work.
