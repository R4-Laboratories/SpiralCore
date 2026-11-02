# Current Combined core through real MVM/vsock boundaries

SC-MVM-004 integrates the current shared Rust Combined selector into the existing
canonical MVM mailbox witness. All **36 native HVF sessions succeeded**, all
**336 publications reopened exactly**, and all **168 publications received by a
second guest passed a separate keyless, read-only reboot verification**.

The large two-guest mixed session uses **41.92% fewer actual encrypted stream
bytes, 25.37% less sender CPU, 26.25% less measured endpoint partial CPU, and
29.21% less sender session elapsed time** with exhaustive Combined than with
matched prepared whole-object zstd. These are medians of three paired trial
ratios. They establish a positive local MVM integration result on this workload,
not a new two-host Internet WAN result or a universal compression advantage.

## Implementation and boundary

Measured source: `9ceaeef5070c87fd17b8519a1e8630b7f7888fd6`, based on main
`95d9b65b1c213c36adc77ec9abf93935f8a5fcbc`. The new explicit arms are `combined`,
`combined-cpu`, and `whole-mixed`. Legacy `warm`, `whole` and `raw` remain intact.
Combined uses the existing `ObjectSender::with_combined_policy`; CPU-conscious
remains opt-in. No compression algorithm, receiver wire format, crypto or mailbox
authorization semantics were changed.

The ternary/Kuramoto/CA engine is active: both Combined arms select `RuntimeV1`
for every generated object, `SparseV1` for every residual and opaque edit,
verified presence for exact repeats, and `SchemaZstdV1` for both schema controls.
The same prepared prediction and acknowledged rolling state are reused throughout
each session. Exact object length/digest and authenticated publication remain the
correctness authority; this does not require identical physical clocks.

