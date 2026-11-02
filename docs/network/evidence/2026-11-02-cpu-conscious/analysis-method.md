# CPU-conscious selector comparison

Registration is frozen before transfer measurements. All three arms use the same
current executable: exhaustive `combined`, opt-in `combined-cpu`, and unchanged
`whole-zstd`. Revision `9f5912698f6ab46ddc771fe15fd0c0a7bcec8c5b` anchors control
source equality; it is not a second timed binary. There are18 persistent sessions,
336operations and288 object deliveries:3arms ×2link profiles ×3trials, two size
groups per session, eight deliveries/group. Both Combined policies pay checkpoint
and schema setup; Whole pays no auxiliary setup. Every arm probes verified cache.

The six historical deliveries remain unchanged. Two controls follow them at each
existing exact length (1,047,202 and14,851,384 bytes). Schema-fit consists of256-byte
records matching the original48-byte value/message template. Schema-drift retains
one matching record, then changes the key to untrained `drifted`, with independently
seeded values. Both have bounded literal tails. Exact seeds, row layout and entropy
consumption are in registration. No target trains the schema; no fixture is tuned
after results. A fitting schema need not be the winning codec. The mixed drift case
intentionally keeps schema evaluation eligible despite most rows being literals.

The Rust offline analyzer uses the existing standalone runtime-analysis package:

```sh
cargo +1.97.1 run --release --locked \
  --manifest-path tools/gateway-network/runtime-analysis/Cargo.toml \
  --bin analyze_runtime_objects -- /absolute/path/to/campaign > analysis.json
```

Running through Cargo binds the executable to the checked-out analyzer source;
do not invoke an old target/release binary directly. Original receipts are retained
in a compact tarball with a per-file manifest; extract to a fresh owned directory
before reproducing analysis. Full original captures and executables remain in the
immutable HF archive with fresh download verification. Prior evidence is untouched.

Qualification requires the complete registered dose, trial-bound fixture identities,
all12 unique reopened exact files/session, matching peer namespaces, successful
publication, complete channel counters, receiver batches joined by transfer ID,
complete capture, valid measured RTT/rate and namespace cleanup. Historical decoder
gates remain: generated→RuntimeV1 transfer, opaque edit→SparseV1 transfer, both exact
repeats→Raw verified-cache-hit in both Combined arms. These rows retain their byte
savings. All other winners and byte differences are measured outcomes.

The opt-in receipt labels `selection_policy=cpu-conscious`. Its separate
`policy_skipped` entries identify SchemaZstdV1, reason no-useful-template-records,
and useful_template_records0. They have no invented size or lower bound. Exhaustive
has no policy skips. Proof-based `pruned` entries remain separately recorded.
Fitting and mixed drift must remain eligible; counts on evaluated cases are not
added to the receipt API. Core tests separately check canonical emitted-tag counts.

For every delivered object, compare actual logical request+response bytes against
the same-trial exhaustive control, including signed extra bytes, winner and skip
reason. Also retain same-source Whole comparisons, preparation and paid cumulative
costs at zero through eight deliveries. There is no blanket byte-equality gate for
the CPU policy: skipping a candidate can cost bytes. Do not relabel a heuristic skip
as a mathematical proof or claim an unencoded candidate's measured cost.

Logical bytes are encoded batches plus4-byte stream prefixes in both directions.
They exclude QUIC encryption, retransmissions, ACKs and connection setup/closure.
Captured bidirectional IP/UDP/Ethernet totals cover the complete two-tier session;
aggregate capture bytes cannot be assigned to individual objects. Source and binary
custody verification remains independent of JSON arithmetic.

Sender operation CPU/wall includes input loading, hashing, selection and verified
publication. Receiver handler CPU is joined by transfer ID and excludes stream wait.
Preparation, authentication, finish and reopened-readback costs remain separate.
Nested CPU stages are never added to whole-process CPU; overlapping endpoint wall
intervals are not summed as elapsed time. CPU is not subtracted to invent send-only
time. Paid cumulative intervals include each seed/preparation once; whole-session
wall retains bookkeeping gaps and finish/readback.

Report all three observations, descriptive medians and medians of matched ratios;
zero denominators remain null. Include negative CPU, time and byte results without
outlier filtering. LAN and calibrated80ms RTT/10Mbps namespace links share one Linux
runner. These are shaped QUIC transfers, not Internet-WAN, energy, transparent-TUN,
production or MVM qualification. Historical analysis outputs must reproduce
byte-for-byte without pooling their measurements into this campaign.
