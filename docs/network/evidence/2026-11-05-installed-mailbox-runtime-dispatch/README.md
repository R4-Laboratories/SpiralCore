# Original Runtime mailbox slot and dispatch boundary

The installed Runtime pins the validated state-directory descriptor during
original launch preparation. It moves the one-shot mailbox handoff into the
original slot before requesting its socket job, binding, or spawning.
Runtime closes admission, joins the worker, and verifies socket cleanup
before releasing the handoff. The verified parent supplies the job.

The original slot distinguishes startup in progress, stop requested,
driver dispatch attempted, and terminal startup. Stop competes with
dispatch only after the original mailbox handoff arms the slot. The driver
boundary records an attempt after the original lease claim and immediately
before the native driver call. A stop that wins before dispatch cannot
release custody while startup is still pending. Once the launch returns a
terminal pre-dispatch failure, the real stop path can retire the mailbox
without supplying a native exit receipt. An attempted dispatch still needs
the original native exit proof before mailbox refund. Ordinary installed
launches retain their prior pending-stop behavior.

The final protected Linux x86_64 binary passed 11 installed custody tests
and one rejected-kernel-recipe control. The actual Unix worker test uses a
fixture permit and charge. It proves socket retention while startup is
pending and socket cleanup through the real stop path after a terminal
pre-dispatch failure. The attempted-dispatch and default-off tests exercise
slot policy without invoking a native driver. Earlier failure receipts
include missing test assets, missing API, a lingering pre-dispatch socket,
premature cleanup, and unintended default-off cancellation.

- [Replayable SDK patch](original-runtime-mailbox-attachment.patch)
- [Phase receipts and source identities](result.json)
- [Protected test output](installed-and-kernel-results.txt)
- [Next resident session contract](resident-session.md)

Patch replay reproduced all six changed files byte for byte. Independent
architecture, code, Rust and security review accepted the scoped candidate.
Hostd and original parent native gates still refuse selected launches. The
shipping parent has no mailbox session job yet. This checkpoint started no
native guest and adds no WAN, rate, or performance result.
