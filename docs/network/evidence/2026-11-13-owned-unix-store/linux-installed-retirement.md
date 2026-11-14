# Linux registered endpoint: transfer and original-owner retirement

The registered host-process FlowMux fixture now completes authenticated transfer, exact readback, HalfClose, reader shutdown, store retirement and the original archive retry. The [published MVM checkpoint](https://github.com/Casey-allard/mvm/commit/31f8a562f19d9eac89cb0d078b507770bb592262) contains both repairs. The fixture supplies authenticated guest-side frames over a Unix stream. It does not boot a guest or exercise a physical vsock device or WAN.

## Repairs

The original endpoint stopped opening an authorized stream because x86_64 seccomp refused syscall 264, `renameat`. The records-only rule admitted the runtime-state descriptor, while the owned store opens namespace descriptors after confinement. Only the owned-store role gains the syscall, with both directory arguments constrained to nonnegative native descriptors. Landlock retains its filesystem grants; the store retains its namespace checks. Records-only restrictions, the base x86_64 profile and AArch64 behavior remain intact. The old filter failed the new kernel control. Its successor passed four scoped tests, including seven rename publication/refusal cases.

The next run verified all 5,804 bytes and joined the peer, then refused `Drained`. Normal stop had fenced admission before the last store permit ran cleanup, which used that same publication guard. The [retained failure](linux-installed-retirement-before.txt) records successful readback before the shutdown refusal.

The original lease now advances monotonically through Open, Closing and Failed. Ordinary session teardown closes admission and wakes the original transport. New work requires Open. The already-held store permit may finish destruction after Closing only while the original holder, live parent, audit processes and held directory still verify. A stale holder, audit failure, unexpected control input or uncertain cleanup remains Failed or quarantined. Ordinary close cannot restore it. Parent sync, confirmed absence, zero accounting, the child's `Drained` message and original child exit remain separate requirements.

On macOS, empty EOF and queued final data both report `POLLIN | POLLHUP`. The implementation checks unread bytes on the same original pipe without consuming input. The old implementation failed controls for trailing input and a restored stale holder; the successor refuses a drain receipt in both cases.

## Functional measurement

| Measurement | Observed |
| --- | ---: |
| Exact application bytes, each direction | 5,804 B |
| Actual TLS bytes toward/from cooperating peer | 3,657 / 3,234 B |
| Setup through authenticated Hello | 91.263 ms |
| Authorized Open through verified HalfClose | 34.724 ms |
| Retirement including archive retry | 43.624 ms |
| Complete test wall time, GNU time rounded | 0.18 s |
| Complete test CPU, user + system rounded | 0.08 + 0.05 s |
| GNU time maximum RSS | 25,316 KiB |

TLS counts include the cooperating-peer handshake but exclude guest-side Unix-stream traffic, host controls and TCP/IP overhead. CPU includes fixture work and waited descendants, rather than codec work alone. Maximum RSS is not simultaneous aggregate process-tree memory. There is no matched raw/zstd control in this functional run, so it establishes neither a comparative speedup nor a byte-saving percentage. [Passing output](linux-installed-retirement-pass.txt) · [Results, commands and executable hashes](linux-installed-retirement.json).

## Regression evidence

| Native check | Result |
| --- | --- |
| Linux hostd library | 2,492 passed, zero failed, 15 existing ignores; 53.12 s |
| Linux VMM library | 1,112 passed, zero failed, four existing ignores; 16.75 s |
| Original endpoint-lease controls | Linux: 10 passed; macOS: nine passed |
| Workspace Clippy, all targets, `-D warnings` | Passed on Linux and macOS |
| Formatting and rebuilt footprint gate | Passed |

The first Linux library run retained 14 permission failures under the host's `0002` umask. Two same-executable controls passed with a private temporary directory and `umask 077`; the full rerun then passed. Both runs and the combined environment control are recorded. No production permission check changed. The exact earlier offending inode was not retained, so no single directory or bit is asserted as the cause of every failure. Existing ignores remain unqualified.

All 4,645 frozen file modes and Git blobs match published MVM tree `4cb85849688d8eeb394de780c911c8894cc98206`. The JSON binds the manifest, 13 changed source hashes, commands, executables and results. Legacy runner revision fields name base `c07d912e`; the complete verified source is the published successor. The full MVM workspace test suite is outside this result.

The fork counts 2,956 tracked Rust files: 1,039,567 SLOC and 1,310,430 physical lines across all categories. Its explicit ceilings are now 1,039,600 and 1,310,500. The 309 added SLOC and 330 physical lines include runtime and retained regression controls. No test category or downward ratchet was removed. This is a contributor-fork adjustment, not upstream approval of a larger budget.

## Remaining acceptance

Run the installed Firecracker workload with controller, installer, peer and helpers aligned to this public source, then verify original-owner retirement. Booted-guest recovery, sustained operation, two-host direct/relay WAN, supported platform/capacity limits and FIPS/security qualification remain open. The separate Windows deadline failures also remain open. [Integration record](mvm-integration.md) · [Acceptance issue](https://github.com/R4-Laboratories/SpiralCore/issues/258).
