# SpiralCore paired host gateway

This Linux service attaches an existing Layer 3 TUN interface to one mutually
authenticated SpiralCore peer over QUIC datagrams. Ordinary applications use IP
routes to that interface. The peer verifies and reconstructs each IP packet before
writing it to its TUN interface. The physical NIC carries ordinary UDP/IP QUIC
traffic; no NIC driver changes are needed.

The shared `spiralcore-gateway-core` crate also supplies the object sender used by
the MVM canonical mailbox/vsock integration. Object transfer retains warm-cache
and prepared-codec selection. Packet v1 uses independent Raw or prepared zstd
packets with literal fallback, not cross-packet object references. It does not
implement a reliable QUIC object service or transparently deduplicate encrypted
TCP streams. TLS/QUIC application payloads already encrypted before reaching TUN
usually offer little compression. Neither arbitrary Internet peers nor existing
MVM FlowMux policy are bypassed.

## Build and isolated demonstration

```sh
CARGO_TARGET_DIR="$PWD/target" cargo build --release --locked --manifest-path crates/spiralcore-gateway/Cargo.toml --bins
bash tools/gateway-network/gateway-netns.sh run "$PWD/gateway-results" \
  "$PWD/target/release/spiralcore-gateway" \
  "$PWD/target/release/spiralcore-gateway-probe" \
  "$PWD/docs/network/evidence/2026-11-01-tun-gateway/registration.json"
```

Run from the repository root on Linux, with passwordless sudo available for the
script's scoped namespace operations. The output directory must not exist.
The registered demonstration uses
network namespaces, a private veth underlay, preconfigured MTU-1280 TUN interfaces,
and explicit IPv4/IPv6 peer routes. It never replaces the host default route or
turns on general forwarding/NAT. The service itself does not administer routes.
The same service can run on two Linux machines with an authenticated reachable
UDP path, but the namespace demonstration alone is not a WAN measurement.

## Service configuration

`spiralcore-gateway run CONFIG.json` accepts a strict JSON object:

```json
{
  "role": "listen",
  "bind": "10.203.0.1:4433",
  "peer": "10.203.0.2:4433",
  "server_name": "gateway.test",
  "certificate": "/private/server.der",
  "private_key": "/private/server.key.der",
  "trust_anchor": "/private/ca.der",
  "peer_certificate": "/private/client.der",
  "tun": "sc0",
  "mode": "zstd",
  "local_addresses": ["10.204.0.1", "fd42:5343::1"],
  "peer_addresses": ["10.204.0.2", "fd42:5343::2"],
  "stats_path": "/results/server.json",
  "ready_path": "/results/server-ready.json",
  "duration_secs": 300
}
```

Use `connect` at the other end and swap local/peer addressing and credentials.
The explicit `mode` values are `raw`, `zstd` (the unchanged level-3 whole-object
profile), `fast-zstd` (prepared level-1 packet profile), and experimental `adaptive`
(the fast profile with bounded per-flow compression skipping). Both new modes use
the existing packet wire format and exact decoder. Adaptive may miss compression
opportunities after content changes; it is opt-in and is not universally best for
bytes. The [packet design](../../docs/specs/2026-11-01-packet-selection.md) and
[microbenchmark record](../../docs/network/evidence/2026-11-01-packet-selection/microbenchmark-results.md)
document the measured tradeoffs, including the adverse byte regression.

Certificates must chain to the configured private CA, permit the appropriate TLS
usage and match the configured server name. Each side additionally pins the
other's exact leaf certificate. Private keys are DER PKCS#8 or another rustls
supported DER key format. Rotate credentials by restarting with new configuration;
there is no anonymous mode, resumption or 0-RTT. This implementation is not a
FIPS-validated crypto deployment.

The interface must exist with MTU 1280 before service startup. Linux TUN permission
is required; use a dedicated namespace or an explicitly delegated interface.
Source and destination addresses are checked in both directions. The first
profile permits exact address pairs, not an unrestricted subnet/Internet router.

SIGTERM, Ctrl-C, the configured duration or a normal authenticated peer close
ends the service and writes a JSON receipt. Operational errors remain failures.
Ready is published only after TUN attachment and authenticated peer admission.
Packet hashes and exact lengths guard reconstruction; datagrams may still be
lost/reordered. TCP retains its normal retransmission behavior. Packet fragments
have bounded queues, a replay window and expiry. A received packet is never a
claim of durable object publication or application processing.

## Measurement

`spiralcore-gateway-probe` generates public deterministic fixtures, verifies TCP or
UDP application payloads and summarizes classic Ethernet/IPv4 PCAP captures.
Each paired run compares raw packet forwarding with equally prepared zstd plus
literal fallback over the same authenticated tunnel. The UDP probe supplies its
own bounded stop-and-wait echo/receipt protocol; all that traffic is counted.

Gateway receipts separate codec CPU from full process CPU and include setup as a
subset of full process totals. Application receipts report exact length/hash and
completion time; do not add overlapping CPU/wall intervals. Header-only capture
counts both directions at one underlay interface and includes QUIC authentication,
ACKs and closure. IP bytes, UDP payload bytes and captured Ethernet lengths are
separate; Ethernet FCS, preamble and inter-frame gap are not claimed. Session CPU
and wire bytes cover the persistent sequence, while application times are per
flow. Raw receipts and failed runs belong with the result report.

## Runtime and warm objects

The reliable `spiralcore-gateway-object` service exposes the existing
ternary/Kuramoto/CA checkpoint-recipe-residual and rolling warm-cache paths through
this gateway. See [configuration, measurement and limits](../../docs/network/runtime-object-gateway.md).
It uses a separate authenticated object ALPN; the packet TUN command above does
not silently acquire object/cache semantics.
