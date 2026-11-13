# macOS arm64 verification of the joined MVM source

The complete locked, offline Cargo workspace test command finished successfully in **1,072.67 s**, followed by strict workspace Clippy in **104.48 s**. The two commands ran against commit `9035492d7979c90dee0538649735ebed2358145d`. The clean checkout, Git tree and four recorded changed-file hashes matched the public commit. The workspace output reached its final `xtask` doctest target. Elapsed times include build work. [Joined source](https://github.com/Casey-allard/mvm/commit/9035492d7979c90dee0538649735ebed2358145d) · [Exact commands, selected target summaries and retained-log SHA-256 values](joined-macos-workspace.json)

| macOS arm64 check | Revision | Exit | Elapsed |
| --- | --- | ---: | ---: |
| Original workspace test | `3c4e763a` | 101 | 1,872.55 s |
| Joined workspace test | `9035492d` | 0 | 1,072.67 s |
| Joined workspace Clippy, warnings denied | `9035492d` | 0 | 104.48 s |

The original workspace run stopped in `cve_lab_gcp_controller`. Its simulated instance-create failure expected zero deletion attempts and recorded one. A create error can leave the remote outcome uncertain, so the shared controller attempts cleanup. The correction changed one expected count in the wrapper test. Both mocked controller scripts then passed individually, and the same Rust integration target passed both tests in 1.48 s during the repaired workspace run. These checks used a fake cloud CLI. [One-line test correction](https://github.com/Casey-allard/mvm/commit/cf7d67f6a5f4d6f2c398589636346e6fe6570502) · [Controller integration target](https://github.com/Casey-allard/mvm/blob/9035492d7979c90dee0538649735ebed2358145d/tests/cve_lab_gcp_controller.rs)

The machine-readable record gives each selected test target’s own counts, including ignored and filtered tests. For example, `mvm_hostd` reported 2,393 passed and 15 ignored. Each row covers its named target. The eight original log and run-record hashes permit independent comparison against the retained private files. Public data includes the failure diagnostic, test summaries and hashes.

The earlier **Linux x86-64 Firecracker** trial used the joined commit’s recorder parent, `cef3b6fe`. That revision passed Linux feature-specific Clippy, the workspace all-targets check and the explicit BDD target compile. Two original guests verified 257 bytes after both mailbox helpers were replaced. The macOS workspace checks add source-build and test evidence at the joined revision. Native Apple guest delivery, WAN and cross-platform capacity remain separately tracked. [Linux native row, source IDs and verification](native-helper-restart/README.md) · [Apple helper-process test](darwin-helper-reconnect.md) · [Platform tracker](https://github.com/R4-Laboratories/SpiralCore/issues/46)
