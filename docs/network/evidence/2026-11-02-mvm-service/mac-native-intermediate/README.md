# Intermediate macOS/HVF canonical mailbox results

**All four local native sessions completed successfully.** These records cover
one guest connected to a host receiver, and two guests connected through an
opaque host relay. They are one trial per arm/topology at the **1 MiB workload
tier**, using production source `333f8e836b60442fe1621d4455c9bc8ac63af2b0` and
canonical MVM fork `5766754fbaae59dbee548f968716a514d3a796c1`.

This is intermediate local HVF evidence, **not WAN, fleet qualification, final
MVM delivery, FIPS validation, CUI certification, or an energy measurement**.
The later MVM dependency/descriptor-lifecycle correction requires its own fresh
run and identity. Do not pool these results with that run or with the unshaped
[Linux intermediate pilot](../linux-native-intermediate/README.md).

The original captured platform is [macOS 27.0, build 26A5378n](os.txt), using the
ARM64 native HVF host and a Linux ARM64 guest. All four runs selected the local
`experimental64` mailbox profile: **67,108,864 B/s (64 MiB/s), 8,388,608 B burst
(8 MiB)**, with `mailbox_rate_enforced: true` in the records. Each logical session
is smaller than that burst; this does not demonstrate sustained line rate or WAN
behavior. There was no WAN delay/loss/jitter shaping in this pilot.

## What was transferred

Both arms transfer the same eight primary payloads, each exactly **1,047,202 B**:
generated trace, sparse residual, repeat, unrelated opaque bytes, opaque edit,
opaque-edit repeat, schema-fit, and schema-drift. Their logical publication
lengths total **8,377,616 B** per arm, including the two repeated payloads.

Combined CPU-conscious selection additionally publishes and pays for a **3,382 B
runtime checkpoint** and a **48 B schema**. Its ten logical publications total
8,381,046 B. Whole-mixed has eight publications/six distinct object IDs;
Combined has ten publications/eight distinct IDs. Repeated publication and
readback events are not counted as distinct stored objects.

The comparator is prepared whole-object zstd with raw fallback and the same
verified warm-object probe. It is not an always-resend baseline: its repeat
operations also avoid sending a complete object. The persistent receiver,
canonical mailbox service, encryption, receipts, framing, and exact durable
readback requirements are shared by both arms.

## Recorded session totals

Every value below is the original integer from the linked `result.json`.
All time values are **microseconds**, and all byte values are **bytes**.

| Complete session metric | One guest: whole zstd | One guest: Combined CPU | Two guests: whole zstd | Two guests: Combined CPU |
|---|---:|---:|---:|---:|
| Original record | [result](one-whole-mixed-1048576/result.json) | [result](one-combined-cpu-1048576/result.json) | [result](two-whole-mixed-1048576/result.json) | [result](two-combined-cpu-1048576/result.json) |
| Encrypted envelopes + framing, both directions, logical transfer | 7,346,926 | 4,929,588 | 7,352,945 | 4,941,951 |
| Physical vsock-stream byte accounting | 7,346,926 | 4,929,588 | 14,705,890 | 9,883,902 |
| Application Calls, including initial Status probe | 73 | 61 | 73 | 61 |
| Sender envelopes sent / received | 146 / 146 | 122 / 122 | 146 / 146 | 122 / 122 |
| Sender setup + operation CPU | 659,628 | 581,649 | 632,015 | 514,408 |
| Receiver setup + processing CPU | 397,834 | 344,329 | 500,943 | 413,119 |
| Sender setup + operation elapsed | 1,215,152 | 1,176,241 | 1,070,964 | 862,987 |
| Receiver setup + processing elapsed | 675,770 | 691,834 | 536,411 | 433,968 |
| Complete lifecycle elapsed | 1,557,040 | 1,408,039 | 1,615,856 | 1,388,788 |
| Orchestrator process CPU, separate accounting domain | 443,490 | 391,722 | 33,853 | 24,902 |
| Receiver exact reopen reported | true | true | true | true |
| Logical publications / distinct object IDs | 8 / 6 | 10 / 8 | 8 / 6 | 10 / 8 |

Compared within each topology, Combined used **32.90% fewer logical bytes** in
the one-guest session and **32.79% fewer** in the two-guest session. Recorded
sender CPU fell **11.82% / 18.61%**, and receiver processing CPU fell
**13.45% / 17.53%**, respectively. Sender elapsed fell **3.20% / 19.42%**;
lifecycle elapsed fell **9.57% / 14.05%**. One-guest receiver processing elapsed
increased from 675,770 to 691,834 us, so the latency result is not uniformly
positive across every measured field. These are individual observations, not
latency distributions, confidence intervals, or general compression claims.

Elapsed already includes computation: **do not add CPU time to elapsed time**.
Sender timing includes setup/codec preparation and operation work. Receiver
processing excludes socket idle waits and independent readback; its elapsed
counter is not the end-to-end transfer duration. Lifecycle includes VM launch,
session orchestration, and the recovery boot where present. Guest process CPU,
host receiver/orchestrator CPU, and enclosing VMM/worker CPU are distinct or
overlapping domains; do not sum them as physical CPU cost or energy savings.
Full worker measurements remain in the original records.

