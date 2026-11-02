# CPU-conscious Combined selection: measured opt-in result

The new opt-in policy avoids full schema compression when canonical preparation
emitted no useful template records. On the registered large unrelated workload,
sender CPU improves **16.77% on the shaped link** and **18.70% locally** versus
same-source exhaustive selection. Every paired operation sends the same logical
bytes in this campaign, and all **288 delivered objects reopen exactly**.

The full expanded mixed session improves shaped process CPU only **1.18%** and
has **0.045% worse elapsed time**. Large unrelated data still uses **5.18% more
sender CPU than Whole-zstd** by matched median. This is a useful targeted CPU
reduction, not elimination of all overhead or a full-session latency improvement.
Exhaustive remains the default; CPU-conscious is an explicit option.

Measured source `30792f17d0a596c4344e4780ebd4b8f455e3bedb`.
[Implementation PR36](https://github.com/R4-Laboratories/SpiralCore/pull/36),
[run 41465450384](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41465450384),
[raw evidence and reproduction](evidence/2026-11-02-cpu-conscious/README.md).

## Unrelated-object transfers

Median observations from three paired trials per link. CPU is sender process CPU
during the operation; elapsed includes preparation, transfer and verified durable
publication. Setup and reopened readback are additionally charged in the session
totals below. Logical channel bytes count both directions, excluding QUIC/IP.

| Link | Object bytes | Policy | Logical bytes | Sender CPU | Completion elapsed |
|---|---:|---|---:|---:|---:|
| 80 ms / 10 Mbps | 14,851,384 | Exhaustive | 14,883,962 | 234.144 ms | 12.926468 s |
| 80 ms / 10 Mbps | 14,851,384 | CPU-conscious | 14,883,962 | 208.266 ms | 12.907807 s |
| 80 ms / 10 Mbps | 14,851,384 | Whole-zstd | 14,883,962 | 201.699 ms | 12.895526 s |
| 80 ms / 10 Mbps | 1,047,202 | Exhaustive | 1,049,988 | 20.797 ms | 1.244890 s |
| 80 ms / 10 Mbps | 1,047,202 | CPU-conscious | 1,049,988 | 18.994 ms | 1.243041 s |
| 80 ms / 10 Mbps | 1,047,202 | Whole-zstd | 1,049,988 | 14.198 ms | 1.073906 s |
| Unshaped local | 14,851,384 | Exhaustive | 14,883,962 | 161.836 ms | 219.371 ms |
| Unshaped local | 14,851,384 | CPU-conscious | 14,883,962 | 130.488 ms | 197.112 ms |
| Unshaped local | 14,851,384 | Whole-zstd | 14,883,962 | 126.111 ms | 187.774 ms |
| Unshaped local | 1,047,202 | Exhaustive | 1,049,988 | 11.533 ms | 13.561 ms |
| Unshaped local | 1,047,202 | CPU-conscious | 1,049,988 | 9.178 ms | 10.846 ms |
| Unshaped local | 1,047,202 | Whole-zstd | 1,049,988 | 9.106 ms | 10.617 ms |

Percent changes use the median of same-trial ratios, not ratios of the separate
medians above. The large shaped CPU pairs improve 16.77%,18.53%,4.21%; all remain.
The corresponding elapsed improvement is only 0.144%. Small shaped CPU improves
11.49%, elapsed 0.153%. Large local CPU improves 18.70%, elapsed 14.31%; small local
CPU improves 20.42%, elapsed 20.50%.

Against Whole-zstd, the CPU-conscious large shaped case still costs 5.18% more
sender CPU and 0.106% more elapsed. Small shaped costs 6.34% more CPU and 15.78%
more elapsed by paired median. Those paired percentages are not implied by
dividing the table medians: raw small shaped CPU triples are exhaustive
15.130/20.797/26.474 ms, CPU-conscious 13.392/18.994/20.760 ms and Whole
13.934/14.198/19.522 ms. Large local CPU remains 3.64% above Whole and elapsed
5.09% above; small local differences are +0.14% CPU/+1.54% elapsed.

## Complete mixed persistent session

Each session includes both sizes and eight delivered objects per size, plus
checkpoint/schema publication for both Combined policies. Captured IP counts
include both directions, connection setup/close, ACKs and retransmissions.
Process CPU sums both endpoints; client elapsed includes its setup and waiting
for the receiver's completion/readback handshake. Receiver readback CPU is in
the receiver process total, not added again as a nested stage.

| Link | Policy | Captured IP bytes | Summed endpoint CPU | Client elapsed |
|---|---|---:|---:|---:|
| 80 ms / 10 Mbps | Exhaustive | 33,946,691 | 1.954898 s | 31.037964 s |
| 80 ms / 10 Mbps | CPU-conscious | 33,943,850 | 1.924081 s | 31.060953 s |
| 80 ms / 10 Mbps | Whole-zstd | 58,149,609 | 2.471298 s | 49.862399 s |
| Unshaped local | Exhaustive | 33,078,569 | 1.356389 s | 1.469797 s |
| Unshaped local | CPU-conscious | 33,082,117 | 1.373229 s | 1.507931 s |
| Unshaped local | Whole-zstd | 56,779,205 | 1.461381 s | 1.442637 s |

Compared with Exhaustive, shaped matched process CPU improves 1.18%, while elapsed
is 0.045% worse. Local matched CPU improves 0.18% and elapsed 0.59%; again these
paired ratios differ from ratios of independent medians. Compared with Whole,
CPU-conscious retains **41.63% fewer captured IP bytes, 21.83% less process CPU
and 37.71% less elapsed** on the shaped link. Locally it uses 41.74% fewer IP bytes
and 6.03% less CPU, but elapsed is **1.41% worse**.

This workload adds two schema controls per size to the preceding six-object
sequence. Its aggregate percentages must not be read as a regression from the
older, different workload's 58.26% byte reduction. No historical timing is pooled.

## Bytes, retained benefits and losses

All **120 matched policy/exhaustive operation pairs**, including auxiliary
publications, have zero logical-byte difference. Complete session totals are
31,570,094 / 31,570,192 / 31,568,438 logical bytes across trials, identical between
policies on each link. Minor captured-IP differences are transport observations,
not new codec savings. The registered 48 required generated/opaque-edit/repeat
mechanism rows preserve their receipts. Large generated objects use 771 logical
bytes, residual and opaque edits 1,374, and exact repeats 230 under this channel's
accounting. These are preserved existing advantages.

There are 48 policy-skip receipts, but **only 12 replace a fully evaluated schema
candidate** in Exhaustive: the unrelated transfers. The other 36 replace existing
proof-based pruning. Reporting 48 avoided full compressions would overstate the
benefit. Skips never carry invented encoded sizes or lower bounds.

Fitting and mixed-drift controls keep schema eligible and select SchemaZstd in
both policies. For large shaped fitting data, CPU-conscious sends 7,278,804
logical bytes versus Whole's 7,361,483 (1.12% fewer), using 246.560 ms sender CPU
and 6.588085 s elapsed versus 166.524 ms and 6.525249 s. Large mixed drift sends
7,311,282 versus 7,361,985 bytes (about 0.69% fewer), using 252.933 ms and 6.597552 s
versus 170.534 ms and 6.538150 s. The small byte advantage still costs compute/time.

Observed losses versus Exhaustive remain: large shaped fitting sender CPU is
6.68% worse, drift 4.21% worse, residual 8.55% worse and opaque-edit 20.78% worse by
paired median. Large shaped generated elapsed is 2.02% worse and opaque-edit
elapsed 2.48% worse; the two repeat elapsed values worsen 0.75% and 0.11%. These
three-trial observations do not isolate their cause or establish statistical
significance. They explain why this report claims a targeted unrelated-object
CPU improvement rather than a general speedup over Exhaustive.

## Implementation and use

`CombinedSession::new` and the gateway's `combined` mode remain Exhaustive.
Choose `CombinedSession::new_with_policy(..., SelectionPolicy::CpuConscious)`,
the shared `ObjectSender::with_combined_policy` facade, or object gateway mode
`combined-cpu` to opt in. Existing checkpoint, recipe and schema setup is required.
The policy is fixed for the session; the shared core is available to integrations.

The canonical encoder counts actual size-saving tag-1 emissions in its existing
pass. A match that emits a literal does not count. Canonical bytes are unchanged.
After existing early bounds and canonical resource/poison checks, zero useful
records permit a policy skip; any useful record follows normal schema selection.
Exact-repeat, runtime and rolling Sparse remain ahead of these candidates.
Receipts distinguish `selection_policy`/`policy_skipped` from proven `pruned`.

Lossless reconstruction is mandatory in both modes. A skipped literal-only schema
encoding could nevertheless compress smaller than another candidate. No such byte
penalty appeared in this dose, but this is not a universal domination proof or
an exhaustive-byte guarantee for CPU-conscious selection. It is an eligibility
policy, not a hard CPU-time budget. Authorization, wire modes, cryptography,
receiver verification and durable publication are unchanged.

## Execution, verification and scope

The one registered run completed 18 sessions,336 operations,288 deliveries and
2,289,396,384 requested bytes without retries. One frozen Linux release executable
served all three arms; Whole encoder/probe/settings and locks match pinned
`9f5912698f6ab46ddc771fe15fd0c0a7bcec8c5b`. Deterministic public fixtures use the
unchanged schema, with fitting and independent mixed-drift controls fixed before
timing. Three complete order rotations retain every result and paid setup.

Directional shaped RTT means are 80.200–80.288 ms; calibrated receiver throughput
is 9.615–9.796 Mbps. Captures reconcile with zero capture drops. Finite shaped
queues dropped 1,349 packets; their recovery cost remains in measured traffic and
time. Host routes and frozen binaries stayed unchanged.

Local validation records 73 scoped transfer tests,15 gateway-core tests,34 gateway
tests,6 fixture tests and12 analyzer tests, with strict Clippy and formatting.
Independent source/security and measurement reviews passed; Linux checks and the
existing TUN regression passed separately. Initial REDs, one corrected test-fixture
assumption and a formatting correction are retained. Raw output whitespace is
preserved; source/document whitespace checks exclude original evidence logs.

All 913 artifact-manifest entries,169 measured-source files,157 control-source
files and three executables verify. The actual execution-workflow blob matches
the reviewed source. Full originals are in the existing private HF dataset at
an immutable revision, verified by forced fresh download and byte comparison.
Git retains readable analysis and a compact original-receipt bundle that
reproduces the analysis exactly. See the evidence index for identities/commands.

This is reliable authenticated QUIC between isolated namespaces on one hosted
Linux runner, including a properly shaped link. It is not a new two-host Internet
WAN, current Combined MVM, transparent TUN runtime-compression, physical-energy,
FIPS or CUI qualification. CPU overlaps elapsed; no subtraction invents send-only
time. Three trials support descriptive comparisons, not p95 or confidence claims.
