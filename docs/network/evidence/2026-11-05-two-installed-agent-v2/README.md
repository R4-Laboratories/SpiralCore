# Two sequential installed Firecracker Recovery V2 guests

The same installed resident started and retired two actual Linux x86-64
Firecracker guests in sequence. Each had its own signed portable bundle,
mailbox enrollment commitment and grant serial. The booted guest agent
applied the Vsock policy before directly executing the mailbox workload.

Each guest completed an exact 65,023-byte Call and 65,023-byte Reply,
reconnected to the same VM to verify the original encoded Reply and send
its retained final acknowledgment, then completed an exact 23-byte Cast.
Both runs used the dedicated vsock mailbox binding, with no guest NIC.

| Measurement | Guest A | Guest B |
|---|---:|---:|
| Guest → host, both carriers | 131,364 B | 131,249 B |
| Host → guest, both carriers | 252,240 B | 252,232 B |
| Guest / host exchange thread CPU | 9.291 / 7.771 ms | 9.227 / 7.523 ms |
| Guest / host exchange elapsed | 16.827 / 16.383 ms | 16.631 / 16.254 ms |
| Through original retirement | 2.530 s | 2.423 s |
| Managed cache charge after retirement | 125,870,080 B | 251,731,968 B |

The two runs used 767,085 B of mailbox framing, encrypted envelopes and
carrier authentication. Supervised wall time was 5.194 s and the original
host witness process consumed 1.952 s of CPU. The outer host command
reported 192,304 KiB peak RSS, which includes fixture preparation. The
existing per-guest managed reservations were unchanged.

The original owner removed each VM, its mailbox socket and vsock bridge,
and verified zero guest and staging charges before the next enrollment.
Each retained runtime used 107,339,776 B of actual allocated blocks and
stayed separately charged. Both original bundle digests matched their
files after retirement. Final readback found no Firecracker process or
listening socket from the test.

The harness now accepts two sequential V2 guests and refuses to combine
that mode with the separate withheld-reply fault fixture. It still
requires the canonical Firecracker installed lifetime build feature.
The earlier mode refusal, incorrect launcher-relative path and
feature-disabled backend refusal are recorded in the validation notes.

These are two sequential native Dev-lane guests under a private publisher
and one installed resident. The run measures neither simultaneous guest
capacity nor WAN, and uses no matched compression baseline.

[Single-guest installed proof and runtime pins](../2026-11-05-installed-agent-v2/README.md)
