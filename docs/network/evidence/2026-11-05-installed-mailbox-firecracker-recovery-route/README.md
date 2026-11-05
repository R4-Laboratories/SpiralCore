# Installed Firecracker mailbox recovery route and worker

Firecracker captures the native process observation from the already-connected
boot API socket before InstanceStart. The original running VM retains the
observation and the armed bridge. Runtime publishes a weak reference to its
original worker only after successful outer launch and original VM retention.
The legacy V1 job keeps its original single-accept behavior.

The V2 worker consumes one replacement attempt, shuts down the old retained
descriptor, and uses the same nonblocking listener and stop wakeup. It checks
the pinned socket and bridge before and after polling and verifies the
candidate peer with the original process observation. Stop serializes with
accept and publication. A wake error does not prevent native stop, and
uncertain mailbox retirement still retains ownership.

Apply [the patch](native-route-handoff.patch) over the preceding pinned MVM
candidate with the [typed handoff](../2026-11-05-installed-mailbox-recovery-typed-handoff/)
and [Runtime deadline](../2026-11-05-installed-mailbox-runtime-deadline/) applied.
[Validation](validation.txt) records exact source hashes and executed Linux
checks. The API socket regression rebinds its pathname while the original
connection is open and verifies the session still observes only that connection.

This is backend and Runtime qualification. The current Hostd and resident
provider still refuse live V2 dispatch. The resident job must carry the
canonical clean-close proof, quiesce the old carrier, pass the original
session deadline, and authenticate the exact enrolled guest before replay.
An installed Firecracker recovery run is required before enabling the profile.
This checkpoint provides no new WAN, wire-byte, CPU, or cryptographic
qualification result.

The locked Linux backend release run passed 246 tests. The same-source Runtime
release binary passed all 19 tests in the focused installed module, including
the wake-failure fault test. The broader installed-prefix filter passed 22 of
23. Its original-handle fixture omits the required installed host agent and
fails before mailbox attachment. The exact same test reproduces the same
failure on the retained earlier binary. The broader failure remains recorded.
The six changed files received independent scoped Code, Rust and Security
review.
