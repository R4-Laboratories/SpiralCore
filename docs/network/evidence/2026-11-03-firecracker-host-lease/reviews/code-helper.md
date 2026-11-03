# Firecracker host-lease helper: focused CODE/RUST review

**HOLD at570cb8f — marker-inspection error handling requires correction.**
Reviewed isolated MVM commit570cb8f49285381ffc259479e10dcc99f1433251,
driver/fc/host_lease.rs SHA-256
b50e72efe3bf1e60f1262d9d41b606e250cd1e962f50b23d5153ab170d2f1c91,
tests ca45602497c59d4060c72e3d1f1867ec5a2bccb7de81a2726546b538bc5a29d4,
and the proposed host-lease contract. No edits/builds/tests/remote changes were
performed. This source has no driver/daemon hook yet; it cannot establish actual
Firecracker custody or full B6a.

The independent security finding is valid: `! -e ... && ! -L ...` interprets
inspection failures as absence as well as genuinely missing markers. In
particular, an I/O error inspecting a durable intent marker must not authorize
launch. The core only treats ENOENT as absence. Replace this permissive predicate
with checked inspection/enumeration whose unsuccessful operation refuses, and
review that exact delta before approving the helper. Root has accepted the
finding and is retaining the old source pin. The existing five tests do not
establish injected marker-I/O-error behavior.

The remainder of the bounded source inspection supports the intended mechanism:
private owned directory/holder metadata and IDs, CLOEXEC source descriptors,
nonblocking source SH, validated distinct ASCII basenames and1..4096-byte state
bound. Reopened descriptions are independently SH-locked and compared with
prepared IDs before the current bounded state hash. Fixed absolute tools,
integer/hex interpolation, quoted names, strict pipes and a one-second preflight
timeout avoid a caller-supplied privileged command seam. The timeout covers the
gate only. The launcher retains the same reopened holder description after the
flock subprocess exits; only the directory is closed, with no explicit unlock.
Prepared source custody must remain live throughout the eventual launch call.

Read actual first-linux.txt/exit: **5 serial PASS** (four behaviors plus one
fake-target helper), zero failed/ignored/filtered. The real exec fixture proves
EX contention after dropping parent lease and successful EX after target exit.
Negative cases preserve no-target execution for changed hash/length, existing or
dangling markers, symlink state and wrong mode. RAII owns/reaps the target and
joins the output reader with bounded event watchdogs. The two missing-module
compiler errors are an absence RED, not execution of these behaviors.

My preliminary no-blocker message preceded the independent marker-error finding
and is superseded by this HOLD. Next review should be only the checked-marker
delta and its exact receipts. Missing-tool/timeout, delayed retirement, disk FD
coexistence, root/sudo profiles, driver gating/idle-pool exclusion, actual VMM
inventory/EX-contention and native positive readmission remain integration or
qualification gates; the fake exec test is not their evidence. No larger native,
WAN, scale, performance or energy claim follows.

## Corrected helper delta

**PASS — scoped CODE/RUST helper source ataf9662b.** Reviewed exact commit
af9662b24732390276c79688710a2c937d583f95, product SHA-256
b4137f464212c6c5e49124cb7a741ccbc951dd6ce25d0359e80719dfa1940ec5;
tests remain ca45602497c59d4060c72e3d1f1867ec5a2bccb7de81a2726546b538bc5a29d4.
This resolves the marker-error HOLD above without rewriting the old identity.

The fixed absolute find command follows only the held command-line directory
reference, scans depth one for the two validated exact basenames, and stops
on a matching marker. Successful command completion and empty output are both
required. Existing/dangling marker entries refuse without needing a successful
stat of that marker; failed enumeration refuses through its checked exit status.
Explicit exit1 guards also cover reopen, descriptor bounds/type/ID, SH, state
metadata, bounded hash pipeline, timeout result and directory close. A failed
gate cannot proceed merely because an outer shell suppresses errexit. The
original same-description SH retention and preflight-only timeout are preserved.

Read corrected-source.sha256 and corrected-linux.txt/exit0: the exact corrected
file passes the same five serial Linux entries, with zero failed/ignored/
filtered. This is fresh execution of the correction, not relabelled old output.
No actual EIO, missing-tool or timeout injection is proved by those tests; their
unexecuted status remains explicit. No additional blocker was found in this
delta. Driver/daemon hooks and actual privileged Firecracker retention remain
unimplemented/unqualified by this helper review. No new tests, builds, edits
outside this review or broader audit were performed by this reviewer.
