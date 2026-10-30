# 16 MiB real-WAN localized updates: SC-WAN-002

The thirty-pair campaign passed its registered HTTP-body and elapsed-time gates.
For a 16 MiB random object with a 32-byte midpoint edit and a verified receiver
base, sparse transfer sent **600 HTTP-body bytes instead of 16,814,026**. Its
paired geometric-mean completion advantage over raw was **8.734144×**; combined
sender plus receiver-handler CPU improved **3.165958×**. All thirty pairs favored
sparse on both elapsed and combined CPU. All **180 seed/update operations** were
reopened and compared byte-for-byte: **3,019,898,880 bytes**, with no discrepancy.

These are real Internet transfers between GitHub Linux runners and an Apple M1
receiver through Cloudflare, using the native Rust implementation. They are not a
shaped 80 ms/10 Mbps test, a direct-peer path, an MVM integration, an energy test,
or a comparison with tuned rsync. The full thirty-pair IP capture claim did not
qualify: the final shard has an unresolved three-packet counter discrepancy.

## Source and execution

Measured source: `f4e5e9afe6f3628ffe82e4553a88111f5bd6d126`.
Campaign ID: `c86dc240664d7e66d840f313d7ebf258812a13f9a5f839f3ab6407dc742189e4`.
The [registered plan](../plans/2026-10-29-cache-profile-and-large-wan.md) and
[evidence packet](evidence/2026-10-29-large-wan/README.md) retain the exact inputs,
source/executable identities, feature trees, raw receipts and analysis.

