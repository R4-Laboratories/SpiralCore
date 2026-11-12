# Coordinator retirement after a failed launch

The first addressed Firecracker launch stopped before either guest booted. Its driver exited after 212.49 seconds with a failed report. The original coordinator continued running with its signer. At 10:43:36 UTC on November 12 its durable tenant record still showed Active, revision 70, with zero pending launches. The registration journal and durable mailbox-owner locator directory were empty. The driver had already dropped its original child process handle. Its exit status for the coordinator remains unknown. [Failure report](README.md) · [Timestamped observation and source hashes](watcher-retirement.json)

The session and health watchers both wake every two seconds. A session probe takes the shared daemon mutex briefly to identify the original coordination gate. The original implementation returned immediately on contention and published an unknown session count. The idle observer then restarted the required 40-second zero-work period. The captured Linux log prefix contains 314 such session refusals. The longest gap between consecutive refusals in that prefix was 10.000079 seconds. The diagnostic identifies daemon lock contention. The log does not identify which task held the lock. [Original host source](https://github.com/Casey-allard/mvm/blob/5ae2cdc4977e12a574a5baa2e4f0522c3f7a1ae7/crates/mvm-hostd/src/host_agent_idle.rs)

## Change and verification

The blocking watcher now retries transient daemon mutex contention with exponential backoff from 1 to 32 milliseconds under one five-second admission deadline. It releases both locks before sleeping. Each reservation still requires the active tenant generation and original daemon ledger in the same file-gate turn. A closed generation, unavailable ledger or file error refuses admission. The coordinator still checks its current registration inventory, outstanding work and retained resource owners before recording a completed drain. [Reviewable patch](watcher-retirement.patch) · [Tested host source](https://github.com/Casey-allard/mvm/commit/0945135d205ed6461bcdf4086229e7c7953ae720)

| macOS arm64 host test | Recorded outcome |
|---|---|
| Hold the original daemon mutex for 40 ms while the original watcher attempts to acquire a resource lease | Original version failed 0/1 with the same diagnostic as the Linux run. |
| Repeat against the corrected host and then fence the generation | The original watcher acquired and settled one turn. A subsequent reservation after the fence was refused. |
| Hold the lock through a short explicit deadline | Admission timed out with zero ledger entries. |
| Complete watcher module | 18 tests passed, including existing idle-decision and inventory controls. |

Run the component suite with the following command at the tested host revision. The failed regression source is also available. [Original regression](https://github.com/Casey-allard/mvm/commit/a5cddf9ce327885e1c6c4e1bf643d71ad6ed10a3)

```sh
cargo test --locked --offline -p mvm-hostd --lib host_agent_idle::tests:: -- --nocapture --test-threads=1
```

This host-only result leaves the original Linux test installation retained. It adds no guest, transfer, WAN or throughput measurement. A new native run must separately report the original coordinator’s exit and tenant lifecycle. [Machine-readable results](watcher-retirement.json)
