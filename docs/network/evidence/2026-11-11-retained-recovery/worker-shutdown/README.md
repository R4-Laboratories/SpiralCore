# Linux host-agent shutdown after a registration journal failure

The installed host-agent worker now finishes an interrupted registration through the original daemon and signer. In an isolated Linux fixture, a client sent a signed Register while the canonical journal writer's temporary path was unavailable. It received an error, restored the path, and sent a fresh signed Deregister for the same VM. The client read the complete canonical `Ok` frame and verified an empty registration journal.

| Linux process test | Earlier worker | Repaired worker |
|---|---|---|
| Signed deregistration after journal failure | Reply received. Worker missed the 20 s guard. Harness reaped one signer. | Reply received. Worker collected its one original signer and exited 1. Harness reaped zero. Full fixture **32.65 ms**. |
| Ordinary idle shutdown | Worker collected its signer and exited 42. Full fixture 1.035 s. | Worker collected its signer and exited 42. Harness reaped zero. Full fixture **1.033 s**. |

Exit 1 identifies the registration recovery path. The test checks its diagnostic as well as the complete framed reply. Exit 42 identifies ordinary idle retirement. The process fixture uses the real host-agent and signer executables in a new private home. The source also includes a controlled task-join test for the ordering of control-loop completion and signer shutdown. There is no client acknowledgment after it reads the frame.

## Changes and results

The daemon keeps the first shutdown deadline for its lifecycle generation. A verified inline control request retains the same lock through dispatch and bounded reply writing. Shutdown waits on that lock before reconciling the original registration. The worker reuses an existing durable Draining state and can establish the forced shutdown fence while the controller holds the daemon mutex. An idle proposal must still belong to the current Active generation and revision.

Five coordinator tests, two daemon tests and two process tests ran on each source state. The earlier run passed six and failed three. The repaired run passed all nine. The additional failures reproduced contention on the daemon mutex and a queued idle proposal claiming a shutdown another owner had already started. [Earlier results](before.json) and [repaired results](after.json).

The 32.65 ms covers fixture setup, both client requests, process exit and test cleanup. GNU time reported user/system CPU of 0.00/0.00 s at 0.01 s resolution, and maximum resident set size of 8,160 KiB for the repaired recovery invocation. That precision cannot establish zero CPU cost. These lifecycle cases transferred control frames only. Payload fidelity, data-plane bytes, WAN timing and guest retirement require separate tests. The next ownership test is retirement with an injected per-VM mailbox helper. The combined service still requires one-guest and two-guest Firecracker qualification.

## Reproduce

Start with the [original-row source](../original-row/README.md) and its repair applied, using its MVM 0.27.0 composition. Set `REPORT` to this report directory. Apply the first patch in the MVM source tree and run the scoped tests. Apply the worker-fence patch and repeat the same commands. The third failure before the repair is sensitive to process scheduling. The two coordinator tests exercise the lock and stale idle conditions directly. [Per-file and patch hashes](source.json).

```sh
gzip -dc "$REPORT/worker-handoff.patch.gz" | patch -p1
export MVM_EMBED=0 CARGO_INCREMENTAL=0
cargo +1.97.1 test --release --locked --offline -j 2 -p mvm-hostd --bin mvm-host-agent worker_deadline_tests:: -- --test-threads=1
cargo +1.97.1 test --release --locked --offline -j 2 -p mvm-hostd --lib broker::daemon::drain_request_tests:: -- --test-threads=1
cargo +1.97.1 test --release --locked --offline -j 2 -p mvm-hostd --test host_agent_idle_collection retained_registration_reply_precedes_original_worker_recovery_exit -- --exact --nocapture
cargo +1.97.1 test --release --locked --offline -j 2 -p mvm-hostd --test host_agent_idle_collection idle_42_follows_original_signer_collection -- --exact --nocapture
gzip -dc "$REPORT/worker-fence.patch.gz" | patch -p1
# Repeat the same four cargo commands.
```

The retained runs compiled once per source state and invoked each of the nine filters individually with `--exact --nocapture --test-threads=1`. Each run verified all 4,496 source hashes and the executable hashes before and after execution. The patches retain the [MVM Apache-2.0 license](../../2026-11-11-startup-publication/composition/LICENSE).
