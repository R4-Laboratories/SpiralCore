# Shared Rust gateway and Linux TUN witness — 2026-11-01

The implementation now provides one shared Rust object/packet core, a Linux TUN
adapter and mutually authenticated paired QUIC gateways. The MVM campaign uses
the same object sender facade. This is a bounded host-to-host IP tunnel; general
Internet egress/NAT, MVM FlowMux interception and transparent warm TCP-stream
reuse are not implemented by this milestone.

## What is shared

`spiralcore-gateway-core::object` reuses the existing raw, prepared whole-object
zstd and warm-object sender. Its context checks retain tenant/recipient/epoch
scoping; authorization still belongs to the adapter. MVM continues using its
canonical encrypted mailbox/vsock channel and existing policy boundaries.

`spiralcore-gateway-core::packet` provides independent Raw or prepared zstd plus
literal fallback for IP packets. Each packet retains its exact length and hash.
It supports a 1280-byte inner MTU with at most two QUIC datagrams, bounded fragment
storage, expiry and replay handling. Losing a packet does not invalidate a later
packet's compression base. There are no cross-packet bases in this profile.

The Linux service attaches an existing TUN interface, validates exact source and
destination allowlists, and forwards to one authenticated peer. The peer restores
and validates IP bytes before injection. Standard rustls certificate validation,
mutual authentication, server-name verification and exact leaf pins all apply;
0-RTT, resumption and reliable QUIC streams are disabled. Configuration and
reproducible commands are in the [gateway README](../../crates/spiralcore-gateway/README.md).

## Verification and MVM continuity

Retained native runs establish four core tests, eleven portable gateway tests and
three MVM campaign tests. They include real TCP/UDP probe exchange, mutual-TLS
negatives, packet corruption/reordering/replay, resource bounds, exact durable
object recovery and scope refusal. Linux test targets additionally cross-compile;
that alone is not a claim of TUN execution. All raw RED/build-failure outputs are
retained alongside successful receipts.

The actual native HVF smoke executed two MVM guests for each of Warm and Whole at
1 MiB, followed by a keyless recovery guest. All **18 objects** (11 Warm, 7 Whole)
matched after reopening and again during recovery. Newly owned backing disks
were removed after verification. Source and binary hashes, full receipts and a
compact summary are in [the evidence directory](evidence/2026-11-01-tun-gateway/).

This was one descriptive pair on a shared development host, not a new speed
qualification. Warm and Whole have different full-campaign operation counts.
For the common exact-repeat operation, the observed sender CPU/wall times were
1.482/3.218 ms for Warm and 4.894/7.362 ms for Whole. Both receivers already held
the object: logical authenticated channel traffic was 8,523 and 8,520 bytes,
respectively. This preserves the existing sender-work advantage without claiming
a new repeat wire-byte advantage. The physical two-guest vsock path traverses
that logical channel twice; it is not WAN traffic.

## Paired TUN measurement

The committed registration fixes three Raw/zstd pairs for each of IPv4 and IPv6:
12 persistent gateway sessions, four application flows per session, 48 flows in
all. Each session sends 16 MiB patterned TCP, 16 MiB opaque TCP, 1 MiB patterned
UDP and 1 MiB opaque UDP. UDP uses a bounded application echo/receipt protocol;
its return payloads and retries are part of the measured traffic.

The runner uses a private veth underlay and two network namespaces on one Linux
host. It provisions only namespace-local peer routes. It records offload settings
and verifies that host routes remain unchanged. This is an actual TUN/TCP/UDP
execution test, not a WAN impairment experiment or a two-physical-host claim.

Per-flow client completion includes connection/transfer and exactness receipt.
Gateway full-process CPU and capture bytes cover the whole four-flow persistent
session, including setup. Codec CPU is a subset of process CPU, not an extra
quantity to add. CPU and wall time overlap; no send-only time is obtained by
subtracting CPU. Capture counts both directions at one underlay interface from
before authentication through shutdown. IP lengths, UDP payload and Ethernet
record lengths are separate; FCS/preamble/inter-frame gap are excluded.

The [Linux workflow](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41361459962)
passed at source `199475fa8a9b54cd8ea87488b5dc120be2fc5279`. All **48 flows / 408 MiB
of application input** matched exactly at both endpoints. All 12 captures were
complete, with zero capture drops, zero UDP application retries and zero rejected
packets. Host routes remained unchanged. Linux execution also passed 17 core and
gateway tests, strict Clippy and the release build. The runner exposed four
x86-64 CPUs on an AMD EPYC 7763 host. The job ran only the registered campaign
during measurement; the underlying hosted infrastructure remains shared.

