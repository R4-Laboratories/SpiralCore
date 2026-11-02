# Packet selection and constrained-network results

Implementation source: `9dd283ca366d6075bd6b423c3a42aa928e222ce2`,
[PR #25](https://github.com/R4-Laboratories/SpiralCore/pull/25).
This milestone adds opt-in FastZstd and Adaptive packet modes. The existing
Raw/Zstd defaults, exact decoder and object/MVM path remain unchanged.

## Local packet evidence

The [microbenchmark report](evidence/2026-11-01-packet-selection/microbenchmark-results.md)
retains 540 baseline and 1,080 after rows with exact recovery, and independently
reconciles all 360 paired legacy controls to identical fixture/body/framed bytes.
For 512 opaque IPv4 packets of 1,280 bytes each, Adaptive made 64 compression
attempts instead of 512 and preserved FastZstd's framed bytes. In the same after
run, median encode CPU was 6.752 microseconds per packet versus 9.744 for FastZstd
and 10.494 for legacy Zstd. These are local diagnostics: unchanged control timings
drifted 14–23% between runs on the shared host.

The faster profile adds two bytes per packet on the repeated and new-pattern
fixtures compared with legacy Zstd. A deliberately shifted opaque-to-compressible
transition evades the Adaptive repetition guard, increasing framed bytes by
1.25–1.28% relative to FastZstd and failing the proposed 1% gate. Adaptive remains
experimental and opt-in. These limitations are retained, not tuned away after
observing the results.

## Network execution

Source-pinned run:
[41364240626](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41364240626).
**PASS:** all 32 sessions, 128 logical transfers and 256 endpoint receipts
reconstructed exactly. The [independent analysis](evidence/2026-11-01-packet-selection/network-analysis.json)
checks fixtures, both endpoint receipts, persistent socket identities, capture
completeness and measured calibration. The separate
[compatibility regression](evidence/2026-11-01-packet-selection/regressionverification.json)
passed 27 Linux tests and 48 additional transfers on the same source.

The [registered contract](evidence/2026-11-01-packet-selection/network-contract.md)
defines 24 primary sessions with 16 MiB TCP objects and eight IPv6 smoke sessions
with 1 MiB objects. Each mode/profile/trial uses one persistent TCP connection
for pattern → opaque → pattern, followed by a 4 KiB UDP echo smoke. This is
32 sessions, 128 application transfers and 1,176.125 MiB of application input.

The same authenticated TUN/QUIC path runs Raw, legacy Zstd, FastZstd and Adaptive
on an unshaped local network and an isolated router applying 40 ms delay and
10 Mbit/s independently in each direction. Real Linux TUN sockets and network
traffic are used; this is **shaped local execution, not a two-host Internet WAN**.
Three primary trials are descriptive, not p95 or confidence qualification.

Measured shaped RTT means were **80.256–80.306 ms**; directional receiver
calibration rates were **9.607–9.797 Mbit/s**. The campaign retained **3,027
finite-queue congestion drops**, with zero capture drops, reassembly expiry,
capacity/replay drops or UDP smoke retries. TCP recovered to exact application
completion. These losses are part of the result, not discarded trials.

Per-object client completion includes exact verification and the returned
receipt; the first also includes TCP connection establishment. Fixture loading
and final report serialization are excluded. Reports are emitted only after the
three-object sequence, avoiding reporting work between timed exchanges. Gateway
CPU, mixed-session captured bytes, calibration, setup and full harness wall are
separate measurements. Codec CPU is already included in gateway process CPU and
must not be added again. Subtracting CPU from elapsed time does not produce an
isolated send-time measurement.

## Measured constrained-link results

The following are medians of three primary IPv4 trials on the calibrated
80 ms RTT / 10 Mbit/s bidirectional shaped link. Each TCP object is **16 MiB**.
Object completion already includes codec work along the path and exact checking;
CPU must not be added to wall time again.

| Mode | Pattern before | Opaque | Pattern after, same TCP connection | Three-object client time |
|---|---:|---:|---:|---:|
| Raw | 15.825 s | 15.562 s | 15.567 s | 46.953 s |
| Legacy Zstd | 2.602 s | 15.558 s | 2.376 s | 20.540 s |
| FastZstd | 2.569 s | 15.557 s | 2.529 s | 20.653 s |
| Adaptive | 2.601 s | 15.556 s | 2.178 s | 20.348 s |

The last column is the median of each trial's three-object sum, not the sum of
the three separate medians. Pattern-after times are descriptively different;
three trials do not establish that Adaptive's lower time there is repeatable.

| Mode | Captured IP bytes, whole mixed session | Gateway process CPU, both ends | Nested codec CPU, both ends |
|---|---:|---:|---:|
| Raw | 62,681,599 | 4.947 s | 0.322 s |
| Legacy Zstd | 28,472,420 | 3.294 s | 0.739 s |
| FastZstd | 28,034,434 | 3.137 s | 0.626 s |
| Adaptive | 28,526,499 | 3.087 s | 0.582 s |

Captured bytes include both directions, authentication, QUIC/UDP/IP overhead,
TCP acknowledgments and recovery, the three TCP objects, UDP smoke, and closure.
They cannot be assigned to individual objects from this mixed capture. Codec CPU
counts are distinct from application CPU. The capture measures one endpoint's
underlay boundary, not the sum of transmissions on every router link. Codec CPU
is a subset of process CPU. The nested column is the median of each trial's
combined encode/decode total, independently recomputed in
[parent-independent-summary.json](evidence/2026-11-01-packet-selection/parent-independent-summary.json).

Using median **trial-paired ratios**, FastZstd used **55.28% fewer captured IP
bytes and 36.42% less gateway process CPU than Raw**. Total gateway CPU fell
despite additional codec CPU in this workload. Against legacy Zstd,
FastZstd used **4.78% less gateway CPU** and observed **1.58% fewer captured IP
bytes**, but was not uniformly faster: pattern-after completion was 7.08% slower
by paired median. Actual capture differences include packetization and transport
behavior; they are not proof of universally smaller codec output.

Adaptive used **1.83% more captured IP bytes than FastZstd** for only **1.03% less
gateway CPU** by paired median. That byte tradeoff, together with the adverse
codec case, does not support promoting Adaptive for the project's byte-first
objective. Opaque transfer time remains essentially unchanged across all modes;
the compressor cannot remove arbitrary ciphertext information.

## Fast local link and disposition

On the unshaped local link, the median three-object client times were Raw
0.773 s, legacy Zstd 0.956 s, FastZstd 0.904 s and Adaptive 0.874 s. FastZstd still
reduced mixed-session captured bytes (26,062,287 versus Raw's 59,443,028), but
was slower than Raw on this fast link. CPU/byte benefits do not imply a speed
benefit when the unconstrained network is already faster than codec processing.

Retain `fast-zstd` as the explicit lower-CPU packet-compression option for further
deployment trials; keep `adaptive` experimental and all defaults unchanged.
These modes use conventional zstd with exact packet framing. This milestone does
not attribute their gain to ternary DAGs, cellular automata or warm-object reuse;
those remain available on the separate object/MVM path.

The next deployment gate is the same frozen comparison across two actual Linux
hosts over a reachable authenticated UDP path, including loss/jitter and recovery.
The current result establishes the constrained-link gain under measured shaping,
not an Internet-WAN, energy, FIPS/CUI or production qualification.

## Retained evidence and storage

The implementation is merged through PR #25. This report, raw public JSON
receipts, test records, independent calculations and negative results are kept
in GitHub. Complete original GitHub artifact ZIPs, including captures and frozen
executables, are stored in the existing private
`caseyallard/spiralcore-experiment-artifacts` Hugging Face dataset:

- Main campaign: `2026-11-01/packet-selection-41364240626.zip`, 53,915,951 bytes,
  revision `4ee46e37e094cc0cdc34e4f3d256c5121bee8986`, SHA-256
  `7c56b4a6abef97122513e74811de7d66a4954180fb8bbbf6bd1fb1619bcfb5db`.
- Compatibility regression: `2026-11-01/regression-41364240628.zip`, 21,917,801
  bytes, revision `8e56a31334310b98e54d3538586c0593d058b7af`, SHA-256
  `8458c72814a4a17fbfc964e522b62ad778657bae8883331624bd0c051b9cf7ae`.

Both archives were freshly downloaded at their pinned revisions and matched the
original hashes before local large-copy cleanup. The archive receipts record
custody independently of GitHub Actions' temporary retention. Reproduction and
accounting commands are in the accompanying analysis-method files.
