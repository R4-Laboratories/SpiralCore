# Native Linux pilot infrastructure review

Reviewed preregistration and runner at commit `6257930`, plus unchanged Rust
host/orchestrator resource and lifecycle enforcement. Verdict: **APPROVE** the
seven fixed cells within the user's existing Wick test-host authorization.
This is source review, not evidence that a guest was launched or passed.

Independent check: `bash -n native-linux.sh` returned zero. The runner checks
exact host, guest, kernel and Firecracker hashes before use, verifies the guest
has no ELF interpreter, and creates a new private result directory. It refuses
an existing `/usr/local/bin/firecracker` path; cleanup removes only the temporary
link whose target, inode and root owner still match. No global process kill,
network reconfiguration, guest NIC or modification of other labs is present.

The Rust `linux_host` specification fixes one vCPU and 1024 MiB per guest and a
single 256 MiB writable disk. Its worker watchdog is 120 seconds; the orchestrator
worker completion bound is 150 seconds. The two active transfer workers finish
before the keyless verification guest starts. This permits at most two owned
guests concurrently, not seven simultaneous guests or a fleet-capacity claim.

The shell runs one cell at a time, records stdout/stderr and exit status, and
stops at the first failure. Any remaining cells are NOT_RUN. Existing canonical
frame/byte agreement and exact readback checks are required by the Rust
orchestrator; the additional jq check requires successful sender and receiver
results. Quotas and the 2048-Call witness cap remain unchanged. The experimental64
profile is explicitly recorded as not rate-enforced by the Linux backend.

Uncontrolled existing host work must be recorded; do not run coverage/builds or
other owned benchmarks during these cells. Retain failures and verify archival
before removing only this campaign's owned payload files. Native role results
remain distinct from shared-process aggregate-accounting, Mac, Windows and WAN
acceptance. Current Mac resource observations occurred around/after its failure
and do not establish a unique scheduling cause.
