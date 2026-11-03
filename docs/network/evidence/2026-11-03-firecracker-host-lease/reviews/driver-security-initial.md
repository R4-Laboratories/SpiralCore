# Firecracker lease driver hook: initial security/architecture review

**BLOCK pending the two concrete integration corrections below.** Source-only
review of the uncommitted driver delta from MVM490860c in the isolated
`/Users/casey.allard/spiralcore/local/mvm-host-lifetime-lease/source` worktree.
No code/test edits, builds, tests, remote commands or expanded audit.

Reviewed initial snapshot SHA-256:

- `crates/mvm-backends/src/driver/fc.rs`: `1df4f51d43bf6b02d58b113adb766dfce46215fa937850f3b0d0d7972ddddcc6`
- `crates/mvm-backends/src/fc/daemon.rs`: `10085db661bdbc26a19b95b6fdb6dd7b20af67d5d5fcfc8693a3708930f0be29`
- `crates/mvm-backends/src/driver/fc/host_lease.rs`: `e01f30163e99fb063488c53385c903b1bc7f2dfe08409c97ed6c9da7720eba4c`
- `crates/mvm-vmm/src/driver/prepared_disks.rs`: `7fbd78241ddc20e4d836c20bf8dfb8e8b063b0d0f4e02f710eda05c1426e2ecd`

1. **Lease plus writable disks cannot compose.**
   `reopen_host_lease_script` changes `setsid nohup sh` to bash. The subsequently
   applied `reopen_handoff_script` still requires the sh runner, so every
   lease+nonempty-disks launch returns `handoff launch shape changed`. Keep
   disk reopen before privileged dynamic lease-FD allocation, but validate and
   accept exactly one existing sh-or-bash runner, or render the prefixes through
   one shared composer. Required functional witness must exercise both lease
   and real prepared disk handoff; diskless typed-entry compilation is insufficient.
2. **Leased root boot adds an unconditional sudo dependency.**
   The new driver calls `privileged_writer_descriptor_objects(pid)` directly.
   That function always runs sudo/find. Existing writable-handoff verification
   uses direct proc inspection for root, and the launcher elides sudo there.
   A previously eligible sudo-less root host therefore fails the leased API
   after spawning the VMM. Preserve the root/direct versus nonroot/sudo split
   in the shared bounded inventory seam, or explicitly review a narrower host
   profile before claiming root compatibility. This does not affect lease=None.

Both findings were reported to the lead before this report. They are concrete
functional/profile blockers; no new remote authorization or live-resource
overlap exploit is asserted. Source fixes and focused functional qualification
remain pending at this initial snapshot.

Other inspected boundaries are aligned: the additive concrete API validates
prepared disks and optional verified-kernel recipe, carries the optional lease
through shared boot logic, filters idle-pool claim whenever lease is present
including diskless roles, and preserves the ordinary None path. The fixed outer
and privileged preflights retain canonical scope/sudo/PID/socket choreography;
the intended disk-first privileged ordering avoids dynamic descriptor collision.
The holder object check occurs before guest configuration, but inode inventory
is only custody corroboration, not proof of flock lifetime, process exit or
kill authority. Publicizing the existing inventory function adds no network API.

The reported typed-method GREEN is not a broad415-test pass or a native boot.
Actual sudo/root composition, diskless/verifier roles, real Firecracker holder
retention after owner+worker death, positive readmission and full B6a remain
open. This report grants no installed scope=host or native recovery approval.

## Corrected source delta — 7d031c7f

**PASS for the driver/launcher source delta; the initial two blockers are
resolved.** Reviewed exact clean MVM7d031c7f source, without edits or execution.

- `driver/fc.rs`: `ffa672a931bc3f1a084ea33cfbca1095c320ba8112e5e5c0a21f563648ebe4ef`
- `fc/daemon.rs`: `286a6f0977c06aac592a8b10bbd3b9e50dc98c8e39bc5ef3a15b70704cd53b3b`
- `driver/fc/host_lease.rs`: `e01f30163e99fb063488c53385c903b1bc7f2dfe08409c97ed6c9da7720eba4c`
- `mvm-vmm/driver/prepared_disks.rs`: `a8f55c250b16f58ddc2dfd4e07d0ea41936b4540df18823c9d831d01fb986394`

The production caller and process tests now share `resource_handoff_script`.
The disk composer accepts exactly one sh-or-bash runner and one heredoc; its
prefix is inserted last, so disk FD1000/1001 reopen precedes dynamic lease-FD
allocation. Existing runner scope/root/nonroot structure remains. The lease
preflight still gates outer mutations and privileged exec separately.

The caller's idle shape is now filtered before both pool claim and the later
readiness refill; the nested claim helper also excludes leased boots. Diskless
lease presence therefore cannot claim or create an unleased pool process.
Actual euid selects direct `/usr/bin/find` for root, existing sudo/find for
nonroot. No caller-supplied privilege flag was added. Linux-only runtime module,
getter use and inventory wrapper are confined by cfg; portable constructor
tests remain test-only. The inventory retains its existing16MiB/65,536-object
parser limits and failed-command refusal. Those limits are not total installed
host memory admission, and inode matches remain custody corroboration only.

Read `final-hook-source.sha256`, functional, inventory-constructor and FC
regression receipts: nine focused entries PASS (including one child helper),
two root/nonroot constructor checks PASS, and238 FC regression entries PASS
with the focused cases overlapping. The shared composer test performs an actual
foreground fake exec with occupied lower FDs and exact disk/meta reads at
1000/1001, retains independent SH after parent lease drop, and proves final
release. The stale outer case refuses before mkdir/socket removal/target exec.
These do not boot Firecracker; constructor checks are not root/sudo boot tests.

Original BLOCK/RED records remain retained. Final strict Clippy and platform
qualification are lead-owned gates, not asserted completed by this source
review. SpiralCore ABI export/installed entry, actual VMM retention and native
owner+worker-death/readmission remain pending; this PASS does not clear full
B6a/B6b or installed scope=host.
