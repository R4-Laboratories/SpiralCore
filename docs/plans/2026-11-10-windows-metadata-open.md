# Remove a discarded Windows metadata preflight

This is one measured follow-up under SpiralCore #44 and #49. Product baseline is `bf7107e62e7fa35547f86795b56d7f8d7b3793fc`, whose Rust source matches `661c74e7`. Native timing run 42927983955, job 130219221283, is the retained performance RED: 3,065 exact blocks per direction before the unchanged 240-second overall gate, with 103.046875 process CPU seconds at the 242.002462-second failure snapshot. Receiver Commit means were 38.966/41.512 ms, while Session/TX waits stayed below 1 ms. A prior same-feature pass remains separate evidence; these executions do not isolate host/storage variability or observer overhead.

## Change and invariants

Only Windows `endpoint::read_meta` skips `private_file(path)`, a checked open whose handle is immediately discarded. The following `storage::open_read` retains native path, ancestor, current-SID, owner, DACL, reparse, hardlink and sharing checks on the actual consumed handle. Preserve missing-file mapping and metadata header/length validation. Unix behavior is unchanged. No storage-provider, quota, authorization, format, hash, stage-sync, rename/durability, cache/resource-budget or FlowMux-deadline changes belong to this slice.

## Tests and bounded native comparison

Focused equivalence guards were authored and executed before the production edit. The two portable tests passed against the old production path for header/length, full body/digest, missing-file and hardlink behavior, using an exact test-only copy of that path as control. Directory refusal was added during source review. The retained native performance failure supplies the RED for this optimization; passing equivalence guards establish unchanged semantics rather than a fabricated semantic regression.

One Windows test compares old and candidate paths in the same executable and process: 64 distinct 16 KiB payloads, six paired trials alternating old/new order. The single owned private store contains 1,051,712 serialized object bytes, below 2 MiB, plus bounded directory/junction metadata. Record each pair's wall, aggregate process CPU and calling-thread CPU separately. The CPU domains overlap and must not be summed. Retain sub-resolution zero CPU samples without manufacturing ratios.

Record creation/sync setup and common metadata/body priming as separate measured setup phases. Compare every original byte and digest before the pairs, after each pair, and after negative fixtures, outside the timed metadata windows. This is a commonly primed metadata comparison; it makes no cold OS-cache or cold-transfer claim. Record every pair without asserting a speed ratio or hiding a loss.

Exercise bad DACL, reparse ancestor, hardlink, conflicting writer sharing, directory and missing-file refusal on both paths. ACL and junction fixtures use fixed local PowerShell scripts confined to the owned synthetic fixture, with no received code or product validation bypass. DACL restoration selects the Access section only. Restore permissions, remove owned aliases/junctions, then verify every original byte before cleanup. No new dependency or unsafe Rust is needed; `cpu-time` is already a direct dependency.

Native command after source/security review:

```text
cargo +1.97.1-x86_64-pc-windows-msvc test --release --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --lib endpoint::metadata_tests::windows::single_checked_open_matches_guards_and_records_paired_cost -- --exact --show-output --test-threads=1
```

Portable tests and strict Clippy run on Wick using a fresh immutable 198-file closure from `bf7107e6`, recorded source hashes and separately identified candidate overlays, the existing Cargo target, two build jobs and nice level 10. No Mac build, copied target or large payload archive is needed. Native qualification requires completed refusal guards as well as timing rows; source review, portable success or partial measurements are not a qualified Windows performance result. After narrow independent security review, execute the original 4,096-block/240-second qualification with two-second operation deadlines and retain both passes and failures. Canonical MVM implementation continues independently.


## Retained native fixture failure and child-environment correction

Source `a26c4882c76b0c353b5f5814122932ad0b5ca35b`, run [42932545356](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42932545356), compiled successfully on all three Windows roles. In Windows 2025 job [130234708225](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42932545356/job/130234708225), both portable guards passed and the native test emitted setup plus all six paired rows with exact readbacks. It then failed in the first `Get-Acl` fixture command: `Microsoft.PowerShell.Security` was found but could not load. The ACL/reparse guards and final guard-completion marker did not complete. Keep these measurements as guard-failed partial evidence, without a qualified speedup claim. The same fixture stage also failed in jobs 130234708105 and 130234708108.

The workflow uses PowerShell 7, which launches Cargo and the Rust test executable; that executable explicitly launches Windows PowerShell. Microsoft documents that the intermediate process passes on PowerShell 7's unfiltered `PSModulePath`, which can resolve shared module names to incompatible versions. Its prescribed correction is to remove that variable from the child environment so Windows PowerShell constructs its defaults. [Microsoft module-path startup contract](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_psmodulepath?view=powershell-7.3)

The correction is test-only: `Command::env_remove("PSModulePath")` on the fixed Windows PowerShell child. It does not change global or parent environment, product code, fixture commands, permission assertions or the mandatory ACL/reparse checks. The observed native failure is the RED; native GREEN requires a reviewed rerun with all guards intact. Repeat the same 85 portable tests and strict all-target Clippy with source hashes before publication. No guard skip, deadline relaxation or module installation is part of this correction.


## Retained restoration-control failure

The module-path correction allowed the PowerShell guards to run at source `7e0a5b6acacbcc476fe8c0cacf3a5391e85e69f1`, run [42934581065](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42934581065). All three Windows jobs reached the same subsequent raw restoration-string assertion: Windows 2025 [130241462075](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42934581065/job/130241462075), Windows 2022 [130241462040](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42934581065/job/130241462040), and Windows ARM [130241462027](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42934581065/job/130241462027). Restored strings had `D:PAI` where the original had `D:P`; their owner, group and sole full-control ACE were unchanged. The partial paired rows remain guard-failed evidence.

Microsoft maps `AI` to the OS-managed DACL auto-inheritance bit `0x0400` and `P` to protected DACL `0x1000`. The latter prevents inherited ACEs from modifying this fixture's DACL. The test now compares entire binary descriptors after clearing only the named `DiscretionaryAclAutoInherited` bit in an in-memory `RawSecurityDescriptor`. It requires a present, protected, self-relative DACL and explicitly compares all other control bits. Binary input avoids losing flags omitted by SDDL. Owner, group, ACL revisions, ACE masks/flags/order/SIDs and all remaining descriptor contents must match; there is no broad string replacement or change to the disk descriptor. [Windows control bits](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-control), [SDDL flag representation and omissions](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-string-format), [RawSecurityDescriptor.SetFlags](https://learn.microsoft.com/en-us/dotnet/api/system.security.accesscontrol.rawsecuritydescriptor.setflags)

The existing broad-ACL no-repair comparison remains exact. Both old and candidate fully checked readers and whole-byte/digest readbacks must accept the restored file before the junction guard runs, then all original objects are verified again before cleanup. Product validation is unchanged and is never normalized. Native GREEN still requires the complete guard sequence and final marker after independent source review; no partial speedup is promoted to qualification.