Medians of three trials, for each four-flow persistent session:

| Metric | IPv4 Raw | IPv4 zstd | IPv6 Raw | IPv6 zstd |
|---|---:|---:|---:|---:|
| Both-direction captured IP bytes | 44,607,439 | 25,979,028 | 45,673,701 | 26,588,242 |
| Both gateway processes' CPU | 1,158.984 ms | 1,381.064 ms | 1,235.620 ms | 1,438.390 ms |
| All application processes' CPU | 273.663 ms | 247.619 ms | 272.448 ms | 281.163 ms |
| Sum of four client completion intervals | 900.331 ms | 1,069.313 ms | 915.907 ms | 1,082.076 ms |
| Larger gateway startup-to-snapshot wall interval | 2,584.902 ms | 2,769.587 ms | 2,622.717 ms | 2,789.720 ms |

Pairing each trial before taking the median, zstd reduced captured IP bytes by
**41.76% for IPv4 and 41.84% for IPv6**. It increased total gateway CPU by
**15.13% / 16.41%** and summed client completion time by **16.07% / 17.04%**.
These paired percentages are not ratios of the independently computed column
medians. Application CPU is separately reported; it must not be mistaken for the
CPU used by the gateway's codec and transport.

Per-flow client completion medians, including transfer and exactness receipt:

| Application flow | Size | IPv4 Raw | IPv4 zstd | IPv6 Raw | IPv6 zstd |
|---|---:|---:|---:|---:|---:|
| TCP patterned | 16 MiB | 267.981 ms | 291.394 ms | 235.099 ms | 302.219 ms |
| TCP opaque | 16 MiB | 267.585 ms | 333.855 ms | 270.929 ms | 341.889 ms |
| UDP patterned | 1 MiB | 180.583 ms | 209.631 ms | 208.136 ms | 217.941 ms |
| UDP opaque | 1 MiB | 181.100 ms | 208.225 ms | 202.089 ms | 217.627 ms |

Every per-flow paired completion-time median was slower with zstd on this fast
local link. The current result is **a working lossless gateway with substantial
mixed-workload byte reduction, at a local CPU/time cost**. This profile uses
ordinary prepared zstd; it does not demonstrate a new SpiralCore-specific packet
compression advantage over zstd. Existing warm-object and shared-runtime gains
remain on the object API and MVM path. A constrained real WAN test is needed to
establish whether the lower packet byte count offsets this compute cost there.

The gateway wall interval includes harness idle time and UDP server receipt
linger, and ends at the pre-close snapshot. Complete capture includes the later
close/drain traffic; neither interval is represented as a network-only send time.
Wire bytes and gateway CPU cover all four flows together, so they cannot be
attributed to individual patterned/opaque flows from this experiment. These are
three descriptive pairs per IP version, not a p95/statistical acceptance or
physical-energy result.

## Recorded evidence and restoration

[Independent arithmetic and custody verification](evidence/2026-11-01-tun-gateway/independent-analysis.json)
checks 96 exact application receipts, all 862 artifact files, 25 source files,
both frozen executable hashes, and all packet-count/length identities. Compact
raw JSON, command output, negative evidence and source/binary/capture manifests
remain in GitHub under [linux-run](evidence/2026-11-01-tun-gateway/linux-run/).

The exact 21,851,334-byte GitHub artifact ZIP, including full header-only captures
and frozen executables, is retained in the existing private
[Hugging Face dataset](https://huggingface.co/datasets/caseyallard/spiralcore-experiment-artifacts/tree/8074002b3803378e5b655a284b0c2fff44338fc2/2026-11-01).
It was independently downloaded from that immutable revision and verified as
SHA-256 `59238980889273cb9c069f396d8dd273229e81f12e218db5e9b0a1056e49e851`.
See the [storage receipt](evidence/2026-11-01-tun-gateway/artifact-storage.json).

```sh
hf download caseyallard/spiralcore-experiment-artifacts \
  2026-11-01/gateway-41361459962.zip --repo-type dataset \
  --revision 8074002b3803378e5b655a284b0c2fff44338fc2 --local-dir RESTORE
shasum -a 256 RESTORE/2026-11-01/gateway-41361459962.zip
```

Use a fresh destination when extracting the archive. The original checksum
manifest expects its contents under a directory named `gateway-evidence`.
