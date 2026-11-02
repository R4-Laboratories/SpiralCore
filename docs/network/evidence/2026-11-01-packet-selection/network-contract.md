# Packet selection: bounded network comparison

Status: registered, not run. `network-registration.json` fixes the dose before
implementation or traffic. The reviewed PR workflow is now launch-ready; its
source/binary freeze checks still run before traffic. The four mode spellings
are confirmed. Adaptive is an experimental arm; a
successful experiment does not automatically make it the service default.

The primary comparison is 24 IPv4 persistent peer sessions: three matched trials
of Raw, legacy zstd, fast zstd and Adaptive under an unshaped local link and a
bidirectionally shaped 80 ms RTT / 10 Mbit/s link. Every session sends 16 MiB
patterned TCP, 16 MiB opaque TCP, the same 16 MiB patterned TCP again, then a
4 KiB UDP exact-echo smoke. The three TCP objects share one persistent TCP
connection so that the final pattern actually tests the same selector flow
state after opaque traffic. Eight separate IPv6 smoke sessions use 1 MiB TCP
fixtures and the same 4 KiB UDP dose. All 32 sessions retain exact sender and
receiver receipts. This totals 128 application transfers on 64 transport flows
and 1,176.125 MiB of application input. This persistent-socket correction was
registered before any network execution; the traffic dose is unchanged.

Two endpoint namespaces connect through a third owned router namespace. Its two
egresses apply 40 ms delay and 10 Mbit/s independently with a 256-packet queue
limit. Shaping in this middle
namespace avoids placing delay on the originating TCP endpoint. Netem is subject
to kernel timer and queuing effects, so the campaign checks observed RTT and
receiver goodput in both directions, preserves `tc -s -j` state, and stops if
calibration falls outside the registered tolerance. The iproute2 documentation
explains these limitations and the sender-side TSQ concern:
[upstream netem manual](https://github.com/iproute2/iproute2/blob/main/man/man8/tc-netem.8).

The finite queue may drop packets under congestion even though no random loss is
configured. Before launch, acceptance was amended to retain queue drops and
reassembly expiry as measured delivery costs rather than invalidate an otherwise
exact TCP transfer. Capture-kernel drops still invalidate complete wire totals.
Malformed frames, authentication/policy failures and byte mismatches still fail.
Reassembly capacity exhaustion is recorded separately as a resource-limit
failure; the current service also includes its decoder `Error::Limit` in
`invalid_frames`. Queue occupancy must drain before final capture acceptance.

Calibration uses bounded ICMP and iperf3 before the gateway capture. Fresh qdiscs
separate calibration counters from measurement. A single endpoint underlay
interface captures both directions from before QUIC authentication through clean
close/drain. Report captured IP bytes and Ethernet record lengths separately;
this does not measure FCS, preamble or inter-frame gap. Captures and gateway CPU
cover the whole mixed session, so per-flow wire/codec claims are not available.

The first TCP object's client completion includes connection, transfer and the
exactness receipt. Later objects measure transfer and receipt on the same live
connection; they do not include the prior object's interval.
The probe freezes each interval immediately after its exact receipt and defers
all three report hashes/JSON writes until the entire sequence finishes, keeping
report I/O out of the next transfer's peer wait.
Gateway process CPU, application process CPU, codec CPU subsets, gateway active
wall and full harness wall remain separately labeled. Pattern-after-opaque is a
selector-recovery check, not a warm packet cache. The opaque fixture is seeded
incompressible data, not decrypted Internet TLS. Three pairs are descriptive;
negative results and byte/CPU tradeoffs stay in the record.

The existing 12-session TUN workflow remains a bounded regression check. A new
same-repository PR workflow owns this registration, pins the exact PR head and
executed workflow, uses read-only repository permissions and no account secrets,
builds/tests before measurement, and retains failed outputs without automatic
retry. The standard public-repository runners are currently free:
[GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Actual two-host path: feasibility, not execution

Two standard GitHub jobs would provide separate VMs without a new paid account,
but they do not by themselves establish a reachable UDP peer pair. Direct inbound
reachability, NAT behavior and geographic separation have not been observed.
Do not replace that missing evidence with a public egress address or label a
same-region pair a constrained WAN. A later bounded UDP reachability probe with
two concurrent jobs can determine feasibility before any throughput campaign;
that probe is outside this registration and has not been dispatched.

The local Mac cannot run the current Linux TUN service. Using it for the identical
gateway would require a supported macOS adapter or a Linux VM, neither part of
this bounded test. Historical HTTP object transfers used Quick Tunnels; those
expose HTTP services, and published non-HTTP Cloudflare TCP uses a WebSocket
client. They are not a direct UDP transport for this QUIC comparison:
[Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/),
[published protocols](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/routing-to-tunnel/protocols/).

The immediate deliverable therefore remains explicitly a **shaped local**
comparison. No account, secret, paid service, host route or local VM is changed
to make an unsupported actual-WAN claim. Captures and frozen binaries use the
existing verified archival process; compact results remain in Git.
