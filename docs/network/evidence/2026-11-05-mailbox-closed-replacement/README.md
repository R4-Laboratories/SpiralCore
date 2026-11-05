# One completed mailbox Call across a replacement connection

The loopback fixture withheld the original complete final ACK after the caller
returned its application result. Both old socket write halves sent FIN. The
fault gate observed clean EOF in both directions and joined its old pumps
before allowing its already-owned replacement carrier. The receiver retained
the same Call and cache under the original broker, replayed its exact Reply,
and verified the caller's canonical receipt before releasing the original
cache. The replacement carried exactly one Reply and one ACK. Both application
Call counters remained one.

| Observed traffic | Caller → receiver direction | Receiver → caller direction |
| --- | ---: | ---: |
| Original caller leg, includes withheld ACK | 10,563 B | 9,734 B |
| Original receiver leg, ACK withheld | 4,675 B | 9,734 B |
| Replacement, matching at both endpoints | 5,869 B | 3,874 B |

Counts include framing and the readiness discriminator. The original withheld
ACK was 5,884 envelope bytes plus a 4-byte frame prefix. The caller thread used
5.253 ms of CPU and the receiver thread used 3.879 ms. The full fixture took
11.488 ms from provisioning through verified cache release and joined pumps.
This is one synthetic local Call, with no throughput or WAN comparison.

The private replacement code is compiled only in tests. The unchanged
canonical receipt path verifies the original tracker identity. The bounded
readiness handshake is reusable and grants no peer or route authority.
Independent code, Rust and security review confirmed this scope. Installed
MVM admission, process restart, production rearm and sustained qualification
are separate gates.

Apply the patch to the combination of PR #91 head `933f36f9` and the adapter
transport in PR #92 head `971dc5cf`. The patch is the original positive direct
witness. The full relay and additional refusal controls are tracked separately.

## Actual Linux duplex relay and final regressions

The follow-up used two actual duplex relay instances, with a joined original
relay before the fixture signalled the replacement. Each relay finished with
zero links, frame slots and serialized-buffer reservations. The replacement
carried 5,853 framed bytes from caller to receiver and 3,847 in the reverse
direction, including discriminator/framing. Caller and receiver thread CPU
were 5.017 ms and 4.262 ms. Full local fixture wall time was 10.796 ms.

The exact Linux-host canonical run passed 42 tests. Its focused additions
cover direct and actual-relay recovery, refusal without a completed tail,
and deadline expiry before the replacement readiness exchange. This is local
component evidence. The production authority and native acceptance plan is
described in [installed-rearm-plan.md](installed-rearm-plan.md).
