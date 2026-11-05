# Installed FlowMux TCP across Linux Firecracker and a native macOS peer

This is a direct **private-LAN** qualification. One Linux x86-64
Firecracker MVM on Wick sent its existing 11,600-byte fixture through
the canonical guest proxy and original installed FlowMux endpoint.
A native arm64 macOS Rust peer read the request exactly and echoed it.
The guest independently compared the reply. The original owner then
retired the VM and endpoint, with zero guest and staging reservations.

The peer generated its own TLS key. Wick signed the exact destination
IPv4, port, and SPKI into the original network plan. A separate
authenticated SSH control channel carried a bounded listener offer,
public original launch scope and result. The actual TLS and object
traffic used the direct TCP connection to the signed listener. No
guest NIC was used. The peer used the existing SmallV1, authenticated
scope, shared FlowMux and object-core APIs, then joined its reader,
reopened and exactly verified the setup schema and checkpoint, and
removed its private store.

| Measurement | Final positive run |
|---|---:|
| Exact application bytes, each direction | 11,600 |
| TLS bytes, installed endpoint to peer | 4,321 |
| TLS bytes, peer to installed endpoint | 3,869 |
| Mac TLS setup | 12.103 ms |
| Mac time from TCP accept through verification and cleanup | 170.896 ms |
| Mac application-thread CPU | 8.659 ms |
| Mac process CPU over the accepted exchange | 24.367 ms |
| Wick witness process CPU, including its lifecycle work | 954.441 ms |
| Wick supervised lifecycle | 2.058 s |

The TLS counters include setup and framing on the cross-host socket.
They exclude guest vsock and SSH control bytes. The host witness
process does not encompass every helper/VMM process. This single
repetitive object-stream fixture did not compare raw or tuned zstd.

For the negative control the launcher changed one byte of the public
projected plan nonce. The host's original signed plan and both TLS
pins stayed valid. The Mac reached the shared record-header binding
check after TLS authentication, refused with `InvalidBinding`,
and delivered zero application bytes. The private peer store was
removed. Wick confirmed original native and endpoint retirement,
then reported `Busy` when it attempted full supervisor shutdown.
The negative demonstrates original retirement and retains the
supervisor custody limitation.

The preceding host binary refused the new remote mode. The first
cross-host attempt retired safely after a flattened JSON `u128`
result failed to deserialize. The exact nested-report successor
passed both controls. Ten Wick mailbox regressions and two Mac
scope-policy tests also passed.

Apply the Wick delta to the
[composed installed witness](../2026-11-05-installed-sustained-finish-redial/README.md).
The Mac source is a qualification example for
[shared-core revision 1025d87d](https://github.com/R4-Laboratories/SpiralCore/commit/1025d87d048b15cefd483b4c11c70a6e029cb29c).
Apply the manifest delta and put the example in the FlowMux crate's
examples directory. Its stdin accepts one launch projection, bound to
the control ID from its initial stdout offer. It writes a compact
terminal record. The fixture requires a fresh private workspace, an
explicit listener and an outer process supervisor. Launcher hashes
identify the private test drivers. The public packet retains Rust
source, hashes and sanitized receipts.

The executed scope covers a native Mac cooperating peer and a Linux
MVM guest. Internet WAN, matched codec baselines, native macOS guest
execution for this path, UDP, concurrency and production security
qualification remain open.
