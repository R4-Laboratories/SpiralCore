# Installed supervisor implementation

Tracking: [mailbox #43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
[capacity #49](https://github.com/R4-Laboratories/SpiralCore/issues/49).
Contract: [installed supervisor v1](../api/installed-supervisor-v1.md).
This continues the installed-owner and Firecracker-handoff components. It does
not replace the broader persistent-service, platform or WAN acceptance plans.

## Current implementation and next changes

1. Retain one installation owner through one shared budget and every outstanding
   budget/scope/link/frame guard. Check the fixed profile before beginning an
   epoch; close admission irreversibly and finish only through the original
   owner. Implemented at `112eeeab`; native Mac component checks and the Linux
   legacy-entry refusal have executed. Full release qualification is pending.
2. Propagate transport accounting corruption to worker admission immediately.
   Add focused underflow/poison regressions before changing production code.
   Preserve zero usage and closed admission as distinct from trusted accounting.
3. Add two fixed worker slots with exact owned Child custody. Reserve before
   spawn. A non-Clone token binds authority, slot, serial and role; dropping it
   cannot refund a slot. Reject stale and foreign tokens without taking a
   caller's Child. Actual reaping and trusted backend-exit confirmation are
   separate transitions; reaping alone never authorizes another boot.
4. Connect all worker starts through one explicit installed launch context,
   including Stage One, sender, receiver and verifier. Encapsulate spawn or mark
   a slot Launching before spawning; cancellation of a reservation must never
   cover the spawn-to-bind interval. Add the anonymous live-controller channel,
   fresh challenges, original startup deadline and holder acquisition described
   by the API contract. No legacy fallback or PID-based cleanup in this path.
5. Update the consistent MVM dependency closure to the reviewed backend candidate
   and use its actual Firecracker handoff. Retain HVF worker lifetime custody.
   Qualify one guest, then two, within the existing two-vCPU/two-GiB/no-NIC limit.
6. Execute supervisor/worker death with a surviving VMM, refused successor
   retirement, actual final VMM exit, durable retirement and positive admission
   of the next epoch. Record exact object readbacks, actual both-direction
   traffic, endpoint CPU, memory and verified/lifecycle time against matched zstd.

## Worker retirement and final installation retirement

Slot reuse requires a positively reaped successful Child plus confirmed backend
shutdown. A failed Child or missing shutdown confirmation permanently closes
launch admission; it must not enable the verifier or another reservation.

Final installation retirement is a different operation. After admission closes,
all exact owned Children must be positively reaped and transport usage must be
zero. Reaped failed/unconfirmed slots may then attempt the original
`owner.finish(epoch)`: kernel holder exclusion remains the final requirement.
Busy retains the same closed epoch for retry. Reserved/Launching slots with
uncertain Child custody, live Children and `try_wait` errors block retirement.
Storage or transport-accounting corruption remains fenced and cannot be reset
by the worker ledger. A token's Drop supplies none of these proofs.

## Verification and publication

Use focused real-process regressions and native component tests before the VM
campaign. Capture the original failing assertions/API absence, exact source pins,
corrected results and independent code/security reviews. Keep the new supervisor
records separate from the sealed Firecracker-handoff packet. The first broader
Linux debug run at `9ff7b247` passed76 entries and failed one retry-deadline check;
retain and investigate that result before a release claim.

Publish reviewed/tested slices through PRs to `origin/main`, keeping #42/#43/#49
open until their complete acceptance criteria have evidence. Keep existing
session reports at orchestrator scope until actual installed all-path custody
and recovery execute. B6b plaintext/cache ownership, completed-tail recovery,
production retention/>2,048 Calls, pipelining, fleet fairness, platform/WAN
acceptance and the final MVM contribution remain in the parent delivery plan.
