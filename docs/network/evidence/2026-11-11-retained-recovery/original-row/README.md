# A resumed caller must still own its original operation

A VM identifier and lifecycle generation can stay the same while a new request
takes over the daemon's single mutation slot. Two Linux regressions paused an
actual caller, settled its first row in a controlled fixture, reserved a new row
for the same VM, then resumed the original caller. Both failed before the fix:
the stale caller could settle the replacement row, and ordinary registration
could expose a broker endpoint.

The repair captures the accepted operation's monotonically allocated row ID
inside the reservation. Before using a borrowed slot again, the daemon checks
the original cell and row. Its final commit returns that **same checked loan**
for settlement after releasing the filesystem and daemon locks. Normal mutation,
stopped recovery and Preparing replay use this rule. The row ID is bookkeeping;
it grants no permission.

| Paused production caller | Before | After |
| --- | --- | --- |
| Recovery of stopped work | Returned success, lost row 2 and its lease, refunded the slot | Refused on row mismatch; row 2, its lease and its charge remained |
| Ordinary signed registration | Returned success, refunded the slot and exposed a listener | Refused before effects; row 2 remained charged and no listener was exposed |

**Eight exact tests passed on the same repaired executable.** Alongside the two
regressions, six controls covered legitimate signed recovery, a conflicting
journal, full record comparison, ordinary bind/unbind, cancelled replacement
retaining its credit, and two-service replay during Preparing. All observations
in the contention tests precede fixture cleanup. The test hooks exist only in
the test build. [Complete output, timings and binary hashes](results.json).

To reproduce, start from the [previous inventory candidate](../README.md) with
its inventory fix applied. In that disposable MVM source tree, set `REPORT` to
this directory in the SpiralCore checkout and run:

```sh
gzip -dc "$REPORT/regression-fixture.patch.gz" | patch -p1
MVM_EMBED=0 CARGO_INCREMENTAL=0 cargo +1.97.1 test --release --locked --offline \
  -j 2 -p mvm-hostd --lib broker::daemon::row_identity_tests:: -- --test-threads=1
```

Both tests should fail on the earlier behavior. Apply
`gzip -dc "$REPORT/row-binding.patch.gz" | patch -p1` and repeat. Both should
pass. [Source hashes](source.json) bind each patch step and the unchanged test
module; both applied states were reconstructed independently. The patches retain
the [MVM Apache-2.0 license](../../2026-11-11-startup-publication/composition/LICENSE).

These are daemon library tests on the MVM 0.27.0 development composition.
They do not execute the installed worker's shutdown loop, boot a guest, or
measure payload transfer. Next: hand the drain request to the original worker,
keep its first deadline, and verify reply completion before signer shutdown.
