# Independent network arithmetic

`network-analysis.jq` is prepared before inspecting the campaign outcome. It
consumes the compact public JSON receipts from the 32 registered sessions. No
performance values or favorable-case filters are embedded in the analysis.

Run only after all session artifacts are available:

```sh
jq -n '[inputs | {path: input_filename, data: .}]' \
  "$campaign"/primary-*/*.json "$campaign"/smoke-*/*.json |
  jq -f network-analysis.jq > network-analysis.json
```

The analysis refuses missing/duplicate required receipts, wrong session counts,
incorrect application lengths, mismatched endpoint hashes, mismatched fixtures
across modes/profiles, incomplete captures, failed gateways, inconsistent codec
counters or invalid calibration. It checks the persistent TCP socket endpoint
identity and receipt-deferral flags. It independently recomputes retained ping
means and checks both directional iperf rates. Queue drops, expiry and replay
counts remain recorded; finite-queue congestion is not silently discarded.

Primary IPv4 results use three descriptive trials: medians per mode/profile and
medians of the **trial-paired ratios**, with each raw paired ratio retained. Raw,
legacy Zstd and equally tuned forced FastZstd are separate denominators. IPv6
smoke sessions remain available individually for exactness coverage and are
excluded from the primary performance summaries.

Wire bytes are mixed-session capture totals, one interface in both directions,
including authentication, QUIC acknowledgments/retransmissions and close/drain.
IP bytes, UDP payload bytes and Ethernet bytes are separate accounting layers.
They are never assigned to individual objects or added together. Gateway process
CPU is summed across the two gateway processes; nested encode/decode CPU is
displayed separately and is never added to process CPU. Application CPU covers
the recorded application intervals, not their complete process lifetimes.

Application timing is per object through its exactness receipt. Client and
server intervals remain separate; first TCP client timing includes connection
establishment. The script does not subtract CPU from wall time, invent a pure
send-time metric, or combine overlapping endpoint wall intervals. Harness time
includes topology setup/calibration/teardown and is retained separately from the
transfer intervals.

This is a shaped local Linux test on one runner. It does not establish an actual
Internet WAN, two physical hosts, tail latency, statistical confidence, physical
energy savings, or production qualification. Source/binary custody, capture
artifact integrity, final route checks and workflow execution identity still
require the separate retained custody receipts and reviewer verification.