| Shard / global trials | Real WAN run | Reopened exact operations | Capture qualification |
|---|---|---:|---|
| 0 / 0–9 | [40921351267](https://github.com/R4-Laboratories/SpiralCore/actions/runs/40921351267) | 60 | Complete runner-to-relay leg |
| 1 / 10–19 | [40922575602](https://github.com/R4-Laboratories/SpiralCore/actions/runs/40922575602) | 60 | Complete runner-to-relay leg |
| 2 / 20–29 | [40923669618](https://github.com/R4-Laboratories/SpiralCore/actions/runs/40923669618) | 60 | Incomplete: received/captured counters differ by 3 |

All three use the same source and canonical v3 plan. Each has a fresh receiver
cache and capability, ten paired trials, three arms, and sixty seed/update rows.
Each arm gets its own identical raw seed and authorization scope; every update
target starts absent. All six arm permutations occur five times globally. The
fixture is the recorded xorshift64-v1 byte generator, with independent seeds
derived from the campaign ID, object size and global trial. No performance trial
was discarded, retried or replaced.

The Mac uses the qualified `hardware-sha256` feature. Linux uses default sha2 CPU
dispatch; the three runners identify as AMD EPYC 7763, 9V74 and 7763 respectively.
Rust is pinned to 1.97.1. Cache payload is 960 MiB per shard, below the unchanged
1 GiB quota after publication headers; each namespace retains 32 MiB. The WAN
endpoint's existing resident reservation is 448 MiB, with the existing adapter
batch limits. Observed process RSS snapshots are retained, not called peak RSS.

## Warm update results

Each cell is a midpoint median across thirty observations, except nearest-rank
p95. Times are milliseconds. HTTP bodies count requests and replies; they exclude
HTTP headers, TLS, TCP/IP and relay overhead.

| Arm | HTTP-body bytes | Elapsed median | Elapsed p95 | Sender CPU median | Receiver-handler CPU median | Combined CPU median |
|---|---:|---:|---:|---:|---:|---:|
| Raw full send | 16,814,026 | 2,030.458 | 2,510.218 | 44.250 | 193.188 | 235.594 |
| zstd full send | 16,814,026 | 2,016.728 | 2,523.489 | 48.164 | 189.848 | 237.824 |
| Sparse COPY/LITERAL | 600 | 219.650 | 362.693 | 40.647 | 39.782 | 79.509 |

Combined CPU is summed within each operation before taking the median; it is not
the sum of the two separately displayed medians. The pseudorandom input is poorly
compressible, so zstd does not reduce the full-send body count in this workload.

| Paired baseline / sparse | Elapsed geometric mean | Combined-CPU geometric mean | Elapsed wins | Combined-CPU wins |
|---|---:|---:|---:|---:|
| Raw | 8.734144× | 3.165958× | 30/30 | 30/30 |
| zstd | 8.935309× | 3.109440× | 30/30 | 30/30 |

HTTP-body reduction is **99.99643155%** in every update pair. Sender CPU alone is
a smaller gain: raw/sparse geometric mean **1.099665×**, with 28/30 wins and two
losses. Those losses and every individual timing remain in the raw and aggregate
records. Ratios above are geometric means of matched trial ratios, not ratios of
the displayed marginal medians.

Elapsed starts with ready in-memory inputs, before codec/recipe work, and ends
after validated durable publication acknowledgment and timed retained-state
teardown. It already includes computation, transmission, receiver work and waiting.
Fixture generation is reported separately. CPU must not be added to elapsed or
subtracted to fabricate a network-only duration. A pure network-only send time is
not isolated here.

Receiver CPU covers frame decode/handle/encode and synchronous publication. It
excludes HTTP ingress/authentication before that timer, socket writes, subsequent
buffer destruction, cloudflared and Cloudflare processing. Process-level origin
and tunnel snapshots are separately retained. The CPU sum spans heterogeneous
machines and is neither a whole-system CPU measurement nor a joule measurement.

Raw/zstd still attempt bounded optional DAG construction, while sparse skips it.
The random 16 MiB graph exceeds the node cap, so equal durable-byte readiness does
not imply equal preparation work. The observed CPU gain includes that policy
difference. This is the current authenticated raw/zstd implementation baseline,
not an optimized minimal byte sink or a matched WAN rsync/VCDIFF comparison.

## Include the real cache-seeding cost

Every method sends the same base before its update. These are matched two-delivery
lifecycles, with costs added within each trial before calculating statistics.

| Arm: seed plus update | HTTP-body bytes | Elapsed median, ms | Combined CPU median, ms |
|---|---:|---:|---:|
| Raw | 33,628,052 | 4,049.591 | 472.087 |
| zstd | 33,628,052 | 4,044.087 | 472.452 |
| Sparse | 16,814,626 | 2,151.035 | 319.119 |

Including seeding, sparse saves about **50%** of body bytes and has a paired
elapsed advantage of **1.811394× over raw** and **1.869955× over zstd**. Combined
CPU ratios are **1.529030×** and **1.513717×** respectively. All thirty matched
lifecycles favor sparse on elapsed and combined CPU. Separate sender/receiver
components include losses, retained in the aggregate. Long successor chains and
their amortization were not measured; no projected repeated-update gain is
substituted for this observed seed-plus-one-update result.

## Packet-accounting limit

All capture records use 96-byte snapshots, but byte counts come from IP header
lengths, not the saved prefix size. Independent raw-PCAP replay confirms:

| Shard | Saved packets | Observed campaign IP bytes, both directions | Received-by-filter packets | Kernel-reported drops |
|---|---:|---:|---:|---:|
| 0 | 623,204 | 874,670,762 | 623,204 | 0 |
| 1 | 622,210 | 874,481,754 | 622,210 | 0 |
| 2 | 622,806 | 874,556,220 | 622,809 | 0 |

The separately labeled, descriptive twenty-pair subset from complete shards 0–1
(global trials 0–19) has median captured IP bytes of **17,488,180.5 raw**,
**17,489,477.5 zstd** and **5,907 sparse**. Its raw/sparse paired geometric mean is
**2,956.08755×**, with **99.96622044% median per-pair reduction**. This includes
observed TCP/IP/TLS/HTTP overhead on that leg. It is not the registered thirty-pair
IP result; the final ten pairs remain in every body, elapsed and CPU comparison.

The final discrepancy is unresolved; zero reported kernel drops does not resolve
it. No combined thirty-pair IP-byte distribution or reduction is qualified. The
body/CPU/elapsed analysis retains all thirty pairs, including shard 2. All observed
capture totals are preserved. Any descriptive IP subset from complete shards is
labeled with its selected global trials and is not the registered full-campaign
IP claim. Captures cover only runner-to-Cloudflare, excluding relay-to-Mac traffic.
Independent PCAP replay validates campaign totals; per-operation connection
ownership remains tied to the reviewed production parser's observed-SYN rules.

## Qualification and disposition

Before WAN timing, the native hardware build passed 128 routine transfer tests,
one doctest, formatting and Clippy with warnings denied. The expensive first-pair
release test was separately executed, and a full sixty-operation hardware local
HTTP shard reopened exactly (1,006,632,960 bytes). Coverage on twelve production
source files is 89.3488% lines, 91.0788% functions and 86.1250% regions; branch
coverage was not collected. An initial coverage-tool PATH discovery failure
produced no coverage and is recorded separately from the successful rerun.

The independent Rust review regenerated canonical fixture ObjectIds, seeds, scopes
and order, bound producer/analysis identities and per-shard reopen receipts,
checked 41,490 request and 41,490 reply frames, and recomputed the shared statistical
records. Review sources, checks and negative cases are retained with the evidence.
Diagnostic console debris is removed after structured receipts are preserved.

No cache-scan optimization was introduced. The separate
[SC-OPT-004 attribution](cache-profile-results-2026-10-29.md) locates populated-cache
CPU in quota scans while preserving the existing filesystem observation contract.
The larger WAN batch delivers repeated 16 MiB evidence on the current native
engine. Larger edits, insertions/deletions at this size, long-lived cache evolution,
64 MiB batched transfers, direct/shaped WAN, conventional delta WAN controls and
energy measurements remain unqualified. MVM and security qualification remain
separate work.

Temporary receivers/tunnels and their local capabilities were stopped/removed;
the repository's six experiment variables and token secret were deleted. Public
GitHub standard runners were used, with no paid VM provisioned. The raw capture
files are retained locally and identified by hashes in the compact repository
packet; hosted artifact retention is one day.
