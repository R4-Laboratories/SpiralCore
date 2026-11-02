# Independent constrained-network interpretation

Run `41364240626`, attempt 1, completed all **32 sessions, 128 application
transfers and 256 matching exact endpoint receipts**. The independently prepared
`network-analysis.jq` ran successfully against the extracted campaign without
analysis changes. It reconciled lengths, hashes, persistent TCP endpoints,
capture completeness, gateway counters and calibration. Total logical input was
1,233,256,448 bytes: 24 primary sessions each sent three 16 MiB TCP objects and a
4 KiB UDP smoke; eight IPv6 smoke sessions used three 1 MiB objects plus 4 KiB.

The shaped profile measured mean RTTs of **80.256–80.306 ms** and directional
calibration receiver goodput of **9.607–9.797 Mbit/s**. This was an isolated Linux
network on one GitHub runner, not an Internet WAN or two physical machines.

These are the three-trial medians for the shaped primary profile. Captured IP
bytes cover the entire mixed session in both directions. Gateway CPU sums the
two gateway processes. Object time is the client interval through its verified
receipt, including work on the transfer's critical path; first TCP includes
connection establishment. Gateway setup is recorded separately.

| Mode | Session captured IP bytes | Gateway CPU seconds | First 16 MiB pattern seconds | 16 MiB opaque seconds | Final 16 MiB pattern seconds |
|---|---:|---:|---:|---:|---:|
| Raw | 62,681,599 | 4.947 | 15.825 | 15.562 | 15.567 |
| Legacy Zstd | 28,472,420 | 3.294 | 2.602 | 15.558 | 2.376 |
| FastZstd | 28,034,434 | 3.137 | 2.569 | 15.557 | 2.529 |
| Adaptive | 28,526,499 | 3.087 | 2.601 | 15.556 | 2.178 |

Using **median trial-paired ratios**, FastZstd versus Raw reduced captured IP
bytes by **55.28%** and gateway process CPU by **36.42%** over the mixed session.
Its patterned objects took about **6.16 times less elapsed time**, while the
opaque object's elapsed time was effectively unchanged. These byte and CPU
totals cannot be attributed separately to each object from this mixed capture.

Against legacy Zstd, FastZstd used **4.78% less gateway CPU** and **1.58% fewer
captured IP bytes** in the shaped run, but the final patterned object's paired
time was **7.08% higher**. On the unshaped LAN, FastZstd's opaque and final-pattern
intervals were **17.46% and 25.64% slower than Raw**. Fewer transferred bytes
therefore do not imply a universal time improvement on an unconstrained link.

Adaptive versus equally tuned FastZstd used **1.83% more captured IP bytes** in
the shaped profile while reducing gateway CPU by only **1.03%**. Its final
patterned object's paired time was 6.44% lower, its first patterned object's time
1.24% higher, and opaque timing effectively unchanged. The actual flow used the
positive guard once after opaque data in every Adaptive session; its recorded
skip/reprobe counters confirm the selector was exercised. Adaptive remains
opt-in: this campaign does not show a consistent byte advantage over forced
FastZstd, and the microbenchmark separately demonstrated a guard-miss penalty.

There were **3,027 finite-queue congestion drops** across all sessions, all
retained as measured costs. Captures had no kernel loss; gateway expiry/replay
counts were zero, and exact application recovery passed. Drops varied by arm
and trial, so small transport-byte or timing differences should not be assigned
solely to the codec. Three trials support descriptive comparisons, not tail
latency, confidence intervals or a production performance guarantee.

`network-analysis.json` retains every session, each raw trial-paired ratio,
per-object timing medians, all accounting layers and diagnostics. Encode/decode
CPU is nested inside gateway process CPU and is never added to it. Application
CPU represents measured intervals, not complete process lifetime. CPU is not
subtracted from wall time to invent a pure send-time metric. IPv6 remains
functional smoke coverage rather than a separate performance qualification.

Execution identity from the artifact: source
`9dd283ca366d6075bd6b423c3a42aa928e222ce2`, workflow merge SHA
`b419ebfdf559171a5408b98a8474853cf7be3204`. Full source/binary and artifact-custody
verification remains in the separate campaign custody records.
