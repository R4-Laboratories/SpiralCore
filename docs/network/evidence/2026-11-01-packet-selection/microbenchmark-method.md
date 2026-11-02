# Packet CPU microbenchmark

The production baseline is `4bce7efa218bfd426116107af8c907254009ac1a`.
`baseline-udp-sha256.txt` binds the source files, benchmark source snapshot and
frozen executable. This is a native optimized Rust measurement on a shared
Apple Silicon macOS host, not a network, authentication or durable-commit test.
Other applications/labs remain running; the process snapshot is retained and
their work was not interrupted. Owned Cargo jobs were stopped before timing.

The benchmark uses 512 deterministic packets per block, IPv4 and IPv6,
64/256/1280-byte total IP packets, datagram capacities 700/1336, and five
workloads: repeated, new patterns, opaque synthetic bytes, alternating patterns
and opaque bytes, and an opaque-to-compressible transition designed to miss an
eight-byte repeated-word guard. The transition's compressible payload contains
repeated seven-byte strings with a distinct eighth byte per block. All packets
have UDP headers, declared lengths and checksums. Each mode receives identical
bytes in identical sequence; their fixture fingerprints are recorded.

One complete unreported warmup trial precedes three measured trials. Mode
order rotates between trials. Raw and legacy forced zstd are measured through
the existing production packet API, including digest, framing, allocation and
decoder verification. Setup and the first packet's encode/decode are recorded
separately; subsequent packet blocks reuse both contexts. Batch clocks amortize
clock-call overhead. Fixture generation, final exact comparisons, byte counting
and JSON serialization are outside timed codec blocks. Every returned packet is
compared byte-for-byte with its original before recording success.

`compression_profile_only` rows isolate level3/window25/LDM versus
level1/window11/no-LDM using the same prepared CCtx and fresh-output-allocation
wrapper. These rows include the first compression but exclude context
construction, packet hashing/framing and decoding CPU. Both profile outputs are
verified by the existing production decoder and exact byte comparison. Their
times must not be presented as full packet or network times. Both configurations
retain checksums, content size, no dictionary ID and zero zstd worker threads.

Reproduction command for the retained baseline executable:

```sh
packet-profile-baseline-udp --label baseline-udp-4bce7ef --trials 3 --packets 512
```

`baseline-udp.jsonl` contains 360 production packet rows and 180 compression-only
rows. All 540 measured rows verified exact output. The whole command completed
in 1.83 seconds with 7,880,704 bytes maximum resident memory according to macOS
`time -l`. Timings are small local diagnostic measurements, not a claim about
WAN latency tails or energy.

Earlier `baseline.jsonl` used experimental/no-next-header IP vectors. It remains
a valid codec-profile diagnostic but is superseded by the UDP vectors for the
Adaptive comparison, because the selector's agreed flow eligibility is TCP/UDP.
The behavior-changing fixture test failed first (`udp-vector-red.txt`), then both
harness tests passed (`udp-vector-green.txt`). The initial harness API RED and
GREEN are also retained. No production selector source changed before the
revised baseline executable and source hashes were frozen.

Before any Adaptive measurement, tracing the agreed probe cadence revealed that
the original transition at index 256 could coincide with a mandatory reprobe,
hiding the guard's miss cost. The parent approved an additional negative
`guard_miss_shifted`, whose transition starts at index 259 for the default
512-packet block. This is registered before observing the selector and has no
historical before-run counterpart. The five original workloads remain bitwise
unchanged and retain their paired before/after comparison. The shifted negative
will compare modes within the new executable only; no before claim will be made
for that additional case. Original baseline evidence is preserved.
