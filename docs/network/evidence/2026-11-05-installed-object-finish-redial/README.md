# Installed object campaign across a Finish receipt redial

A signed Linux x86-64 Firecracker guest withheld the first successful
Finish Reply's admission receipt before invoking the send. It closed
the original carrier at a frame boundary. The original resident
accepted one replacement through its enrolled V2 acceptor and
reauthenticated the same peer. It replayed the exact original
encrypted Reply with the original tracker identity. The guest
compared that envelope byte for byte, sent the original prepared
receipt, and continued the campaign.

The fixture uses separate bounded initial authentication and Finish
replacement windows. Replacement is capped by the original
operation and receipt authority and an eight-second window. The
positive path keeps the original Session, ordering scope, broker,
object endpoint and store. The host recorded one cached Reply replay
and the guest recorded zero Call retries. A truncated old frame
refused replacement in the focused negative test.

The campaign completed 20 canonical Calls and seven publications.
After original VM retirement, the host reopened and byte-compared
851 logical object bytes, removed the private store and verified its
absence. No Firecracker process remained. Guest and staging
reservations returned to zero.

| Measurement | Guest to host | Host to guest |
|---|---:|---:|
| Original carrier bytes with authentication and framing | 37,133 | 39,754 |
| Replacement carrier bytes with authentication and framing | 175,678 | 160,448 |
| Total bytes | 212,811 | 200,202 |
| Application frames, both carriers | 40 | 41 |

Both carriers' byte and frame counts reconcile. The original cut was
after seven guest frames and eight host frames. Guest campaign CPU
was 61.880 ms and host campaign CPU was 54.098 ms. Post-authentication
campaign wall counters were 140.154 ms guest and 95.686 ms host.
The supervised run took 2.820 s. Outer process peak RSS was
190,524 KiB, including fixture preparation.

The final protected host suite ran 201 passing tests, with 31
ignored. The signed native run executed separately and exited zero.
The RED and intervening test-build corrections are recorded in
validation.txt. Apply the source patch to the
[composed sustained qualification source](../2026-11-05-installed-sustained-campaign/README.md).

This one-guest Dev fixture combines an object campaign with
pre-send receipt loss and successful redial. The previous 2,311-Call
run crossed retention on one carrier. A combined sustained run,
loss during an uncertain partial receipt write, cross-host WAN,
concurrency and a production security profile require separate
qualification.
