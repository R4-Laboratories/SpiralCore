# Shared object-v1 endpoint evidence

[Human-readable results](../../v1-endpoint-results-2026-11-02.md) describe the
native 24-session legacy/v1 comparison, including the small-workload regressions.

- [Measurement contract](measurement-contract.md), [registration](registration.json)
  and [schedule](schedule.json) pin both implementations, binary/kernel hashes,
  stages, sizes and trial order.
- [Execution receipt](execution-receipt.json) records Linux tests, the isolated
  adapter lockfile-resolution refusal/correction, restored source hashes and
  owned-resource cleanup.
- [Analysis](analysis.json) contains every session, operation, paired ratio and
  limitation. Each named session folder retains `result.json`, guest/host events,
  a checked summary, verified ephemeral-store cleanup and `receipts.sha256`.
- [Source-file hashes](source-files.sha256) cover the six archived crate trees;
  [executable/runner identities](identities.sha256) record the remote campaign.
  VM disks, caches, private MVM homes and credentials are excluded.

Reanalyze without running a guest:

```sh
jq -s -f crates/spiralcore-mvm-live/scripts/v1-comparison.jq \
  docs/network/evidence/2026-11-02-v1-endpoint/*/verified-summary.json
```

The analyzer requires 24 unique cells and 120 matched object/mode/planned-byte/
actual-frame-count groups. Planned application bytes can include frames skipped
after early Published and are not claimed as actual plaintext transport bytes.
Actual encrypted envelope/framing counts are recorded independently.

The three-trial local Linux/vsock campaign is not WAN or production MVM mailbox
qualification. The prior zstd comparison remains separate evidence. Compact raw
receipts are public synthetic measurement evidence; validator fixtures with
synthetic implementation labels never count as native transfers.