Encoding occurs inside the authorized plaintext endpoint before canonical MVM
encryption; decoding follows canonical verification/decryption. The two-guest
host relay is keyless. This uses MVM
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b` plus the existing retained patch on
experimental vsock **5257**. Production activation remains disabled; FlowMux
**5253** and ordinary TUN traffic are unchanged.

## Registered workload and measurement

Three rotated paired trials cover one guest → host and two guests → keyless host
relay. Each arm has a fresh store that persists across eight targets: generated,
residual, repeat, unrelated, opaque-edited, opaque-edited-repeat, schema-fit and
schema-drift. Combined also pays checkpoint and schema publications. The 96
cross-arm payload groups match exactly by object identity and length.

Nominal 1/16 MiB fixture tiers produce **1,047,202 / 14,851,384 bytes per target**.
The eight-target sessions request 8,377,616 / 118,811,072 payload bytes. Across all
arms/stages/trials this is **2,289,396,384 requested payload bytes**, plus 48 setup
publications. Fixture identities match the previous gateway campaign; the data
was not refitted after timing.

Every arm uses the same explicit **64 MiB/s allowance and 8 MiB burst**. The
default 4 MiB/s policy remains unchanged. This is not a cross-rate comparison.

Actual logical stream bytes count both directions of encrypted MVM envelopes,
framing, readiness and replies. Two-guest physical stream bytes count both hops
and are twice the logical count; virtio credit/control traffic is excluded.
Selector HTTP-body candidate costs and planned application frames are retained
as diagnostics, not substituted for these actual counters.

Sender session elapsed includes fixture/codec preparation, handshake, setup
publications and target operations. It excludes JSON reporting, ownership copies
and independent readback; complete VM lifecycle is separately recorded. Common
synthetic fixture generation includes CA replay for all arms, and Combined also
pays prepared prediction construction. Thus setup CPU is not pure encoding CPU.
Operation timing covers encode through authenticated publication reply.

Sender and receiver CPU are separately measured. Their partial sum excludes
receiver framed receive/allocation and idle waiting. Worker/VMM, orchestrator and
recovery CPU are separate counters; never add guest CPU to its enclosing worker
CPU. Exchange wall time includes crypto, remote work and waiting, so it is not an
isolated send-only measurement. No physical energy measurement was made.

## Paid session results

Values below are independent arm medians, n=3. Percentages elsewhere use medians
of matched trial ratios, which need not equal ratios of these table medians.

| Topology | Actual target size | Arm | Logical encrypted bytes | Sender CPU, ms | Endpoint partial CPU, ms | Sender session elapsed, ms |
|---|---:|---|---:|---:|---:|---:|
| Guest → host | 1,047,202 | Combined | 4,206,727 | 133.603 | 201.288 | 290.643 |
| Guest → host | 1,047,202 | CPU-conscious | 4,207,785 | 132.172 | 200.918 | 280.873 |
| Guest → host | 1,047,202 | Whole zstd | 6,486,072 | 149.661 | 225.010 | 298.142 |
| Guest → host | 14,851,384 | Combined | 56,543,001 | 1,466.424 | 2,153.157 | 2,339.153 |
| Guest → host | 14,851,384 | CPU-conscious | 56,539,518 | 1,465.900 | 2,145.210 | 2,348.549 |
| Guest → host | 14,851,384 | Whole zstd | 97,362,834 | 2,031.718 | 2,980.889 | 3,393.181 |
| Two guests | 1,047,202 | Combined | 4,207,092 | 130.471 | 215.902 | 234.177 |
| Two guests | 1,047,202 | CPU-conscious | 4,207,678 | 129.194 | 215.030 | 244.303 |
| Two guests | 1,047,202 | Whole zstd | 6,485,562 | 151.986 | 253.503 | 310.784 |
| Two guests | 14,851,384 | Combined | 56,538,203 | 1,442.327 | 2,379.372 | 2,620.826 |
| Two guests | 14,851,384 | CPU-conscious | 56,535,989 | 1,422.216 | 2,351.801 | 2,567.689 |
| Two guests | 14,851,384 | Whole zstd | 97,329,036 | 1,924.770 | 3,200.029 | 3,599.495 |

For two large-target guests, physical stream medians are **113,076,406 bytes**
Combined, **113,071,978** CPU-conscious and **194,658,072** Whole. Complete lifecycle
medians, including boot, shutdown and keyless reboot verification, are **4.168 s**,
**4.061 s** and **4.872 s**, respectively. These are not sender transfer times.

Large two-guest CPU-conscious versus Whole paired reductions are **41.91% bytes,
26.06% sender CPU, 25.66% endpoint partial CPU and 27.34% elapsed**. Against
exhaustive Combined, the same mode improves sender CPU **1.53%** and elapsed
**1.43%** on the paid session. This is a small, workload-specific result; it does
not justify changing the default or establish universal policy superiority.

## Where the gains and losses occur

Selected large two-guest operation medians; each row reconstructs 14,851,384 bytes:

| Workload | Combined logical bytes | Whole logical bytes | Combined sender CPU, ms | Whole sender CPU, ms | Combined elapsed, ms | Whole elapsed, ms |
|---|---:|---:|---:|---:|---:|---:|
| Generated CA trace | 33,866 | 6,118,673 | 14.076 | 148.768 | 63.604 | 274.447 |
| Residual edit | 26,806 | 6,119,607 | 25.360 | 143.985 | 69.113 | 257.161 |
| Exact residual repeat | 8,502 | 8,506 | 6.186 | 60.867 | 16.726 | 72.725 |
| Opaque edit | 26,846 | 28,450,629 | 19.673 | 425.747 | 61.784 | 866.583 |
| Unrelated opaque object | 28,452,497 | 28,450,458 | 467.728 | 438.690 | 926.554 | 889.754 |
| Schema fit | 13,928,035 | 14,082,317 | 377.904 | 281.673 | 672.841 | 533.079 |
| Schema drift | 13,986,139 | 14,082,766 | 387.399 | 279.707 | 644.738 | 518.932 |

The generated and edited cases show the value of shared state and exact residuals
through the real MVM boundary. Repeats save sender work even though Whole also
uses receiver presence and already sends a short exchange. An exact-repeat
receipt may name `Raw`; it does not mean the whole raw object crossed the stream.
Canonical envelope overhead makes these repeats roughly 8.5 kB, not the much
smaller application-only count from the gateway campaign.

The losses remain material. Schema fit/drift save only **1.11% / 0.69%** bytes
while costing **34.16% / 38.50%** more sender CPU and **28.18% / 24.24%** more elapsed
by paired comparison. Exhaustive unrelated sender CPU is **5.01%** above Whole;
CPU-conscious is approximately **0.24% below Whole** for that operation in this
small dose. Fresh canonical envelope identities can vary encoded envelope lengths,
so tiny cross-policy stream differences are not attributed to codec improvement.

All 120 matched Combined/CPU-conscious operations have identical selected modes,
planned application bytes and object identities. At the smaller two-guest tier,
CPU-conscious unrelated transfers still cost **6.64% more sender CPU and 18.49%
more elapsed** than Whole by paired medians. Its trial-zero complete session is
24.42% slower than Whole; that observation is retained. All 72 campaign boundary
snapshots show zero compiler processes, with one-minute load 2.27–3.96, but those
snapshots do not establish an exclusive host or rule out mid-session contention.

Three trials on a shared host are descriptive. All observations are retained;
no statistical significance, p95, isolated causal explanation for timing
variation, or general workload performance guarantee is claimed.

## Correctness, retention and next boundary

All 33 scoped live tests and strict Clippy passed. An authenticated socket is
deliberately interrupted during an unrelated object: only the five previously
acknowledged operations exist as publications, no successful completion is
reported, and a fresh socket/reopened endpoint completes ten exact objects.
This socket-level regression is distinct from the actual keyless guest reboot
checks; it does not qualify durable mailbox admission or crash/revocation resume.

The [evidence packet](evidence/2026-11-02-mvm-combined/README.md) contains the exact
source/binary identities, registered order, complete compact raw receipts,
reproducible analysis, observed RED/GREEN and independent reviews. Large binaries
are retained in the existing private Hugging Face artifact store with a verified
fresh download. All 36 disposable payload stores were removed only after exact
acceptance; no persistent VM image is required by these results.

The immediate implementation milestone is complete. The next deployment boundary
is the same current-core workload between two reachable hosts over an actual WAN.
Production MVM mailbox lifecycle/transport binding, transparent network-stream
integration, and FIPS/CUI qualification remain separate work.
