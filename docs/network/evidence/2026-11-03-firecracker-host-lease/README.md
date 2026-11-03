# Host lifetime handoff components: implementation checkpoint

This packet records two cooperating local components, not a completed installed
MVM service. The network/envelope protocol and codec are unchanged.

## Source and executed scope

- MVM backend candidate
  [`7d031c7f3fbeef62826712d2976a764c62cd5852`](https://github.com/Casey-allard/mvm/commit/7d031c7f3fbeef62826712d2976a764c62cd5852),
  on `feat/host-lifetime-lease`, based on canonical-mailbox preparation ae12e9d.
  Additive concrete FcDriver entry, shared fixed launch composer, independent
  SH/expected-state digest verification across sudo, disk-FD coexistence and actual-writer
  descriptor corroboration. Lease-present boots disable both idle claim/refill;
  root inventory uses direct find, nonroot retains fixed sudo invocation.
- SpiralCore holder export `88f4ea56eb3b7409d09af77faef916f3d9f157d8`,
  product SHA-256 `fc0b3dd44256da7bcfd7cdc793e77f0312ce421d365d6e295fda68ce17225279`.
  The original acquired generation is retained, fresh canonical state is checked
  and exact CLOEXEC directory/holder duplicates plus full176-byte digest export.
  Original guard drop cannot release SH while the handoff still owns its copy.

| Executed check | Result | Scope / overlap |
|---|---:|---|
| MVM backend focused tests | 9 pass | Includes1 child helper,1 signature test and3 actual foreground-launch composition tests; no VM |
| Selected MVM Firecracker regressions | 238 pass | Includes the same9; 180 other package entries filtered |
| Root/nonroot inventory selection tests | 2 pass | Command construction; no root/sudo backend execution |
| MVM backend/vmm strict Clippy | exit0 | Exact final source, library/tests |
| Holder export Linux harness | 15 pass | 4 unit+11 lifecycle incl1 helper; serial |
| Holder export Mac27 ARM64 harness | 16 pass | 4 unit+12 lifecycle incl1 helper; serial and native ACL case |
| Holder export strict Clippy | both exit0 | Separate native Linux/Mac targets |
| Holder export changed Linux line coverage | 18/19,94.74% | Whole mapped420/447,93.96%; Mac cfg excluded, branchNA |

The functional launch fixture uses real descriptor slots1000/1001, child-only
FD occupancy, exact disk reads and an event barrier. After parent lease drop,
an independent EX attempt remains refused until final child close. The stale
outer launch fixture verifies refusal before mkdir, socket removal or target
execution. They run the same composer as production, with bounded foreground
fake targets. They do not boot Firecracker.

## Records

- `test-intent.md`, `intent/`: original absent-source/API test intent and harness.
- `validation/`: original missing APIs, failing disk/lease composition, corrected
  runs and exact source pins. `owner-handoff-linux/` is the exported-owner
  missing-method RED,15-pass result, Clippy and source receipt.
- `coverage/holder-handoff/`: separate instrumented target, protected objects,
  child-inclusive profiles, compact line counts and collector. No CPU claim.
- `reviews/`: original BLOCKs and narrow final code/security resolutions.
  An early review can correctly state that a later receipt was not yet available;
  actual final outputs are retained separately rather than rewriting chronology.

The initial driver integration had three real defects: lease/disk sh-to-bash
composition refusal, a remaining unleased idle refill and unconditional sudo
inventory for root. All have source corrections and passing focused checks;
the original failures/reviews remain. Earlier marker absence error handling was
also corrected to require successful bounded directory inspection. Root/sudo
native execution, actual marker EIO/timeout injection and real VMM final-close
proof are unexecuted, not inferred from these tests.

## Remaining integration

The backend source is public in the fork. SpiralCore's default MVM dependency
still points at ae12e9d until consumer integration deliberately updates its
consistent dependency closure; these component results do not claim the new
boot path is already the installed default.

Next is the [installed supervisor contract](../../../api/installed-supervisor-v1.md):
one consumed owner, one budget retained through every guard, explicit grants
for all workers/verifiers and a live-controller channel before boot. Then run
actual Linux/Firecracker and Mac/HVF owner/worker-death→backend exit→retirement
and readmission within2guests/2vCPU/2GiB/no-NIC. Full B6a, B6b, long sessions,
capacity, canonical WAN and upstream acceptance remain open. No new bytes, CPU,
transfer-time, energy or government-qualification result is established here.

Builds and profiles stay on Wick. No VM disk or payload store was copied to the
Mac. The new disposable Mac test target was cleaned after qualification:
238files/34.8MiB, in addition to the earlier component target cleanup.
