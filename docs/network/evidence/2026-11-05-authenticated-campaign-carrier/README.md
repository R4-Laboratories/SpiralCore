# Authenticated mailbox carrier handoff to the object campaign

The campaign can consume the carrier established by MVM's existing V2
pin authentication. The internal carrier keeps the exact bootstrap used
to establish its route, along with the stream and cancellation token.
The installed path uses that carrier directly. The ordinary raw-socket
paths keep their prior handshake and setup order. The receiver borrows
the existing service, checks its budget, and opens the object store under
the supplied limits.

On Wick, an authenticated Unix socket pair used the guest's nonblocking
socket adapter and a resident-owned receiver service. One campaign
completed 20 Calls and reopened all seven publications for exact byte
comparison. The publications totaled 851 bytes, including small fixtures
that intentionally exercise control traffic and repeat cases. A wrong
enrolled host pin prevented the guest from acquiring a carrier. The
ordinary campaign regression suite passed all nine tests, including its
existing 16 MiB case.

| Measurement | Sender | Receiver |
|---|---:|---:|
| Application frames sent | 40 | 40 |
| Bytes sent, including framing and authentication | 212,602 | 194,533 |
| Campaign wall interval after local authentication | 91.502 ms | 55.101 ms |
| Process CPU observed within that interval | 106.097 ms | 74.854 ms |

The directional counters reconcile exactly. The two CPU windows overlap
in one process, so they cannot be summed or interpreted as separate
endpoint CPU. Each interval starts after that endpoint's authentication
and includes campaign setup. There is no measured WAN, codec win, or
native workload from this component.

The native witness still has to supply the V2 pin and bootstrap from the
same signed enrollment, and the broker and service from the same resident
job. The next installed step must explicitly fund the campaign's object
workspace and sustained broker tier, then run within the admitted
lifetime through real retention expiry.

[Original service ownership evidence](../2026-11-05-sustained-original-service/README.md)
