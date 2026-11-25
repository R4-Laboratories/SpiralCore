# FlowMux session closure with a pending audit flush

MVM development commit `df5941ba4262bd9547b4ab47f07147add5ebdd80`, tree
`dd139ae31dcdca4004366d8c052dd27f52257bce`, extends the test source from
the reviewed epoch admission checkpoint `abd5155f`. The two changed files
strengthen the existing authenticated FlowMux session-completion test and allow
its frame helpers to count I/O at the actual Unix carrier. The production
endpoint, installed dispatch and audit authority remain on the existing path.
[Development source](https://github.com/Casey-allard/mvm/commit/df5941ba4262bd9547b4ab47f07147add5ebdd80).
[Admission checkpoint](../2026-11-25-epoch-admission/README.md).

The fixture opens an admitted local TCP flow through an authenticated session,
then compares the exact application payload in both directions, including
trailing zero bytes. A guest write-half shutdown triggers ordinary session
completion while the guest still holds its read half. A test sink signals
entry into audit-health flush and waits for a release message. The fixture
observes EOF on both the carrier and the upstream TCP socket, and confirms
that the host thread is still waiting on the flush, before releasing and
joining it. One absolute one-second deadline covers entry into the flush and
both EOF observations. The test sink's own bounded wait allows failed fixtures
to retire. The failed-flush error and the required ordering of flow decisions
and their audit records are covered by the existing tests in the same scope.

## Executed qualification on Wick

The Linux x86-64 `mvm-hostd` binary compiled with Rust 1.97.1. Its FlowMux
audit scope passed 13 tests. The closure test then passed 32 additional
repetitions on the identical executable. Formatting and strict all-target
`mvm-hostd` Clippy passed.

| Observation across 32 repetitions | Minimum | Median | Maximum |
|---|---:|---:|---:|
| Guest-to-upstream TCP payload | 13 B | 13 B | 13 B |
| Upstream-to-guest TCP payload | 7 B | 7 B | 7 B |
| Guest-to-host encrypted carrier | 192 B | 192 B | 192 B |
| Host-to-guest encrypted carrier | 367 B | 367 B | 367 B |
| Setup through accepted TCP flow | 1,093 µs | 1,130.5 µs | 1,941 µs |
| Bidirectional transfer through verified readback | 211 µs | 217 µs | 297 µs |
| Write-half shutdown through both EOF observations | 39 µs | 44 µs | 77 µs |
| Elapsed through joined session worker | 1,393 µs | 1,455.5 µs | 2,397 µs |

The carrier counters cover encrypted frame and credit bytes during the measured
transfer interval. Authentication, opening and shutdown are outside those counters.
The joined-worker time ends before local fixture descriptors are dropped. The
13-test process took 0.44 s wall, 0.01 s user CPU and 0.01 s system CPU with
43,876 KiB peak RSS. The test build took 82.28 s wall, 124.37 s user CPU and
6.86 s system CPU. These measurements cover processes sharing one local test
host. Per-endpoint transfer CPU and WAN traffic remain unmeasured.

The exact 4,877-file source manifest has SHA-256
`8dfaf79333538d8423053b611cef30565a3c21a9d449dce9710f53e81f2c0b69`.
The retained executable has SHA-256
`56a025cc9cb375e717677dcaaaed17cc1044353f4b351f6740dbc5d889e3ac79`.
The verified 34,928,801-byte compressed archive has SHA-256
`c09b452c35e5a437c1f96566f8847c58eaf88415c582ece7ff7d2423f4bee208`.
Raw run records and the archive are retained on Wick's HDD.

## Remaining delivery gate

This witness starts from ordinary session completion and holds a synthetic
audit-health flush. The canonical watcher still waits for its plan-bound
revocation write before process exit. Its qualification requires ledger-triggered
closure of the active listeners and relays under a stalled writer and custody
of the signed revocation evidence. Ari's reply on that ownership question,
the original installed audit interface and a compatible publisher-signed
guest runtime remain pending. Installed pins remain MVM `f2ddbcbf` and
SpiralCore `8994951b`. Original readiness, both #258 criteria and full MVM
acceptance remain open. [Watcher discussion](https://github.com/tinylabscom/mvm/issues/5848#issuecomment-7171352474).
[Installed audit request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842).
[Storage qualification](https://github.com/R4-Laboratories/SpiralCore/issues/258).
