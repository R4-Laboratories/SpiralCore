# Persistent compressed-schema transfers over a real WAN

The SC-OPT-013 sender optimization carries over to the measured WAN deployment.
For matching approximately 1 MiB batches, the optimized sender used **31.67% less
sender CPU** than the source-derived old encoder, winning all thirty paired
trial blocks. The paired sender-plus-receiver-handler CPU reduction was **9.98%**
(27/30 wins), and the paired completion-time reduction was **11.19%** (20/30 wins).
Transferred body bytes were unchanged. All **1,920 objects** passed independent
byte-for-byte comparison after the receiver processes stopped and their stores
were reopened; all **660** baseline/optimized seed-and-target body/mode pairs matched.

Against the existing ordinary-zstd profile, the optimized schema profile sent
**2.28% fewer body bytes** on matching batches but used **48.37% more combined
measured CPU** by paired trial-block ratios. Completion-time medians were nearly
equal. This establishes a useful sender optimization, not a broad speed or energy
advantage over zstd.

## Run and scope

Measured source: `2fcfb0dae9524c073f6aab8982736fb4b9242173`.
[Hosted run 41128434555](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41128434555)
completed its three sequential shards on October 30, 2026, from **21:09:19 to
21:15:36 UTC**. The path was GitHub-hosted Ubuntu/x86-64 Linux (four exposed CPUs,
AMD EPYC 7763) through Cloudflare HTTPS/QUIC to an Apple M1 macOS receiver.
The Mac remained a shared desktop; this was not a dedicated isolated host.
No paid VM was provisioned.

The current source tree and harness were used for both senders, replacing only
`src/schema.rs` with the pinned SC-OPT-012 source for the baseline. The receiver
was identical for all arms, with durable byte publication, optional DAG warming
disabled, and the bounded prepared-codec cache enabled. Each arm/session used a
separate authorized namespace. Thirty paired trial blocks per size used all six
arm orders five times. Each session reused a sender and receiver across ten
distinct targets: four matching, two lexical/order variants, two schema-drift,
and two unrelated binary objects.

Small objects ranged from **210 to 331 bytes**; batch objects ranged from
**1,048,576 to 1,048,903 bytes**, preserving whole records and exact lengths.
There were 1,800 target transfers, 120 actual 169-byte schema artifact seeds,
120 sender preparations, and 180 sessions. The artifact was previously fitted;
it was not fitted again or charged an invented fitting cost on each transfer.
This campaign did not send a single 16 MiB object; the earlier
[16 MiB WAN matrix](broad-wan-results-2026-10-29.md) is separate evidence.

The ordinary-zstd comparator is the existing protocol's level-1 compression in
chunks of at most 61,440 bytes. Compressed schema uses the unchanged level-3
checksummed frame over its value/literal stream. Thus the old/new encoder CPU
comparison isolates the optimization, while the zstd comparison is between these
specific profiles. It does not establish a schema-only advantage over equally
tuned whole-object zstd or every conventional codec.

## Matching batches

These are medians of 120 transfers per row: four matching targets in each of
thirty trial blocks. CPU columns are separately computed medians; their displayed
medians need not add exactly. All times are milliseconds.

| Profile | Both-direction body bytes | Sender CPU | Receiver handler CPU | Combined CPU | Durable elapsed |
|---|---:|---:|---:|---:|---:|
| Old prepared schema | 127,548 | 10.663 | 14.817 | 25.333 | 191.461 |
| Optimized prepared schema | 127,548 | 7.164 | 15.537 | 22.662 | 176.791 |
| Ordinary zstd | 130,507 | 3.168 | 12.205 | 16.057 | 176.194 |

The registered paired comparison first sums the four matching transfers within
each trial, then takes the geometric mean of thirty optimized/comparator ratios.
It is not the ratio of the table's medians and does not treat 120 positions as
120 independent trials.

| Optimized / comparator | Sender CPU ratio | Combined CPU ratio | Elapsed ratio | Elapsed wins | Body-byte ratio |
|---|---:|---:|---:|---:|---:|
| Old encoder | 0.683323 | 0.900241 | 0.888097 | 20/30 | 1.000000 |
| Ordinary zstd | 2.011937 | 1.483656 | 0.970516 | 18/30 | 0.977235 |

