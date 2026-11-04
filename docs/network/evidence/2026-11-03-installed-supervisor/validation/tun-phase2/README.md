# Bounded capture contrast: original gap not reproduced

Two predeclared Linux namespace cells used the original artifact's frozen
`spiralcore-gateway-probe`, SHA-256
`5da4bc45b4a0e2cefac96bfb16c1c65828c648bfc3d74962e64f11642555b340`.
No rebuild or campaign retry occurred. The original PR71 run41757732453 ZIP
remains retained at `/tmp/spiralcore-installed-supervisor-20261103/validation/tun-41757732453.zip`
(SHA-256 `7bb851a11779915f9cd9d64ee45513946c2a24668d73ca70b665954949ffabd3`).
That run failed its strict capture gate: pcap/captured34659,
received-by-filter34660, dropped0, despite exact application receipts.

Here the same finite32,768-byte pattern fixture, stop-and-wait UDP probe,
fresh owned veth namespaces, port4433 filter, snaplen96, buffer4096,
`-U`, and shutdown sequence (0.3s quiet, SIGUSR1, 0.3s quiet, SIGINT)
were used in both cells. The sole capture option difference was
`--immediate-mode` in the second cell. Both passed the unchanged predicate:
other_packets0, pcap_packets>0, pcap=captured=received, dropped0.

| Cell | Pcap packets | Captured | Received | Dropped | Exact endpoints |
| --- | ---: | ---: | ---: | ---: | --- |
| default | 68 | 68 | 68 | 0 | both true |
| immediate | 68 | 68 | 68 | 0 | both true |

Each capture contained66,112 UDP payload bytes across both directions;
application bytes were32,768, with zero probe retries. These are separate
domains. Probe JSON retains its actual per-process CPU and wall scopes;
this is a diagnostic component fixture, not a throughput comparison,
gateway campaign, codec result, VM result, or WAN qualification.

The original one-packet gap is **NOT_REPRODUCED** by this bounded contrast.
These cells do not establish that immediate mode fixes the original campaign.
No capture gate was relaxed and no driver/product source was changed.
Both owned namespaces were deleted; host IPv4/IPv6 route hashes match
before/after. The executed script and compact receipts are adjacent.
Raw pcaps, original probe, and fixture remain in the owned remote directory
`/var/tmp/spiralcore-tun-capture-session-debug-20261103`; no payload/binary
was copied into this packet.
