# Independent SC-MVM-004 native result review

Reviewed the 36 completed native HVF session receipts in
`local/mvm-combined-20261102/campaign` against registered source
`9ceaeef5070c87fd17b8519a1e8630b7f7888fd6`. No blocking discrepancy was found in
the retained accounting or the bounded result claims below.

## Reproduction and custody

- Verified all eight registered executable/kernel/registration/runner/analyzer
  identity entries; see `review-identity-check.txt`.
- Verified all 36 session receipt manifests, covering 288 files; see
  `review-receipt-check.txt`. All checks succeeded.
- Re-ran `analyze-combined.sh` against the raw campaign receipts and compared its
  output byte-for-byte with the retained `analysis.json`. Both have SHA-256
  `f2ddf26bc322fa085581b0885323c6e2dfdd39e8a2fea5eb973a7a06d0196e9d`.
  The duplicate derived JSON was removed after comparison; raw evidence remains.
- Independently recomputed all eight stage/size/Combined-policy versus Whole
  paid-session comparisons directly from raw `result.json` fields rather than
  using the analyzer's extracted rows. All byte, sender CPU, partial endpoint
  CPU and elapsed paired medians agree exactly. The independent values are
  retained in `review-independent-ratios.json`.

The validated count is 336 publications/readbacks, comprising 288 common payload
operations and 48 Combined setup publications. All 168 two-guest publications
also have exact read-only verification after a separate credential-free guest
boot. All 96 cross-arm payload-identity groups agree. This is exactness of these
fixtures and observed runs, not a proof over every possible input or fault.

## Mechanisms and byte parity

The raw sender receipts establish the intended mechanisms in every measured
Combined session: generated objects select `RuntimeV1`; residual and opaque
edits select `SparseV1`; repeats report the exact-repeat probe; unrelated content
uses raw fallback; fit and drift controls select `SchemaZstdV1`. The exhaustive
and CPU-conscious policy names and skip counts remain distinguishable. See
`review-mechanisms.json`.

Across all 120 matched Combined/CPU-conscious operations, selected mode and
planned application request/response byte counts are identical; see
`review-application-parity.json`. Actual encrypted envelope lengths vary between
independently provisioned sessions. Those observed wire differences are retained
and are not presented as a change in codec output size. Planned application
bytes remain a diagnostic and are not substituted for actual stream counters.

## Two-guest large-session result

Each common payload is **14,851,384 bytes**; each session sends eight payloads,
with Combined additionally paying checkpoint/schema publication and preparation.
The table contains medians of the three arm observations.

| Arm | Logical encrypted stream bytes | Sender CPU | Partial endpoint CPU | Sender setup + operation elapsed |
|---|---:|---:|---:|---:|
| Combined exhaustive | 56,538,203 | 1.442327 s | 2.379372 s | 2.620826 s |
| Combined CPU-conscious | 56,535,989 | 1.422216 s | 2.351801 s | 2.567689 s |
| Prepared whole-object zstd | 97,329,036 | 1.924770 s | 3.200029 s | 3.599495 s |

The independently verified **median paired ratios** give exhaustive Combined
41.9194% fewer logical bytes, 25.3659% lower sender CPU, 26.2476% lower partial
endpoint CPU and 29.2083% lower elapsed time than Whole. CPU-conscious Combined
gives 41.9123%, 26.0613%, 25.6646% and 27.3367% reductions respectively. These
percentages are not ratios of independently chosen table medians.

Two-guest physical stream accounting doubles each logical count because both
vsock hops are counted. It excludes virtio control/credit headers. The rate is
the same explicit 64 MiB/s experimental allowance in every arm; this is not a
comparison against the default 4 MiB/s profile.

## Retained losses and practical limits

- Large two-guest exhaustive schema-fit and schema-drift operations save only
  1.1061% and 0.6942% of actual bytes versus Whole while costing 34.1641% and
  38.5017% more sender CPU and 28.1838% and 24.2433% more elapsed time. The
  byte-minimizing selector is not a universal latency-minimizing selector.
- Large two-guest unrelated content costs exhaustive Combined 5.0143% more
  sender CPU than Whole. CPU-conscious reduces sender CPU 7.6745% relative to
  exhaustive on this operation; its comparison with Whole is approximately
  flat (-0.2375%). These are descriptive measurements, not a hard CPU budget.
- CPU-conscious is not uniformly better than exhaustive. In the small two-guest
  trial zero it takes 24.4220% more paid-session elapsed time than Whole and
  72.9208% more than exhaustive. That observation is retained. Small two-guest
  opaque-repeat paired medians also show 18.4149% more sender CPU and 10.7542%
  more elapsed time than exhaustive. No causal explanation is established.
- Common deterministic fixture generation, including CA replay, is included in
  setup for all arms; Combined additionally pays prepared prediction replay.
  Thus these session CPU values include workload construction. Operation CPU
  excludes fixture ownership copies and independent expected-identity hashing.
- Partial endpoint CPU excludes receiver framed receive/idle wait. Guest Linux
  counters must not be added to the enclosing host worker/VMM process counters.
  Sender elapsed includes preparation and transfer acknowledgement, while full
  VM lifecycle and independent readback are separately recorded. Exchange time
  includes remote processing and waiting and is not send-only time.
- Three paired trials are descriptive. The campaign is on one shared macOS host
  with actual Linux guests and vsock, not an Internet WAN, production mailbox,
  physical energy, FIPS or CUI qualification. Socket interruption/reconnect
  evidence is separate from native guest restart/readback and does not establish
  a durable mailbox replay inbox.

The supported conclusion is that the current Combined core crosses the existing
real MVM guest/mailbox boundaries with exact recovery and materially lower
paid-session bytes, CPU and elapsed time for the registered mixed workload.
Per-operation losses remain part of the result, and CPU-conscious remains opt-in.