Against zstd, the paired elapsed ratio favors schema by 2.95%, while schema's
single-transfer median is 0.34% slower. The distributions are variable: p95
elapsed is 451.586 ms for old schema, 269.830 ms for optimized schema, and
448.069 ms for zstd. These are observations from one route/campaign, not a stable
latency guarantee. On the three matching targets after the first in each session,
optimized schema's median is 175.439 ms versus zstd's 169.569 ms. The first zstd
target also pays its initial connection setup, whereas a schema session's paid
artifact seed precedes its first target. Session totals below include that cost.

The measured exchange medians are 181.254 ms (old schema), 170.130 ms (optimized),
and 173.943 ms (zstd). These intervals include transport waiting and remote
processing. They are **not pure wire-transmission times**. CPU must not be added
to elapsed time: elapsed already includes computation and waiting.

## Paid setup and other workloads

Each row below is the median of thirty complete ten-target sessions, including
actual artifact transmission and prepared sender construction where applicable.
The approximately 1 MiB rows are about 10 MiB of original data across ten objects,
not one large object. The first-use connection cost is included for every profile.

| Object size | Profile | Session body bytes | Combined CPU, ms | Durable session total, ms |
|---|---|---:|---:|---:|
| Small | Old schema | 7,909 | 122.385 | 1,605.855 |
| Small | Optimized schema | 7,909 | 115.344 | 1,588.266 |
| Small | Zstd | 6,734.5 | 109.115 | 1,493.865 |
| ~1 MiB | Old schema | 3,079,510 | 246.154 | 2,504.581 |
| ~1 MiB | Optimized schema | 3,079,510 | 233.328 | 2,402.279 |
| ~1 MiB | Zstd | 3,094,132.5 | 162.718 | 2,050.563 |

The mixed large-object session sends about 0.47% fewer bytes with schema than
zstd, but its median completion total is 17.15% higher. Small schema sessions send
17.44% more bytes than zstd. All small matching targets fall back to raw bytes;
the extra schema setup is not beneficial there. The full small matching paired
sender CPU ratio also regressed slightly versus the old encoder (1.012426).

Large-object controls retain the unfavorable cases. The following are paired
optimized-schema/zstd ratios from thirty trial blocks:

| Workload | Body-byte ratio | Combined CPU ratio | Elapsed ratio | Elapsed wins |
|---|---:|---:|---:|---:|
| Lexical/order variants | 0.920249 | 1.620317 | 1.053487 | 14/30 |
| Schema drift | 1.107303 | 1.458551 | 1.063119 | 10/30 |
| Unrelated binary | 1.000000 | 1.278318 | 1.045949 | 14/30 |

Variants save about 7.98% of body bytes but cost more CPU and elapsed time.
Drift sends about 10.73% more data than zstd. Unrelated objects fall back to raw
with equal body cost but pay failed schema-selection work. The explicit schema
profile currently chooses schema or raw; it does not automatically select the
better ordinary-zstd result on these cases. This makes codec selection and an
equally tuned conventional-compression control the next useful work, before
presenting schema as a general default or proceeding to MVM integration.

## Evidence and limits

[Raw observations, verification receipts and reproduction instructions](evidence/2026-10-30-schema-wan/README.md)
are retained with the [strict summary](evidence/2026-10-30-schema-wan/wan/summary.json).
The hosted candidate passed 300 tests (two historical ignores); the derived
baseline passed its 21 relevant codec tests. Local production line coverage was
90.64% for the WAN module and 93.52% for the CLI, using only current objects.
Independent source/security reviews preceded measurement, with a separate result
consistency review after the campaign.

Receiver telemetry proves 120 decoder preparations, 470 reuse hits and 96
bounded evictions across three stores. Each finished with eight retained entries,
charged 40 MiB. Each shard has exactly 640 admitted requests and one unobserved
ordinal gap from its unauthenticated preflight probe. No measured operation was
retried or discarded. A prior service-launch attempt was cancelled before its
transfer step and is preserved separately.

Byte counts include actual request/reply HTTP bodies, framing and acknowledgments;
they exclude HTTP/TLS/IP/TCP overhead, retransmissions and additional relay-hop
traffic. CPU totals include the sender process and receiver handler, but not
receiver HTTP/socket handling, cloudflared, or Cloudflare processing. Cache growth,
desktop load and WAN variability remain part of this environment. No shaped
80-ms/10-Mbps acceptance, direct-peer path, real application corpus, energy saving,
FIPS qualification, or MVM/vsock integration is claimed by this campaign.
