# Concrete FC hook: focused final CODE/RUST delta

**PASS — all three initial source blockers resolved at7d031c7f.** Reviewed clean
frozen MVM7d031c7f3fbeef62826712d2976a764c62cd5852; reuses the prior helper and
initial driver review without a broader source/inventory audit. No builds,
tests or product edits were performed. driver-code-initial.md remains retained.

1. **Disk/lease composition resolved.** Production and functional fixtures now
   call the same resource_handoff_script. Each transform requires exactly one
   heredoc and one sh-or-bash runner; the disk transform accepts the already
   leased bash form. Injecting the disk prefix last places its reopens first
   in the privileged body, before Bash dynamically allocates lease descriptors.
   There is no competing test-only composition implementation.
2. **Idle claim/refill resolved.** Caller idle_shape is filtered for Some lease
   before passing it to claim and before the later refill. Diskless leased
   boots therefore cannot claim or create an unleased pool process. The callee
   also preserves its guard against a Some-lease claim. None follows the shared
   previous boot/scope implementation.
3. **Root profile resolved.** The actual-uid wrapper selects one common builder:
   root invokes fixed /usr/bin/find directly; nonroot retains sudo--fixedfind.
   This preserves canonical root sudo elision. Command-construction tests use
   explicit uid inputs and do not claim privileged backend execution.

Public lease types/methods and optional lease arguments/call expressions remain
Linux-gated. The runtime inventory getter is Linux-only; its pure command builder
is available under Linux or test, so portable tests use `_as(pid,uid)` without an
unused runtime getter on Mac. No unguarded Linux-only type leaks into non-Linux
signatures. Disk validation, optional verified-kernel borrowing, canonical scope
and parent custody through boot/error cleanup remain intact. Inventory is still
object corroboration, not proof of a live flock or process exit.

Read actual retained disk-composition RED and corrected receipts:9 focused tests
pass,0 failed/ignored,409 filtered;2 inventory-role command tests pass,1060
filtered;238 FC regressions pass,180 filtered. These scopes overlap; they are
not249 new independent scenarios or a whole-suite execution. The foreground
exec fixture forces low FDs occupied and uses actual1000/1001 disk descriptions,
checks their bytes, drops the parent lease, proves EX remains refused, then
proves final EX after target exit. The stale outer fixture preserves the real
outer mutations and proves no mkdir/socket removal/target before refusal.
Owned subprocesses are reaped and reader threads joined with event watchdogs.

The four backend file pins match the focused receipt. Inventory's earlier
hook-corrected pin77d31c21 is superseded by finala8f55c25 only for changing the
runtime getter cfg from Linux-or-test to Linux; its Linux executable body is
unchanged. Exact-final Clippy was still running at this review, so no final
Clippy PASS is claimed. Mac build execution and complete release gates remain
separate from this source assessment.

Exact final SHA-256 pins:

```text
ffa672a931bc3f1a084ea33cfbca1095c320ba8112e5e5c0a21f563648ebe4ef  crates/mvm-backends/src/driver/fc.rs
286a6f0977c06aac592a8b10bbd3b9e50dc98c8e39bc5ef3a15b70704cd53b3b  crates/mvm-backends/src/fc/daemon.rs
e01f30163e99fb063488c53385c903b1bc7f2dfe08409c97ed6c9da7720eba4c  crates/mvm-backends/src/driver/fc/host_lease.rs
73a6373980834a5a2c8ed36362b3bde890e0ffe55c10ceb2e42f75efeafab607  crates/mvm-backends/src/fc/host_lease_launch_tests.rs
a8f55c250b16f58ddc2dfd4e07d0ea41936b4540df18823c9d831d01fb986394  crates/mvm-vmm/src/driver/prepared_disks.rs
```

No source blocker remains in this fix delta. Actual root/sudo Firecracker
launch, live VMM FD/EX-contention after owner/worker death, positive retirement/
next epoch, and SpiralCore's trusted consumer export/default-path integration
remain unqualified. This is not full B6a, dependency delivery or a native/WAN/
performance claim. Root owns further integration and exact-source qualification.
