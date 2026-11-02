# SC-MVM-004 registered Combined integration campaign

This campaign carries the current Combined gateway core through the existing
experimental canonical mailbox binding, first one native guest to a host endpoint,
then two native guests with a keyless host relay. It is a local macOS/HVF/vsock
integration comparison, not a new Internet WAN result. No codec or rate change is
attributed to MVM itself. Port 5257 remains experimental; FlowMux 5253 is unchanged.

## Fixed dose and fair comparison

- 36 sequential sessions: stages `one`, `two`; nominal sizes 1,048,576 and
  16,777,216 bytes; trials 0, 1, 2; arms `whole-mixed`, `combined`, `combined-cpu`.
- Actual runtime-derived payload lengths are **1,047,202** and **14,851,384** bytes.
  Requested size is a fixture tier selector, not the payload byte count.
- Eight payloads per arm: generated, residual, repeat, unrelated, opaque-edited,
  opaque-edited-repeat, schema-fit, schema-drift. All three arms must have identical
  object identities and lengths within each stage/tier/trial/kind. Runtime width
  4096, steps 289 or 4096 and trial payload parameter are fixed by the source.
- Combined arms additionally transmit checkpoint and schema. Their two setup
  publications, setup CPU, and handshake remain in paid session totals. Whole
  transfers the same eight payloads using prepared whole-object zstd/raw fallback.
  It does not receive unused checkpoint/schema setup.
- One persistent receiver per session. Fresh stores between arms/trials.
  Exact reopened reads follow every complete session. Two-guest sessions then
  boot a separate credential-free read-only verification guest on the same disk.
- Arm order rotates by trial: Whole/Combined/CPU, Combined/CPU/Whole,
  CPU/Whole/Combined. Complete all one-guest sessions before two-guest sessions.
- Every arm uses the explicit **experimental64** profile: 64 MiB/s allowance and
  8 MiB burst. The production/default 4 MiB/s profile is unchanged. There is no
  cross-rate speedup claim and no tuning after observing the measurements.

## Identity, launch and retention

The host executable, guest executable and kernel SHA-256 hashes plus the exact
committed SpiralCore source are registered before launch. The runner rejects
dirty/untracked implementation or script files, changed executable hashes,
existing output directories, invalid registrations and a free-space reserve below
4 GiB. It writes the full schedule before starting. No credentials, key material
or credential-bearing bootstrap/initramfs are captured.

Registration is JSON with the following exact fields (fill hashes from final
reviewed, built and signed artifacts; placeholders are deliberately invalid):

```json
{
  "format": "spiralcore-mvm-combined-registration-v1",
  "source_commit": "40 hexadecimal characters",
  "host_sha256": "64 hexadecimal characters",
  "guest_sha256": "64 hexadecimal characters",
  "kernel_sha256": "64 hexadecimal characters",
  "trials": 3,
  "stages": ["one", "two"],
  "sizes": [1048576, 16777216],
  "arms": ["whole-mixed", "combined", "combined-cpu"],
  "bandwidth_profile": "experimental64"
}
```

```sh
bash crates/spiralcore-mvm-live/scripts/run-combined.sh \
  HOST_RUNNER KERNEL GUEST_ELF NEW_OUTPUT_DIR REGISTRATION_JSON
bash crates/spiralcore-mvm-live/scripts/analyze-combined.sh \
  NEW_OUTPUT_DIR > NEW_ANALYSIS_JSON
```

Each successful session's received payload cache/ext4 disk is disposable only
after exact readback, reboot verification where applicable, and independent
receipt validation. Cleanup retains a receipt with the owned path and allocated
size; all JSON, stdout/stderr, identities and failure receipts are preserved.
Failures stop the campaign and retain their data; they are not silently retried
or replaced. Any subsequent diagnostic run has a separate name and status.
At each session boundary retain host load averages and compiler/witness process
counts without command arguments or environment. Shared-host activity is recorded
as a limitation; the runner does not pause, kill or modify another lab's work.
Compact public receipts belong in Git. Large build binaries stay in the existing
private artifact store after hash-verified custody; do not add a new local build
cache or persistent VM image for this campaign.

## Measurement definitions and acceptance

Actual encrypted stream bytes include both directions, envelope expansion,
framing, readiness and authenticated replies. Two-guest physical stream bytes
count both vsock hops; virtio control/credit headers are excluded. Planned
application bytes are retained as diagnostics, never substituted for actual
stream counters because an early publication can skip prepared frames.

Operation wall time covers encoding through authenticated publication response.
Exchange wall time is nested within that total and includes crypto, receiver
processing and waiting; it is not isolated socket-send time. Session sender wall
time adds fixture/codec setup, handshake and all operations but excludes JSON
serialization, fixture ownership copies and independent readback. Full lifecycle
wall time separately includes guest boot and recovery verification.

Sender process CPU and receiver processing CPU are reported separately and as a
clearly labeled partial endpoint sum. Receiver framed receive/allocation and idle
wait are outside processing CPU. Guest Linux CPU, host endpoint CPU, worker/VMM
CPU, relay/orchestrator CPU and recovery worker CPU remain separate domains.
Never add nested guest and worker CPU or infer physical energy from these values.

Require exact reconstruction/reopen for all 336 publications (288 payloads plus
48 Combined setup publications), 168 separate two-guest reboot verifications,
all 96 matched payload-identity groups, and directional counter agreement.
Generated/residual/repeat mechanisms and unrelated/schema fallback must remain
observable. Report policy selection details, any loss in bytes/CPU/time, and all
failed attempts. Socket-level interruption/reconnect tests are separate evidence
from actual VM reboot readback; neither proves a production durable mailbox inbox.

The analyzer reports individual rows, arm medians and medians of trial-paired
candidate/baseline ratios. The latter are not ratios of independent medians.
Comparisons are Combined/Whole, CPU-conscious/Whole, CPU-conscious/Combined, per
operation and paid whole session. Three trials are descriptive; no p95,
statistical-significance, energy, production-mailbox or real-WAN acceptance claim.
