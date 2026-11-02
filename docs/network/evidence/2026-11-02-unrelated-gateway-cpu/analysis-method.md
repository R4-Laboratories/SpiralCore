# Registered unrelated-object CPU paired arithmetic

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

The helper refuses incomplete dose: 18 distinct sessions, three explicit binary/profile arms,
two link settings and trials 0–2; 264 successful operations and 216 delivered
objects with the registered lengths and total requested bytes. It binds every
operation to the trial fixture manifest, its receiver's reopened object identity
and exact-readback assertion, and its `transfer_id`-matched receiver batches.
The two repeats retain their original fixture IDs. Checkpoint/schema operations
are separate paid setup in both Combined arms. All six data operations are
selected sends; this comparison has no forced raw target seed.

Validation includes complete singleton session metadata, client/server namespace
agreement, matching logical channel totals in opposite directions, monotonic
per-operation counter snapshots, nonnegative integer timings, successful
publication paths and receiver batch attribution. Both Combined arms must execute
RuntimeV1 on fresh generated targets, SparseV1 on opaque edits, and Raw verified
cache hits on both exact repeats. Their residual winner must match each other;
it is not preselected by this campaign.
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
trial ratios against pinned Combined-before and current Whole-zstd. Ratios are paired before taking medians;
zero denominators remain null rather than dropping the observation or claiming
an infinite speedup. No outlier filtering, confidence interval or tail-latency
claim is made. Shaped local Linux traffic is not an actual Internet WAN, two
physical hosts, transparent TUN runtime compression, MVM qualification or an
energy measurement. Losing cases and initial setup remain in the output.

Focused arithmetic regressions use hand-computed bidirectional deltas, a seed
charged once, checked integer addition, paired-ratio medians and explicit zero
denominators. Negative receipt-validation coverage remains small and directly
related to the campaign joins; no new performance experiment is introduced.

The strict unrelated-gateway-CPU registration fixes baseline source
4a8ec79c9b185c85407e9767653dbcc4af97bf7e and arms combined-before,
combined-after and whole-zstd. Both Combined arms use mode combined and pay
checkpoint then schema before six sends in each tier. This is264operations:
216send+24schema+24checkpoint, with216deliveries and1,717,047,288 requested
bytes. Three complete arm-order rotations are used. Both service executables
are built on the same runner with pinned Rust1.97.1 and the same release flags;
source manifests, baseline git HEAD, exact control-source/lockfile equality and
per-session service binary hashes bind which executable was used.

Before/after parity checks every operation's group/label/action, exact target
identity/length, selected mode, publication path, frame/data byte counts, request
and batch counts, and actual request/response logical counter deltas. Missing
fields, counter rollback or any mismatch reject qualification. Timing and
candidate/pruned receipts may differ. Six explicit parity receipts are reproduced
from raw client records and checked against retained harness receipts. Existing
network receipts do not expose canonical offer/base/body hashes: that stronger
byte-equality proof belongs to the separately retained frozen-baseline core tests,
not to encrypted PCAP inference. Captured IP totals may legitimately vary due
to acknowledgments and retransmissions and are compared as measured outcomes.

Each Combined group's zero-delivery cumulative row includes both auxiliary
publications and preparation exactly once. Full-session captured bytes and
process CPU remain separate from nested stage metrics. The optional native
candidate-stage profile is a distinct diagnostic: independent stage timings
are not summed into a transfer or substituted for actual gateway measurements.

Older Runtime, Combined and candidate-CPU policies remain separately supported with their original schema and source pins. Their complete
historical analysis JSON is regenerated and byte-compared after this change;
old evidence and timings are never rewritten or pooled with this comparison.

This milestone changes only exact residual run search. The fixture generator,
workload, transport, zstd settings and paired receipt fields stay unchanged.
Unrelated and all other per-object results remain visible, even if the complete
CPU or elapsed gap against Whole-zstd remains. No isolated stage timing is
substituted for end-to-end work.

Compact original public receipts will be retained in one `receipts.tar.gz` with
its SHA-256 and per-file manifest, alongside readable analysis. Extract it to a
fresh owned directory and pass its campaign path to the same analyzer command.
This changes packaging only: original JSON/log bytes are preserved. Full captures
and executables remain in the immutable, freshly re-downloaded HF original ZIP;
private keys/configuration, symlinks and large generated data are excluded from
the public receipt subset. Prior evidence directories are not rewritten.
