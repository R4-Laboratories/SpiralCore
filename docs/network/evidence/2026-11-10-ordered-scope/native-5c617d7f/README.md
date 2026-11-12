# Native metadata guards and subsequent Windows Clippy refusal

Source `5c617d7fe4d6e5e70c3126194b616b90f2077542`, [run 42939661617](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42939661617), completed all three selected metadata tests and the complete ACL, reparse, conflicting-writer, hardlink and exact-readback guards on all three Windows roles. Each role ran six alternating old/new pairs over 64 records. The artifact's actual PR checkout `a9a7b2b34a27f00aaecbb2fb03dcb952ac695549` has the same complete source tree as the intended branch revision. `receipts.json` verifies GitHub archive sizes/digests, seven inner checksums per archive and that tree identity.

| Native role | Old median header-read wall | Candidate median header-read wall | Median paired wall change |
|---|---:|---:|---:|
| Windows 2022 x64 | 29.243 ms | 15.568 ms | -46.55% |
| Windows 2025 x64 | 17.199 ms | 9.221 ms | -46.50% |
| Windows 11 ARM64 | 27.517 ms | 14.759 ms | -46.62% |

These are bounded metadata-header measurements. Store preparation and full-body verification are outside the timed header reads and remain in the original records. The same prepared filesystem state is reused; these are not cold-cache results. Process and thread CPU domains overlap and their native counters can quantize short intervals to zero. Raw paired CPU and wall values remain in each ZIP; zero is not evidence of zero compute. No network, end-to-end speed, fleet, energy or WAN improvement is inferred.

The jobs then failed strict all-target Clippy because three existing bindings become unnecessarily mutable when Windows excludes Unix-only `DirBuilderExt::mode` calls. Consequently these failed jobs never executed the sustained FlowMux test. The three-site successor uses immutable bindings with Unix-only mutable shadows, retaining the original `0700` calls and behavior without lint allowances. Independent code/Rust review accepted it. Its hash-verified 199-file Wick baseline with three source overlays passed 13 capture-control and five receiver-stage release tests plus strict release all-target Clippy. The source pins and unabridged small logs are retained here. Native Windows lint and sustained-session acceptance remain separate gates for the successor.
