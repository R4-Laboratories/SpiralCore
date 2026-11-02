# Registered object-campaign arithmetic

The Rust helper `tools/gateway-network/analyze-runtime-objects.rs` is prepared
before campaign results are inspected. It uses the gateway's existing JSON
dependency in a standalone analysis package, so post-run arithmetic does not
change the measured gateway package or binaries. Run after the
complete public campaign directory and terminal exit receipt are available:

```sh
cargo +1.97.1 run --locked --release \
  --manifest-path tools/gateway-network/runtime-analysis/Cargo.toml \
  --bin analyze_runtime_objects -- /absolute/path/to/campaign > analysis.json
```

The helper refuses incomplete dose: 24 distinct sessions, four explicit codec arms,
two link settings and trials 0–2; 312 successful operations and 288 delivered
objects with the registered lengths and total requested bytes. It binds every
operation to the trial fixture manifest, its receiver's reopened object identity
and exact-readback assertion, and its `transfer_id`-matched receiver batches.
The two repeats retain their original fixture IDs. Checkpoint/schema operations
are separate paid setup; the Warm generated seed is the first delivered object,
not an additional delivery.

Validation includes complete singleton session metadata, client/server namespace
agreement, matching logical channel totals in opposite directions, monotonic
per-operation counter snapshots, nonnegative integer timings, successful
publication paths, receiver batch attribution, active RuntimeV1 execution on
fresh generated/residual targets and SparseV1 execution on Warm opaque edits.
An offered mode that reaches Published at Begin is a cache hit, even when the
sender already uploaded a batch. Candidate costs retain the original HTTP-body
accounting label and are never substituted for measured channel traffic.

Per-operation request/response bytes are the differences in encoded-batch plus
four-byte framing counters. They include the actual object messages in both
directions, but exclude QUIC encryption, acknowledgments, retransmissions and
connection setup/close. Receiver handler CPU and wall are joined by transfer ID
and summed across that operation's batches; they exclude stream wait/framing.
Sender operation CPU includes file reads, identity hashing, codec selection and
publication processing. Sender and receiver stage CPU can be summed as endpoint
stage work, but that sum is not added to whole-process CPU. Client operation wall
is the measured complete interval, not CPU-subtracted or isolated send time.

For each group, cumulative rows at zero through six deliveries include preparation
and actual auxiliary schema/checkpoint transfer, then each delivered object once.
The zero row is paid setup alone. A second cumulative prefix includes connection
setup once and all earlier groups, so the second tier never silently resets the
already-paid session cost. Connection-setup attribution is not allocated twice
to the two groups. Finish and reopened-readback costs remain separate; the full
client session interval includes the peer's readback before finish acknowledgment.
For cumulative rows, `client_wall_us` is the sum of those measured paid intervals,
not a second uninterrupted stopwatch; uninstrumented bookkeeping gaps remain
visible only in whole-session wall. Sender/receiver CPU sums remain separate.
The generic `receiver_cpu_us` field is handler CPU for operation/group rows and
setup plus handler CPU for connection-prefix rows; operation rows also name the
narrow `receiver_handler_cpu_us` field explicitly. Neither includes receiver
finish/readback work, which remains in separate session fields.
Connection-setup objects and every connection-prefix row/summary also carry an
explicit `receiver_cpu_scope` marker distinguishing setup-only from setup plus
preceding handlers. These labels do not change the arithmetic.

Whole-session IP, UDP-payload and Ethernet bytes come from the single bidirectional
underlay capture. These are different accounting layers, not quantities to add.
Full process CPU is the sum of the two independent endpoint process counters;
client/server wall intervals remain separate because they overlap. Harness wall
adds capture/drain/store-cleanup overhead; shared topology/calibration and fixture
generation are reported separately. Per-operation wire bytes cannot be recovered
from these aggregate captures and are not invented.

The helper checks zero capture drops, accepted directional RTT/rate calibration,
drained finite queues, unchanged host-route receipts and successful completion.
Queue drops remain visible rather than disqualifying a slow or losing result.
Source/executable hashes, original PCAP custody and archived artifact integrity
remain a distinct parent-owned verification step; compact JSON arithmetic alone
does not authenticate those artifacts.

Each summary retains all three descriptive trials, their medians and matched
trial ratios against Raw and Whole-zstd. Ratios are paired before taking medians;
zero denominators remain null rather than dropping the observation or claiming
an infinite speedup. No outlier filtering, confidence interval or tail-latency
claim is made. Shaped local Linux traffic is not an actual Internet WAN, two
physical hosts, transparent TUN runtime compression, MVM qualification or an
energy measurement. Losing cases and initial setup remain in the output.

Focused arithmetic regressions use hand-computed bidirectional deltas, a seed
charged once, checked integer addition, paired-ratio medians and explicit zero
denominators. Negative receipt-validation coverage remains small and directly
related to the campaign joins; no new performance experiment is introduced.
