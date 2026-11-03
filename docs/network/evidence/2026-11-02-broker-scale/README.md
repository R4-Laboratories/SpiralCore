# Shared broker ownership and persistent ordering: measured local load

The shared Rust broker budget/clock and stable ordering streams are implemented.
The load campaign exercised **2, 8 and 32 logical endpoints**, using real TCP
loopback and canonical encrypted MVM messages. Stable streams retained **3, 12
and 48 FIFO checkpoints** during the paced runs instead of roughly 1,950 per
cell. Process peak RSS decreased in every matched comparison. CPU and latency
varied; these single trials do not establish a universal speed improvement.

Measured source: `65abaa0b68a1a0e28b8e3aa713dfb4c8addf29dc`, based on main
`6bf85f9`. The executable actually used had SHA-256
`9117e495f886e56ed9c30486b785c6bcb2fe898510bd11486a30c1973c1a8d31`.
Later documentation and test-only diagnostic changes are not relabeled as the
measured binary. The source, registration and raw records support rebuilding;
the executable is not vendored or promised to remain at its local build path.

## Implementation and protocol continuity

- `broker::BrokerContext` owns one shared `HostBudget`, monotonic clock origin
  and synchronous local dispatch gate. Time is sampled after acquiring the gate
  so independently scheduled endpoint workers cannot submit stale timestamps to
  the budget's globally monotonic accounting. It does not perform socket I/O.
- `service_sender::OrderingStream` owns a stable canonical scope with checked
  sequence allocation, bound to address/sender/execution epoch/kind. Invalid
  metadata and wrong Reply bindings do not consume positions. Retries preserve
  the original message options. Replay cursors are not silently evicted.
- `PreparedReceipt::reserve_send` reserves one control/rate attempt during
  broker dispatch. Its consuming permit sends outside dispatch, retaining its
  charge and rechecking current policy generations, expiry and cancellation.
  The combined send API remains compatible with single-owner callers.
- Native `sender_with_broker` and `receiver_with_broker` accept the same injected
  owner. Single-endpoint convenience wrappers remain explicit. Separate guest
  processes still need aggregate host supervision; a Rust context does not
  automatically aggregate processes or turn the witness into a production broker.

Canonical MVM cryptography, grants, envelope format, four-envelope Call lifecycle
and the object-v1 codec remain intact. No ternary/CA/Kuramoto, warm-cache, sparse,
schema or zstd implementation was copied or changed. The existing native object
campaign tests retain exact recovery, including the larger object/reopen case.
This load probe isolates mailbox state; it does **not** measure codec savings.

## Measurement contract

The [registration](registration.json) preceded the successful campaign. Each
cell uses a fresh process on **Apple M1, 8 logical CPUs, 16 GiB RAM, macOS 27.0
build 26A5378n, ARM64**, Rust 1.97.1 release build with `live`, locked dependencies
and no `hardware-sha256` feature. The host was shared and not isolated; starting
load averages were 10.62/16.51/12.32. There is one trial per cell, ordered
fresh/stable, without randomized repeated-pair confidence intervals.

Endpoint counts include both sides: 2/8/32 service endpoints form 1/4/16
persistent TCP pairs. One broker dispatches operations serially in round robin,
with pipeline one. This is not 32 simultaneously outstanding operations or 32
booted MVMs. Calls use four encrypted envelopes; generic Casts use request plus
admission receipt, without an application Reply. Payloads are exactly **4,096
bytes**, including embedded/trailing zeros and an operation counter.

The explicit probe profile uses native-witness identity ceilings, quarter
control reservations, a one-second application-message TTL and one-second
`ServiceLimits` retained-message extension. Production defaults are unchanged.
`ReceiptTracker` still retains terminal send records for its existing fixed
**300,000 ms extension**. A 65-second run crosses rate-history windows but does
not qualify that full tracker-retention cycle or an all-state memory plateau.

Byte totals are actual canonical envelopes plus stream framing in both directions,
including setup/readiness traffic. They exclude TCP/IP headers and kernel
retransmission. CPU is aggregate process CPU for both endpoint roles and broker;
setup is recorded separately. Operation latency includes processing and socket
exchange. Do not add CPU to elapsed or infer send-only time or physical energy.
RSS is the kernel's lifetime process peak, independently measured in each child.
State maxima and timelines are post-operation samples, not inflight high-water
marks. Descriptor sampling is also explicit in the raw rows.

## Paced persistent sessions

Each cell ran for approximately **65 seconds at 20 offered operations/s**, with
**1,300 completed operations**. Every accepted request was verified; there were
no incomplete admissions, mismatched byte comparisons, counter disagreements or
unexpected teardown errors. All six cells together performed **7,800 operations
and 11,716 exact request/Reply readbacks**.

CPU below excludes separately recorded setup. MB means decimal bytes; latency
is complete-operation p95. Raw records also contain p50/p99 and separate
admission/application-Reply distributions.

| Endpoints | Scope policy | Both-direction bytes | CPU seconds | p95 ms | Peak RSS MB | Retained FIFO scopes |
|---:|---|---:|---:|---:|---:|---:|
| 2 | Fresh | 30,711,254 | 13.808 | 25.073 | 26.49 | 1,950 |
| 2 | Stable | 30,716,558 | 13.954 | 23.592 | 21.51 | 3 |
| 8 | Fresh | 30,744,886 | 14.685 | 21.245 | 26.08 | 1,952 |
| 8 | Stable | 30,747,047 | 15.176 | 24.195 | 21.74 | 12 |
| 32 | Fresh | 31,076,672 | 15.126 | 24.458 | 26.02 | 1,956 |
| 32 | Stable | 31,079,449 | 13.959 | 21.712 | 22.56 | 48 |

