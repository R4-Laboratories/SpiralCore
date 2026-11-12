# Mailbox reader and filesystem authority on Windows

This qualification exercises the MVM portable library with Rust 1.97.1 and native MSVC targets. The same protocol verifies signed Receive capabilities, guest possession, delivery leases and ACK signatures. Loading a boot binding from disk needs a supported owner-only filesystem implementation. On Windows both boot-binding loaders return `io::ErrorKind::Unsupported` before opening or inspecting the path. The Windows filesystem tests also verify that the owner-only operations leave an absent directory and file absent. [MVM source](https://github.com/Casey-allard/mvm/commit/ea58be69e1481a3b84938bdaf41514cb165caaca) · [Windows loader](https://github.com/Casey-allard/mvm/blob/ea58be69e1481a3b84938bdaf41514cb165caaca/crates/mvm-core/src/mailbox/reader/platforms/unsupported.rs)

## Native executions

| Windows runner | Native MSVC architecture | Filesystem tests | Reader tests | Whole job wall time |
|---|---|---:|---:|---:|
| [Server 2022](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43321628616/job/131543471763) | x64 | 4/4 | 5/5 | 8 min 17 s |
| [Server 2025](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43321628616/job/131543471675) | x64 | 4/4 | 5/5 | 10 min 21 s |
| [Windows 11](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43321628616/job/131543471890) | ARM64 | 4/4 | 5/5 | 6 min 42 s |

Library checking and strict Clippy passed on all three runners. Each native reader executable ran the unsupported-host boot test and four signed-reader tests. The durations include toolchain setup, compilation and testing. [Runner image versions, executable SHA-256s and original job log hashes](results.json)

The first run passed all four filesystem controls on every Windows runner and stopped before the reader suite. The workflow had expected two refusal tests and the pinned source supplied three, including verification of the diagnostic path. Correcting the expected count from two to three left the MVM source and tests unchanged. [Original three failures](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43320499211) · [Qualified run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43321628616)

The original x64 Windows GNU cross-check on macOS reported 14 compile errors. The repaired source passed cross-target library and test compilation and strict library Clippy. On macOS arm64, the private-filesystem selector exercised the Unix no-follow and link-count checks, and four canonical signed-reader tests passed. The additional unsupported-host reader test is selected only on non-Unix platforms.

Each Windows job checks the runner OS and architecture, compiler host triple and MVM commit, then runs the library check and strict library Clippy. It executes two native test binaries and rejects a successful Cargo exit with missing assertions. The counts are one secure-open/link-count refusal, three filesystem refusal tests, including diagnostic path context, and five mailbox reader tests. The five include the unsupported-host boot loader. [Native workflow](https://github.com/R4-Laboratories/SpiralCore/blob/dfd8810886f045d645a020a8fed01c5772cf9dc1/.github/workflows/mvm-object-reader.yml)

Component scope ends at the portable library. No guest was launched, no gateway storage was opened under Windows authority, and no network transfer was measured. The separate Linux guest job retained its prior MVM source revision. Windows WAN and durable gateway/storage acceptance remain open on the [delivery roadmap](https://github.com/R4-Laboratories/SpiralCore/issues/42).
