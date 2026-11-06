# Same-VM installed mailbox Recovery V2 on Firecracker

One signed Linux/Firecracker development guest completed the installed
mailbox round trip on port 5257. It verified the first Reply, held its
positive ACK, closed the original socket at a frame boundary, and redialed
from the same guest workload. The original resident authenticated the
replacement against the signed enrollment, replayed the exact original
cached Reply, and verified the held ACK against the original tracker. The
recovered socket then carried an exact Cast and its verified receipt.

The guest sent 65,023 Call bytes. The Reply reproduced all 65,023 bytes.
The Cast carried 23 bytes. There was one Call on the first carrier and no
Call on the second. Endpoint counters agree in each direction.

| Carrier | Guest to host | Host to guest |
| --- | ---: | ---: |
| First, including framing and authentication | 120,932 B | 126,117 B |
| Replacement, including framing and authentication | 10,330 B | 126,131 B |
| Both carriers | 131,262 B | 252,248 B |

The guest used 9.711 ms of thread CPU over a 17.592 ms exchange. The host
worker used 8.824 ms of thread CPU over 17.111 ms. The per-guest wall time
through retirement was 3.062 s. The whole supervised run took 3.330 s.
The original mailbox and bridge sockets were absent after retirement.
The guest count and staging disk charge returned to zero. The separately
retained runtime cache remained accounted for at 125,870,080 bytes.

Apply [the witness patch](witness.patch) to the composed SpiralCore candidate
and [the Hostd admission patch](hostd-v2-admission.patch) to the composed MVM
SDK. Hostd's V2 callback defaults to refusal. The signed native witness
supplies the original staged resident grant. The locked Linux release suite
passed all seven focused witness tests. The static guest build and signed
one-guest native test both exited zero. Independent scoped Code, Rust, and
Security review passed. The [native result](native-verified.json),
[validation](validation.txt), and [input pins](input-pins.json) retain
source and executable identities.

The development guest applies the canonical Vsock filter in its pinned
private entrypoint because the older guest agent lacks the release
enforcement. The SDK's guest redial still uses a blocking connect. The
original exchange deadline bounds authentication and I/O once it returns.
The legacy development fixture now shares one exchange timeout where it
previously timed each operation. This checkpoint establishes same-VM
recovery in the development lane. Production enrollment and guest-agent
enforcement, hard dial bounds, cross-host WAN, and the upstream PR remain
separate gates.
