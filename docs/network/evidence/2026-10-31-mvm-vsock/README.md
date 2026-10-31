# Actual MVM/vsock integration evidence

Start with the [result report](../../mvm-vsock-results-2026-10-31.md) and
[registration](registration.json). This is a local native HVF functional pilot,
not a new WAN or statistical performance qualification.

- `pilot/accepted/`: twelve complete raw session results and host journals;
  guest console, operations, clocks, worker CPU, shutdown and recovery receipts
  are embedded in each `result.json`.
- `pilot/operations.jsonl`: 100 derived operation rows, paired against receiver
  records and readback; all are exact, including 50 separate guest reboot checks.
- `pilot/common-workload-summary.json`: 84 matched rows (seven per session),
  excluding 16 Warm-only schema/runtime rows.
- `pilot/session-summary.json`: full session/lifecycle costs and separately
  labeled OS CPU accounting. No guest/worker double-counting.
- `pilot/identities*.sha256` and `qualification/pilot-source*.sha256`: actual
  executable, kernel, registration and source identities for the two recorded doses.
- `pilot/amendment-01.json`: the console-drain fix and narrowly repeated matched
  two-guest 16 MiB group. Earlier failed/superseded results remain alongside it.
- `qualification/`: observed RED/GREEN, builds, actual 4 KiB guest tests, focused
  MVM checks, independent reviews, coverage and source-linked performance diagnosis.
  Raw failures and command output are retained deliberately at the user's request.

The physical byte metric is the sum of actual encrypted stream writes/reads across
both directions and, for two guests, both vsock hops. Virtio control/credit headers
are excluded. `planned_application_batch_bytes` is a sender planning counter:
frames skipped after early Published may be included; it is not a wire-byte metric.

The receiver CPU metric excludes framed receive/allocation/copy. Guest Linux CPU,
native macOS process CPU and native vCPU-thread CPU use different accounting
mechanisms and are reported separately. No physical-energy claim follows.

Use `crates/spiralcore-mvm-live/scripts/operation-rows.jq` to derive operation rows
from each accepted `result.json`/`host-events.jsonl` pair. The independent
`qualification/measurement-review.json` reconciles 100 rows, 12 sessions, 50 reboot
readbacks, 24 summaries and byte-equal copies of all 24 retained raw files.

The MVM patch bundle has its own exact base/blob/file provenance and portable
preparation script. Kernel/tool receipts describe the isolated downloads. Raw
bootstrap secrets and credential-bearing initramfs were never persisted. Generated
public fixture stores/disks are disposable after custody checks; Git retains the
compact reproducible source and observations.
