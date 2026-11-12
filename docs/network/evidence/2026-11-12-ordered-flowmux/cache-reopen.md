# Linux cache reopen during concurrent CLI launch

The Linux contract job failed at the immediate cache reopen in the persistent HTTP suite. The server had returned and the queued connection was still available. On Wick, the same pair of unmodified tests failed at the same assertion on the second run when `strace` delayed concurrent `execve` entry by 15 ms. The hosted log reported `Storage`. The Linux reproduction captured the failing system call. [Hosted failure](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43271817693/job/131370059480) · [Tested fix](https://github.com/R4-Laboratories/SpiralCore/commit/ac35b6c8e30257451a86ce521d0006482eb31807)

| Time on test host | Operation |
|---|---|
| 13:05:33.489862 | Server acquired an exclusive nonblocking flock on the cache’s private `.owner` file. |
| 13:05:33.489871 | Another test thread began spawning the WAN CLI. Its child inherited the open file description until exec. |
| 13:05:33.490392 | The original server closed its owner descriptor. |
| 13:05:33.490561 | The new endpoint attempted to lock the same file. The kernel returned `EAGAIN`. |

An initialized Unix object cache now retries a contended lock for up to 50 ms, polling at most every 2 ms. An uncontended open still takes the first lock immediately. After waiting, the endpoint checks the named file against its locked descriptor, validates the private root and profile, and checks the deadline before proceeding. A live writer remains exclusive. New-cache creation, Windows locking and MVM’s installed service ownership retain their previous behavior.

| Verification | Result |
|---|---|
| Library, CLI and persistent HTTP suites on Linux x86-64 | 48 + 9 + 11 passed. Strict library Clippy passed. |
| Same three suites on Apple Silicon macOS | 48 + 9 + 11 passed. |
| Original two tests with delayed exec on the fixed main-line source | 30/30 pairs passed. Three trials observed actual lock contention. |
| Same two tests with the fix applied to the FlowMux candidate at `61339f6c` | 30/30 pairs passed. Twelve trials observed actual lock contention. |
| Storage controls | A temporary duplicate can drain. A live owner still prevents reopen. A changed profile and a replaced lock file are refused. |

To reproduce on Linux, run `cargo test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --test wan_persistent --no-run`. Run the printed test executable under `strace -f -y -tt -e trace=flock,clone3,clone,execve,close -e inject=execve:delay_enter=15ms` with `--test-threads=2` and `--skip` for all tests except `invalid_keep_alive_plan_is_rejected_before_cache_creation` and `legacy_zero_duration_returns_ok_without_accepting_or_processing_requests`. Inspect the flock return values. The execution delay changes scheduling, so these runs qualify cache handoff correctness. They do not measure WAN speed or throughput.

The ordinary Windows Server 2025 stream timed out separately at block 2,352 in that platform run. That release gate remains open. [Platform results and independent Windows timings](https://github.com/R4-Laboratories/SpiralCore/issues/44#issuecomment-6831573987)
