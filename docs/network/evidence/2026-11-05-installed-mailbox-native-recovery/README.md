# Sequential native Firecracker mailbox recovery

The same installed resident on Wick admitted two synthetic Linux x86_64
guests in succession. Each used one vCPU, 1 GiB and native AF_VSOCK port 5257.
At most one guest was live. Firecracker configured no guest network interface.

The first guest verified the encrypted acceptance receipt for its exact
65,023-byte Call, confirmed the Call was pending, and disconnected before
any Reply. The host verified the Call and observed EOF between frames with
one complete frame in each direction. It checked that neither a partial
prefix nor a partial payload was consumed. The original resident retired the
first guest and verified zero live guest, guest memory and staging charges.
Only then did it prepare a fresh signed image and admit the second guest.
The second verified the full 65,023-byte Call and Reply, 23-byte Cast and
encrypted receipts, followed by original-owner retirement.

| Measured quantity | First guest | Second guest |
| --- | ---: | ---: |
| Guest to host carrier payload | 120,159 B | 129,712 B |
| Host to guest carrier payload | 5,858 B | 131,038 B |
| Guest / host exchange CPU | 6.586 / 3.214 ms | 11.292 / 6.583 ms |
| Guest / host exchange elapsed | 10.371 / 10.364 ms | 16.115 / 15.640 ms |
| Guest phase including preparation and retirement | 2.884 s | 3.290 s |
| Resident cache charge after retirement | 125,870,080 B | 251,731,968 B |

Firecracker port-5257 data-packet payload sums matched the endpoint
envelope-plus-framing counters in both directions for both guests. They
exclude virtio/link overhead and control packets. Per-endpoint timings
overlap. The resident completed in 6.264 s including both lifecycles and
verified final state. Runtime generations remain accounted as retained
storage. Both socket paths and staging were absent after both runs.

The local regression was RED when the intended close mode continued the
full exchange. GREEN passed all six local checks and the composed host
controls. The native runner required distinct grant serials, signed bundle
hashes and enrollment commitments. The retained input set rehashed exactly.

This result covers a deliberate frame-boundary disconnect after an
authenticated Call receipt. It uses the private development entrypoint
documented in the [previous native result](../2026-11-05-installed-mailbox-native-development).
Release guest-agent enforcement, abrupt process or host death, final-ACK
recovery, sustained sessions, concurrent capacity and real WAN remain open.
