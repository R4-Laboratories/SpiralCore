# Linux installed native-death acceptance runner

Disposition: **HOLD on failure-path cleanup**, not a production-source finding.
Reviewed native_death.rs SHA-256
`611f92df37bca4ab28fd2023a65cc9513071f06b55034d33fda9f9cd5bce0d02`.
No build, VM execution or source changes by this reviewer.

Normal-path identity is sound for this scoped witness: /proc stat suffix starts
at field3, so nth(19) is starttime field22; birthtime and identity are checked on
both sides of pidfd_open. Worker candidate must be a live child of the exact
owned supervisor, same host executable inode, expected kernel/role/profile/socket
shape and exact run MVM_HOME. FC candidate requires the pinned executable inode,
runner euid, unique installation holder inode, run console and real KVM handle.
Signals use retained pidfds, never a numeric-PID fallback. SIGSTOP is followed by
state observation; death is positively observed through pidfd. Actual reconcile
Busy while only FC survives provides SH proof; an inode match alone does not.
After exit, canonical reconciliation and next-epoch production CLI are required.

**Blocking failure window:** if backend discovery at lines313-339 times out
after FC has booted, no backend pidfd is retained. Unwinding invokes worker
ExactProcess Drop (131-140), SIGKILLing it, then ExactChild Drop (143-148), killing
the supervisor. A live FC can survive both and escape the runner's cleanup
custody. Failure before worker capture can likewise leave an already-booted
backend without an observer. The installation's durable holder fence still
protects readmission; that is not proof that the fixture cleaned up its VMM.

Before qualification, make pre-discovery errors preserve cooperative owned
supervisor/worker cleanup under a finite deadline, or capture the exact backend
before abrupt worker termination. If cleanup cannot be positively established,
retain explicit unresolved outcome/custody and stop further launch; do not mark
cleanup successful or kill a guessed PID. ExactChild::drop also calls wait()
without a bound; use the fixture's declared finite failure policy rather than
an unbounded destructor wait. After backend capture, the retained pidfd Drop
targets only that exact process, including a stopped process.

The recovered command's success is necessary; retain and separately verify its
actual exact-readback result before reporting the new epoch as an exact-transfer
qualification. This runner is Linux/KVM-only and cannot establish macOS recovery
or the whole B6a/capacity/WAN goal.

## First executed discovery failure, read-only diagnosis

The retained native-death-first receipt is exit1, specifically "no exact live
KVM backend with installation holder". Worker discovery therefore succeeded.
The interrupted production CLI completed its16MiB transfer successfully and
reported Clean; processes-after is empty. No signals or new VM were issued by
this diagnostic review.

The retained native-death-run.sh executes the runner as callard/kvm. The canonical
nonroot FC launch is sudo-elevated, while backend_matches line224 requires the FC
/proc uid to equal the runner euid. That refuses the root backend; nonroot access
to its fd inventory is an additional obstacle. Changing the UID comparison alone
cannot establish fd inspection or signal permission. No live fd snapshot remains
after this completed invocation, so none is claimed.

Minimal next qualification profile: run the entire fixture and its NEW private
canonical installation as root, exercising the existing direct-root FC path and
keeping canonical owner queries under the provisioning UID. Label this a
root-installed Linux recovery witness; it does not qualify nonroot recovery.
Do not mix a root observer with a callard-owned installation and silently bypass
canonical UID checks. A nonroot privileged observer/command helper would require
its own narrow authority contract. Apply the pre-discovery cleanup correction
before repeating either profile.

## Cleanup correction source disposition

**PASS for the narrow fixture source correction**, SHA-256
`81a6c1e52ab81c751845b75d534932ddd7be976ad0edabb9e18b345bc969573a`.
The prior pre-discovery cleanup HOLD is resolved: ExactProcess starts unarmed;
worker is not abruptly terminated unless an exact backend observer has also
been captured. Discovery failure lets the owned production CLI perform normal
cleanup for at most150s; unknown completion emits HOLD without a guessed kill
or success claim. Once backend capture succeeds, both exact pidfd guards are
armed, use bounded5s exit observation, and explicitly report unresolved exit.
No new VM/test was executed by this review; the lead retains the actual guard
RED/GREEN records and owns native execution.

The fixture now explicitly requires root and uses a new root-owned installation.
It captures the backend by executable inode, unique holder and run console
before waiting for guest_ready and stopping it. The earlier KVM-fd predicate was
removed: report actual boot through the guest_ready observation, not a retained
KVM-descriptor observation. Runtime proof still requires the complete surviving
backend/Busy/unchangedActive/positiveexit/Clean/newEpoch chain and exact recovered
readbacks. This pass permits the scoped root-profile witness; it does not itself
qualify recovery, nonroot death handling or macOS.
