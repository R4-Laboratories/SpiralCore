# MVM overhead measurement packet

This packet measures actual two-guest HVF/vsock transfers. The registered sample
is three matched trials at 1 MiB and 16 MiB for Whole and Warm, with three
execution conditions: preserved implementation at 4 MiB/s, updated implementation
at 4 MiB/s, and updated implementation at the explicit 64 MiB/s experimental
mailbox profile. All profiles retain the same 8 MiB burst and other controls.

`registration.json` records the original plan. `amendment-01.json`, written before
any guest launch, rotates the three conditions within each size/trial/arm and
allows unrelated shared-host builds with recorded load. The initial preflight
refusal is retained under `qualification/preflight-build-overlap-01`; it launched
no guest. Owned SpiralCore builds finish before measurement. Endpoint compiler
names/counts and host load are sampled before and after each session. These are
descriptive results under observed host load, not clean-host qualification.

`run-session.sh PHASE SIZE TRIAL ARM` uses frozen binaries, creates a fresh
receiver disk, retains public receipts and requires exact object reconstruction,
stopped-store reopen and keyless read-only reboot verification. Only after copied
receipt hashes verify does it remove that session's disposable receiver disk.
It does not delete failed state or automatically retry a failed session.

`collect.sh` extracts public result receipts using `extract.jq`, verifies exact
fixture IDs and lengths across conditions, and writes `sessions.jsonl`,
`operations.jsonl`, `completion.json`, `fixture-pairing.txt`, and `summary.json`.
`analyze.jq` computes descriptive medians and medians of trial-paired ratios.
No p95, confidence interval, WAN, energy, FIPS or production-readiness claim is
supported by this sample.

Operation time starts before preparation and ends at authenticated publication.
Exchange time is its nested request/reply interval, including authentication,
receiver work and waiting; it is not isolated socket-send time. Sender Linux CPU,
receiver Linux processing CPU, native macOS process CPU and native vCPU-thread
accounting remain separate. Native process totals include the orchestrator and
each worker once, including the read-only recovery worker; guest CPU is never
added to its enclosing native worker.

Compare the common seven object operations across arms. Warm additionally sends
schema and three shared-runtime objects; its complete lifecycle metrics include
those extra operations. Logical bytes count both directions of encrypted canonical
MVM envelopes and framing; two-guest physical stream bytes are twice the logical
bytes and exclude virtio control/credit headers. The existing Whole path already
avoids transferring the body of an identical cached object.

Source fixes, RED/GREEN checks, isolated preparation diagnostics and VM measurements
have separate receipts. The isolated diagnostic uses a native debug build and a
test acknowledgment and is not substituted for authenticated guest results.
