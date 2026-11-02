# Shared runtime and warm objects through the paired gateway

**Both existing mechanisms now work through the authenticated object gateway,
with exact receiver reconstruction and measured savings on the shaped link.**
Across the complete mixed sessions at 80 ms RTT / 10 Mbps, Warm reduced captured
IP bytes by **17.09%**, summed endpoint process CPU by **15.77%**, and client elapsed
time by **15.90%** relative to prepared whole-object zstd. Runtime reduced those
measures by **17.11%, 8.04%, and 15.05%**. These are medians of three matched-trial
reductions, including setup, both object tiers and receiver reopened-readback.
They are not universal compression ratios or an actual Internet WAN result.

The profiles expose different strengths. Runtime reconstructs predictable objects
from the shared checkpoint/recipe and residual; Warm reuses the latest object for
sparse edits. **They remain separate explicit profiles, not a combined selector.**
The measurements expose the cost of that separation: Warm pays an expensive raw
initial seed, while Runtime misses the useful rolling base for opaque edits.

Measured source is `d5f35a34c3c2c63c18deaf84739d10c43da7d2b4` in
[PR #27](https://github.com/R4-Laboratories/SpiralCore/pull/27), executed by
[run 41373931230](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41373931230).
The [validated analysis](evidence/2026-11-02-runtime-gateway/analysis.json) retains
all individual trials, operation rows, costs and matched comparisons. The complete
archive's 1,021 entries, three executable identities and 156 source identities
were verified separately from the arithmetic. Compact original receipts remain in
[linux-run](evidence/2026-11-02-runtime-gateway/linux-run).

## What was transferred and compared

The campaign completed **24 sessions, 312 operations and 288 requested deliveries**,
with **zero fidelity loss** on published identities, exact lengths and receiver
reopened byte comparisons. Total requested object bytes were **2,289,396,384**.
Each persistent session transferred two six-object sequences: generated output,
residual edit, exact repeat, unrelated opaque object, localized opaque edit, and
its exact repeat. Object lengths were **1,047,202 bytes** (`near1m`) and
**14,851,384 bytes** (`max-trace`, about 14.16 MiB).

The four arms used identical authenticated transport and durable publication:
Raw; prepared whole-object zstd with its existing raw fallback; rolling Warm;
and prepared Runtime. **All four used the same verified-cache probe**, so even
Raw and Whole-zstd avoided retransmitting exact repeats. Warm's generated-object
seed counted as the first delivery, not extra uncounted setup. Its schema and
Runtime's checkpoint publications were separately paid operations. Runtime
preparation and receiver replay were charged; predictions were not supplied free.

These synthetic fixtures deliberately exercise deterministic generation and
localized reuse. The runtime uses the existing shared cellular-automaton/recipe
codec; no new mathematics engine was substituted. Fresh generated/residual
Runtime cases executed RuntimeV1, and Warm opaque edits executed SparseV1.
Published-at-Begin cache hits are identified separately from decoder execution.

The unshaped and shaped cases ran between Linux network namespaces on one runner.
Directional calibration measured 80.206–80.288 ms RTT and 9.611–9.795 Mbps in the
shaped profile; the unshaped profile was roughly 0.040–0.051 ms RTT. Captures were
complete with zero capture drops. The finite network queues dropped **1,574
packets** across the shaped campaign, which remain in the cost and retransmission
measurements. This is controlled network impairment, not a real Internet path.

## Complete mixed-session costs

Every row below is the median of three sessions. Each session includes both
object tiers and twelve deliveries, connection/codec setup, auxiliary seeds,
finish and receiver reopened-readback. IP bytes are captured bidirectional totals,
including transport overhead. CPU is measured process time in seconds; elapsed
is the client's complete session interval. Summed CPU is summed per trial before
its median, so it need not equal the sum of the displayed component medians.
The service timers begin inside `run_inner`, after CLI configuration parsing,
validation and output reservation, and end before the final stats serialization/
write and endpoint shutdown. They measure the service session through reopened
readback, not OS process launch through termination; capture continues through
close. The analysis retains separate harness wall including launch/capture/cleanup.

| Link | Arm | Captured IP bytes | Sender CPU s | Receiver CPU s | Summed CPU s | Client elapsed s |
|---|---|---:|---:|---:|---:|---:|
| Unshaped | Raw | 66,765,960 | 0.517692 | 0.564891 | 1.082583 | 0.951781 |
| Unshaped | Whole-zstd | 40,269,966 | 0.543629 | 0.453608 | 1.001667 | 0.999259 |
| Unshaped | Warm | 33,393,139 | 0.475114 | 0.417995 | 0.893109 | 0.905908 |
| Unshaped | Runtime | 33,399,780 | 0.530890 | 0.458023 | 0.988489 | 1.017856 |
| 80 ms / 10 Mbps | Raw | 68,293,195 | 0.837343 | 1.373719 | 2.191453 | 56.620919 |
| 80 ms / 10 Mbps | Whole-zstd | 41,294,155 | 0.747579 | 0.945115 | 1.692694 | 35.480427 |
| 80 ms / 10 Mbps | Warm | 34,235,554 | 0.607903 | 0.843186 | 1.453566 | 29.852479 |
| 80 ms / 10 Mbps | Runtime | 34,230,457 | 0.694412 | 0.865645 | 1.560057 | 30.139607 |

On the unshaped profile, Warm's matched reductions versus Whole-zstd were 17.08%
IP bytes, 10.44% CPU and 10.49% elapsed. Runtime saved 17.06% IP bytes and 1.32%
CPU, but elapsed time was **0.78% higher**: it did not establish an unshaped
end-to-end speed advantage for this mixed workload. Against Raw on the shaped
link, Warm saved 49.87% IP bytes, 35.09% CPU and 47.30% elapsed; Runtime saved
49.88%, 29.13% and 46.77%, respectively.

All percentages above come from matched-trial ratios before taking their median,
not by dividing the medians in the table. Three trials support descriptive
comparison, not confidence intervals or p95 claims.

## Where the operation-level advantages are

The following 14,851,384-byte operations used the shaped link. Bytes here are
**logical bidirectional object-channel bytes**, including batch prefixes, not an
allocation of the session's IP capture. CPU columns are milliseconds: sender
operation work and receiver Endpoint-handler work joined by transfer ID. The
elapsed interval includes input read/hash, selection/encoding, network waits,
decoding and validated publication acknowledgment; it excludes prior group setup.
CPU uses process-time counters over those windows, so concurrent Tokio activity
in the same process is included; handler CPU is not an isolated decoder profile.

| Operation | Arm | Logical bytes | Sender CPU ms | Receiver handler CPU ms | Elapsed ms |
|---|---|---:|---:|---:|---:|
| Generated | Whole-zstd | 3,194,293 | 110.613 | 24.674 | 3,008.031 |
| Generated | Runtime | 771 | 15.697 | 43.193 | 266.529 |
| Generated seed | Warm | 14,883,930 | 182.890 | 25.949 | 12,806.198 |
| Residual edit | Whole-zstd | 3,194,762 | 94.221 | 22.498 | 2,985.315 |
| Residual edit | Runtime | 1,571 | 77.614 | 42.253 | 330.322 |
| Residual edit | Warm | 1,374 | 68.737 | 25.729 | 304.343 |
| Opaque localized edit | Whole-zstd | 14,883,962 | 189.964 | 20.721 | 12,888.220 |
| Opaque localized edit | Runtime | 14,883,962 | 180.959 | 20.231 | 12,879.997 |
| Opaque localized edit | Warm | 1,374 | 52.012 | 21.060 | 278.291 |

Runtime's large generated case reduced logical bytes by 99.976%, combined stage
CPU by 56.21%, and elapsed by 91.30% relative to Whole-zstd. Warm's large opaque
edit reduced these by 99.991%, 64.64%, and 97.84%. Runtime's residual case saved
99.951% of logical bytes and 88.92% of elapsed time, with similar combined stage
CPU to Whole-zstd. Receiver reconstruction is real work: it is not inferred to
be free from the very small message size.

At **1,047,202 bytes**, Runtime generated output used 771 bytes / 172.781 ms versus
Whole-zstd's 88,481 bytes / 422.392 ms; its residual used 1,571 bytes / 171.439 ms
versus 89,002 bytes / 280.884 ms. Combined operation CPU was higher for Runtime
in both small cases: matched increases of 21.60% and 12.56%. Warm's small opaque
edit used 1,374 bytes / 167.904 ms versus 1,049,988 bytes / 1,084.967 ms, with
69.28% less combined stage CPU.

The local-link elapsed tradeoff remains visible: large Runtime generated output took
106.474 ms versus Whole-zstd's 149.251 ms, but its residual took 159.425 ms versus
147.644 ms. Small Runtime generated output took 8.267 ms versus 7.743 ms.
Byte reduction is substantial in those cases; CPU and low-latency elapsed wins
are conditional.

## Setup is paid, and the initial Warm seed loses

These shaped-link cumulative rows include each group's preparation and actual
schema/checkpoint transfer. The first-delivery columns include setup plus generated
output; the six-delivery columns include setup plus that entire tier. Stage CPU
is the per-trial sum of sender work and receiver handlers. Elapsed is a sum of
those measured paid intervals. Connection authentication is not charged again to
each group; it remains in the complete-session table and the analysis's separate
connection-prefix rows. Finish/readback also remains in the complete session.

| Tier | Arm | First delivery bytes | First stage CPU ms | First elapsed s | Through six bytes | Six stage CPU ms | Six elapsed s |
|---|---|---:|---:|---:|---:|---:|---:|
| Near 1 MiB | Whole-zstd | 88,481 | 8.980 | 0.422412 | 2,277,907 | 67.227 | 3.024905 |
| Near 1 MiB | Runtime | 4,646 | 12.855 | 0.258836 | 2,106,685 | 70.279 | 2.970744 |
| Near 1 MiB | Warm | 1,050,497 | 21.553 | 1.243776 | 2,103,693 | 61.183 | 2.855855 |
| 14.16 MiB | Whole-zstd | 3,194,293 | 134.662 | 3.008044 | 36,157,439 | 814.646 | 32.064046 |
| 14.16 MiB | Runtime | 4,508 | 81.554 | 0.372384 | 29,774,495 | 772.348 | 26.780312 |
| 14.16 MiB | Warm | 14,884,333 | 209.218 | 12.887444 | 29,771,503 | 670.242 | 26.598627 |

Warm's current first large seed costs **4.66 times the bytes and 4.28 times the
elapsed time** of Whole-zstd after setup, with 52.13% more combined stage CPU
(matched medians). Its later sparse changes amortize that loss over the registered
six-object sequence; they do not erase the first-delivery penalty. Near 1 MiB,
Runtime's setup-paid first delivery still saved 94.75% of bytes and 38.68% of
elapsed, but cost 47.64% more stage CPU. Through six small deliveries, its stage
CPU remained 10.60% higher and elapsed improved only 1.76%.

Runtime's fixed checkpoint does not adopt the unrelated opaque object as a rolling
base: its next opaque edit still sends 14,883,962 bytes. Exact repeats also expose
avoidable work. On the large residual repeat, Runtime used 262 bytes and
167.888 ms versus Whole-zstd's 230 bytes and 158.460 ms; combined stage CPU was
87.464 ms versus 78.016 ms. Warm used 230 bytes and 103.063 ms with 22.360 ms
combined stage CPU. Warm unrelated-object transfers saved no bytes over the raw
fallback and cost 5.91% more combined stage CPU in the large case.

## Interpretation and next implementation

The next bounded change is to let one session use both mechanisms: keep the
prepared runtime checkpoint as a prediction candidate, keep the verified rolling
object for sparse updates, and compare them with the prepared whole-object/raw
fallback under one acknowledged receiver-state contract. The measurements justify
that integration; they do **not** establish what its combined performance will
be. The first targets are removing the raw cold-seed penalty and avoiding runtime
residual/hash preparation before a cheap exact-repeat decision, while retaining
all current byte savings and exact publication checks.

Raw receipts and the [analysis method](evidence/2026-11-02-runtime-gateway/analysis-method.md)
separate operation logical bytes from captured session bytes, handler CPU from
whole-process CPU, and paid group intervals from complete session wall. Actual
send-only time cannot be recovered by subtracting CPU from wall because CPU,
network activity and waiting overlap. No physical-energy saving is established.

The earlier [failed run](evidence/2026-11-02-runtime-gateway/failed-run-41373268385)
remains retained: its first Raw session reconstructed exactly, but the capture
was seven packets incomplete and was rejected. The corrected run used immediate
capture delivery plus bounded drain checks; capture acceptance was not relaxed.

This delivery is the explicit cooperating object interface over authenticated
QUIC. Ordinary TUN packets still use their separate datagram profile; arbitrary
TLS ciphertext does not acquire runtime/cache compressibility from this result.
Live MVM boundaries, production multi-tenant operation, dynamic capability
revocation, FIPS/CUI qualification and real Internet WAN testing remain distinct
integration and qualification work.
