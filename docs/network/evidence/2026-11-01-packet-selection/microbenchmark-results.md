# Packet selector microbenchmark results

All 540 baseline rows and all 1,080 after rows verified exact bytes. The 360
paired legacy Raw/Zstd rows have identical fixture fingerprints, body bytes and
framed bytes before and after. The additional shifted guard-miss case was
registered before observing Adaptive and has no historical-before counterpart.

The tuned packet zstd profile reduces compression work, but it sometimes emits
more bytes than the legacy profile. Adaptive avoids most unsuccessful compression
attempts on a stable opaque flow. It also demonstrably misses savings after an
adversarial content transition and exceeds the proposed 1% framed-byte penalty
gate. These are reasons to preserve the explicit modes and keep Adaptive opt-in.

The following are medians of three after-run trials for 512 IPv4 packets of
1280 bytes each, with maximum datagram size 1336. CPU is microseconds per packet.
Framed bytes include SpiralCore fragment headers, not QUIC/UDP/IP overhead.

| Workload | Mode | Framed bytes per 512 packets | Encode CPU | Decode CPU | Compression attempts |
|---|---|---:|---:|---:|---:|
| Repeated | Raw | 684,032 | 4.527 | 4.590 | 0 |
| Repeated | Legacy Zstd | 66,048 | 7.131 | 4.576 | 512 |
| Repeated | FastZstd | 67,072 | 5.365 | 4.570 | 512 |
| Repeated | Adaptive | 67,072 | 5.029 | 4.553 | 512 |
| Opaque | Raw | 684,032 | 4.631 | 4.678 | 0 |
| Opaque | Legacy Zstd | 684,032 | 10.494 | 4.639 | 512 |
| Opaque | FastZstd | 684,032 | 9.744 | 4.785 | 512 |
| Opaque | Adaptive | 684,032 | 6.752 | 4.865 | 64 |
| Shifted guard miss | FastZstd | 417,623 | 20.936 | 6.191 | 512 |
| Shifted guard miss | Adaptive | 422,888 | 13.813 | 7.350 | 281 |

The last workload's Adaptive penalty is **5,265 framed bytes, or 1.261%**. With
700-byte datagrams it is 5,545 bytes, or 1.283%, because the missed compression
also adds fragments. IPv6 shows the same limitation: 1.251% at cap 1336 and
1.274% at cap 700. Exact reconstruction still passes. The unshifted transition
coincides with a reprobe and does not show this penalty, illustrating why that
phase was insufficient as the sole negative case.

FastZstd adds 1,024 body bytes over 512 repeated packets compared with legacy
Zstd: two bytes per packet. For the new-pattern case it likewise adds two bytes
per packet. It retains all of the opaque bytes and reduces body bytes slightly
for some other patterns; it is not universally byte-optimal. The isolated
parameter diagnostic and complete body/framing totals are retained in
`profile-baseline-summary.json` and `microbenchmark-analysis.json`.

Timing interpretation is deliberately limited. This is a shared macOS host;
unchanged-control median after/before CPU ratios are 1.141 for Raw encoding,
1.159 for Raw decoding, 1.230 for legacy encoding and 1.137 for legacy decoding.
The after command recorded 16,668 involuntary context switches versus 102 for
the UDP baseline. Small differences, including Adaptive versus FastZstd on a
patterned flow where both make every compression attempt, are not a reliable
optimization claim. No new process was interrupted to manufacture a quiet
measurement environment. These rows are local diagnostics; constrained-network
measurements are a separate experiment.

Cold setup and first-packet costs, both monotonic wall and process CPU times,
per-case counters, all raw trial rows, executable/source identities and negative
test history remain available. The after command completed in 7.96 seconds,
with 8,273,920 bytes maximum RSS reported by macOS `time -l`.

Recompute the recorded arithmetic without editing raw receipts:

```sh
jq -n --slurpfile before baseline-udp.jsonl --slurpfile after after.jsonl \
  -f microbenchmark-analysis.jq > microbenchmark-analysis.json
```

This evidence does not measure transfer time, authentication cost, durability,
real WAN behavior, physical energy or production qualification.