Stable RSS decreased **18.8%, 16.6% and 13.3%**. CPU changed **+1.1%, +3.3%
and −7.7%**, respectively. The 8-endpoint stable cell also had higher p95
latency; it remains a recorded loss. Both arms intentionally ran for the same
paced duration, so equal 65-second cell times do not demonstrate transfer speedup.

These are uncompressed generic-message controls. Stable wire bytes rose by
**0.007–0.017%**, consistent with longer increasing JSON sequence numbers.
The win here is persistent state ownership, not compression. No earlier object
codec/WAN measurement is replaced by this result.

## Unpaced saturation and incomplete work

Each cell ran until the first error, its registered operation bound or 120-second
hard stop. All six reached an explicit `Full`/`Service(Full)` admission refusal.
The actual failure phase is retained; the counters do not identify the precise
host/tenant/sender quota that bound first. Do not infer a specific limiter from
the generic error alone.

| Endpoints | Scope policy | Completed operations | Both-direction bytes | CPU s | Elapsed s | Peak RSS MB | FIFO scopes |
|---:|---|---:|---:|---:|---:|---:|---:|
| 2 | Fresh | 1,638 | 38,711,545 | 13.096 | 18.228 | 36.67 | 2,457 |
| 2 | Stable | 1,638 | 38,720,493 | 12.187 | 13.080 | 30.26 | 3 |
| 8 | Fresh | 3,640 | 86,017,461 | 26.447 | 30.574 | 66.24 | 5,460 |
| 8 | Stable | 3,640 | 86,031,909 | 26.811 | 28.506 | 55.71 | 12 |
| 32 | Fresh | 3,637 | 86,767,248 | 25.859 | 31.101 | 67.70 | 5,461 |
| 32 | Stable | 3,637 | 86,775,771 | 26.154 | 27.890 | 52.25 | 48 |

Stable scopes did **not** increase the completed-operation count before this
next admission limit. They decreased RSS by **17.5%, 15.9% and 22.8%**; CPU
changed **−6.9%, +1.4% and +1.1%**. The single-trial elapsed differences are
observations, not controlled statistical speed claims.

There were **six admitted-but-incomplete requests**, one per cell. Four had no
receiver payload comparison before the refusal; two had a verified request but
an incomplete application exchange. Those requests are not counted as delivered
successes. All performed comparisons remained exact: **26,760 readbacks over
17,830 completed operations**, including the two partial request readbacks.
After the explicit 2.1-second TTL/retention wait, all cells had zero active Calls
and zero queued messages, without resetting services. Replay/rate/tracker
identities remained retained; queue cleanup does not prove immediate renewed
admission capacity or completed application processing.

All process peaks were below the registered 512 MiB ceiling, and sampled
descriptors were 6/12/36, below 128. These are this workload's observations, not
recommended production memory budgets or descriptor capacity limits.

## Reproduction and preserved failures

```sh
cargo +1.97.1 build --release --locked \
  --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml \
  --features live --example service_capacity
```

Run the resulting `service_capacity NEW_PACED.jsonl 65000` followed by
`service_capacity NEW_SATURATION.jsonl 0`, using new output paths. The CLI starts
one fresh bounded child per cell. `--cell 2 stable 1000` selects a short single
cell. The probe writes compact JSON, never persistent payload datasets or VM disks.

- [Paced raw records](paced-v2.jsonl), [saturation raw records](saturation.jsonl)
  and [derived summary with source/hash/host identities](summary.json).
- [Initial teardown failure](attempt1-teardown-failure.json) and its unchanged
  empty [result file](paced.jsonl): the original recorder propagated a socket
  close error and persisted no row. It is not counted as a passed trial.
- The corrected recorder retains raw close errors. `NotConnected` is labeled
  `peer_already_disconnected`; the OS error itself does not prove which side
  initiated the disconnect. Unexpected close errors are separately counted.
- Original Intel macOS portable-test failure is retained under
  [qualification](qualification/). It remains distinct from local release load
  measurements; exact platform qualification is recorded separately.

The diagnostic Windows Server 2025 run at `0ee32ab` recorded `Unauthorized` at
Reply admission after 1.297524 seconds under the one-second profile, with an
exact request readback and no completed exchange. Inherited Reply expiry is the
source-supported diagnosis; the row does not retain exact issued/expiry/admission
timestamps to establish a unique trace-level cause. The
[original artifact and diagnosis](qualification/windows-time-budget-diagnosis.json)
remain preserved; the earlier Intel run did not capture its exact refusal.
Portable functional CI now uses a separately named 30-second smoke profile,
fixed to two endpoints and four operations. The benchmark CLI and recorded
one-second results retain their original profile; no retry silently increases
their TTL. Production limits and expiry tests are unchanged.

## Remaining scale gates

[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49) stays open. The
next work is precise quota attribution, sustained tracker-retention/recovery
testing, hot/slow/malformed-peer fairness, inflight/lock-wait measurements and
broker authority-preserving scheduling under large reconstruction. Measure those
before sharding locks or standardizing a bounded correlated pipeline window.
Real WAN, actual guest populations, production mailbox adoption, FlowMux and
owner access remain separate final-delivery gates.
