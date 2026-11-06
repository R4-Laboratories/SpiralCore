# Installed supervisor: retained candidate evidence

Tracking: [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).
This packet continues the sealed Firecracker handoff packet without rewriting it.
The bounded implementation merged through PR#71. Its
[final release qualification](release-qualification/README.md) retains exact
hosted source/tree custody, native client/storage results, scoped Linux coverage
and independent review. Native HVF installed recovery, aggregate
plaintext/cache/workspace ownership and the wider Ari delivery criteria remain
open. The historical candidate failures below retain their original scope.

The [results report](../../installed-supervisor-results-2026-11-03.md) includes
the later Linux 16 MiB pair and all four native macOS/HVF pairs. Original
records are in `native/final-linux/` and `native/hvf-transfers/`; the smaller
Linux lifecycle loss below remains part of the result.

## Hosted capture gate remains unresolved

Source `4a86c7e9` passes all seven portable/storage rows, Linux host contracts,
all four Unix owner rows and the registered candidate-CPU campaign. The Intel
portable first attempt failed toolchain DNS download before tests; the explicit
failed-job rerun passes.

The unrelated TUN campaign `41757732453`, attempt 1, failed its first strict
capture gate: pcap/captured 34,659 versus filter-received 34,660, zero reported
drops. All application and gateway predicates passed, but the capture gate is
**FAILED**. A two-cell default/immediate capture diagnostic on Wick passed both
and did **not** reproduce the gap; see `validation/tun-phase2/`. No driver,
codec, assertion or limit changed. A requested same-source hosted rerun did not
execute: the workflow intentionally admits only attempt 1. Its skipped status
is not a successful qualification or a repair. Subsequent source qualification
must retain this failure rather than claiming that a later pass explains it.

## Actual installed Linux transfers

Original full JSON is in `native/`; `functional-summary.json` derives selected
fields without changing the originals. Requested tiers are4096 and1048576 bytes.
The mixed corpus's eight exact target objects are each1047202 bytes, rather than
exactly1MiB. Combined additionally publishes a3382-byte checkpoint and48-byte
schema. Both arms send the same eight targets.

| Cell | Canonical both-direction bytes | Physical vsock bytes | Sender CPU, us | Receiver CPU, us | Lifecycle, us |
|---|---:|---:|---:|---:|---:|
| One guest, whole-zstd,4096 tier |448815|448815|76224|62971|438250|
| Two guests, whole-mixed,1MiB tier |7353966|14707932|343390|302224|1236986|
| Two guests, combined-cpu,1MiB tier |4922739|9845478|309702|257658|1245464|

All three report successful exact reopened receiver objects. The two-guest
cells have8 and10 independent keyless verification readbacks respectively,
including Combined's setup objects; their primary and verifier workers report
the same installed epoch and positive backend exit. Two guests are the maximum
simultaneously booted; each is1vCPU/1GiB, without a guest NIC.

The single fixed-order pair shows33.060% fewer bytes,12.121% less aggregate guest
endpoint CPU, and **0.685% longer lifecycle** for Combined. Sender worker CPU is
119615us versus86668us, another retained loss. The host is shared, including
other bounded builds/local model work. This is not a randomized acceptance
campaign, an optimization attribution, WAN, send-only or energy evidence.
Worker/VMM CPU overlaps nested guest activity; do not add those samples to guest
CPU. `functional-summary.json` retains separate control/input, worker, boot,
backing synchronization, observed VMM CPU and orchestrator costs. Peak resident
memory was not measured by these cells; the configured guest RAM ceiling and
serialized-buffer high-water mark are not a process RSS measurement.

## Actual supervisor/worker death and next-epoch transfer

`native/native-death-monitor-root.txt` and its exit0 record the final native
witness. The Rust acceptance runner lives at
`crates/spiralcore-mvm-live/tests/installed-supervisor-harness/src/native_death.rs`.
It uses a nonroot installation and the existing delegated login/KVM scope;
an explicitly privileged observer captures/rechecks live executable, UID,
birthtime, exact private holder/run identity and pidfds. It never attaches using
a persisted PID or kills another lab's process.

After actual guest readiness, the fixture stops FC, kills its exact owned
supervisor and worker, then proves canonical successor reconcile returns Busy,
begin returns RecoveryRequired, and original Active bytes/epoch remain unchanged.
After positive FC exit and the exact retained sudo launcher's exit, canonical
retirement succeeds. A production next-epoch4096-tier transfer then succeeds and
ends Clean at generation2, with exact readback in `recovered-next-epoch.json`.
This proves this Linux recovery path, not unprivileged observation authority or
native macOS/HVF recovery.

Earlier attempts remain recorded. The initial native launch lacked the literal
Firecracker command in sudo's PATH; a guarded temporary link corrected that
prerequisite and was removed. The65536 mixed tier was refused before boot because
the existing campaign permits1MiB/16MiB. The first death fixture could not inspect
root FC as the nonroot observer. A direct-root fixture then refused the missing
delegated user bus before VM activation; no host/cgroup configuration was changed.
The first privileged-observer death witness proved Busy with surviving FC but
still found Busy after FC exit: sudo's stopped job-control monitor retained a
real SH descriptor. The final runner captures that monitor before signaling,
continues it after FC exit, and positively observes its exit before retirement.
Original failures are not replaced by the successful final row.

## Component and release scope

RED/GREEN records cover accounting corruption, exact Child custody, Drop/drain,
spawn adoption, fresh worker facts, installed-only entry, missing assets and CLI
preflight. Review receipts separate the channel author's work from independent
controller and channel verdicts. Source and binary identities retain each scope.

The first full parallel100-test library run passed95 and failed5 with Busy/fence
observations. A same-binary installed-only serial run passed27/27, while16-thread
execution failed11/27. The deterministic `validation/fork-window.rs` witness
proves CLOEXEC descriptors still retain flock through a held pre-exec fork window:
EX is Busy until exec/reap. Fault hooks are thread-local and fixture-specific.
The release workflow already requires `--test-threads=1` for this process/lock
suite; individual concurrency tests retain their own intentional threads. No
production lock or refusal assertion was relaxed. Final whole-source results
are retained separately from these diagnostic scopes.

## Storage custody

The inactive primary checkout build cache was archived privately on Wick before
cleanup. All18439 archive entries, file lengths/hashes and link targets verified;
regular file bytes totaled6953966182. Archive bytes2383374947, SHA-256
`6f9a89ba2ab31d1579175a6b811548f90289ac17c0c3d543383828ae82255ea3`.
Manifest SHA-256
`29dc088075d79d7eab544207b88163fe1cdfd454fdb3093e26afccb20c728489`.
No open files or tracked paths were found before exact-target cargo clean,
which reported6.5GiB removed. Compact custody receipts are retained here; the
private raw build archive is not published as project source. Source, local
runtime, saved sessions and historical evidence were preserved.