The logical byte figure counts both directions once. Two-guest traffic crosses
two vsock links through the relay, so physical stream accounting is twice the
logical figure. Totals include canonical encrypted envelopes, four-byte frame
prefixes, readiness traffic, requests, Replies, and both destination receipts.
They exclude virtio control/credit headers, boot assets, and the separate exit
service. They are not NIC/IP/WAN wire measurements.

## Exact recovery and receipt evidence

The [derived summary](summary.json) checks each session's `ok`, canonical service
profile, matching endpoint Call counts, matching byte counters, and exactly two
sent/two received encrypted envelopes per Call at each endpoint. This is the
four-envelope Call → Call receipt → Reply → Reply receipt lifecycle. The native
consumer only finishes after its canonical receipt trackers verify admission
and its application Reply is separately admitted; no receipt-of-receipt is sent.
Raw signed receipts are not serialized into these records, so these archived
JSON files do **not** permit independent offline signature re-verification.

Both two-guest records contain **three successful `guest_done` boots**: sender,
receiver, and a subsequent recovery guest. Recovery `guest_ready` reports
`role: verify`; its disk mount reports `read_only: true`. The whole-mixed
recovery records **eight** `verified` events and Combined records **ten**, all
with `exact: true` and `read_only: true`. The recovery role is the source-pinned
keyless path: no mailbox material/socket is provisioned to that verifier.
Private bootstraps are intentionally not archived here.

The first receiver also independently reopens and compares the published bytes
before reporting `exact_reopened: true`. One-guest sessions perform this check on
the host; they have one guest boot each and no recovery guest. Across these four
sessions there are **eight successful guest boots**, of which two are recovery
boots. HVF reports `workload_exit_code: null` here; this report does not substitute
an invented zero exit code. Worker `ok`, successful `guest_done`, and the recorded
clean power-off contract are the observed HVF completion evidence.

## Mechanisms exercised

The Combined sender-operation records in both topologies select `RuntimeV1` for
the generated trace, `SparseV1` for the residual and opaque edit, and
`SchemaZstdV1` for schema-fit/drift. Thus the checkpoint-driven ternary/
Kuramoto/CA runtime path, verified warm objects, sparse changes, and schema
compression are active in the native mailbox consumer. This is lossless
deterministic fixture reconstruction; it does not establish identical physical
time across distributed machines.

For example, these are the **two-guest Combined** per-operation logical wire
bytes, including both receipt directions and envelope/framing overhead:

| Operation | Recorded mode / warm probe | Logical bytes |
|---|---|---:|
| Checkpoint setup | Raw | 67,549 |
| Schema setup | Raw | 61,556 |
| Generated trace | RuntimeV1 | 82,072 |
| Sparse residual | SparseV1 | 62,930 |
| Exact repeat | Raw label; verified warm probe = true | 20,554 |
| Unrelated opaque object | Raw | 2,270,700 |
| Opaque edit | SparseV1 | 62,999 |
| Opaque edit repeat | Raw label; verified warm probe = true | 20,499 |
| Schema-fit | SchemaZstdV1 | 1,134,339 |
| Schema-drift | SchemaZstdV1 | 1,138,319 |

The Raw label on a successful warm probe does not mean that a raw body crossed
the link. The unrelated-object row also exposes cold-envelope expansion rather
than hiding it behind the favorable session total. Initial Status/setup records
are separate, so this operation table is not the entire session total.

## Identity and archive verification

The original [binary manifest](binaries.sha256) records the signed host as
`efb16a45b1ee3a888916f3cfbcda89ffffcc7678ec0e5200ffec38da079d4216`, guest as
`3bbb1bcfba69c62e9b7637ec45256dcaf51d97211a6c040b06b920554887a6c7`, and kernel as
`0f97f109154a173352c07db223ecff48d44b38514b500f39cd9897701dbac0b6`. Paths in that
manifest identify the original local files, not vendored binaries.

`result.json` and `host-events.jsonl` are byte-for-byte copies of the original
four run records. [SHA256SUMS](SHA256SUMS) covers this archive's records and
metadata. [source-files.sha256](source-files.sha256) hashes the pinned Git objects
for the live/adapter/transfer/gateway/object-store crates at source `333f8e8`,
not later working-tree changes. The deterministic uncompressed source archive
hash in [source-archive.sha256](source-archive.sha256) is reproducible with:

```sh
git archive --format=tar 333f8e836b60442fe1621d4455c9bc8ac63af2b0 \
  crates/spiralcore-mvm-live crates/spiralcore-mvm-adapter \
  crates/spiralcore-transfer crates/spiralcore-gateway-core \
  crates/uor-r4-object-store | shasum -a 256
```

To verify this directory's archived records, run `shasum -a 256 -c SHA256SUMS`
from this directory. No disks, caches, private MVM homes, bootstraps, kernels,
or executable binaries were copied into this report.
