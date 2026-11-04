# Object QUIC finish custody v1

Status: implemented and component-qualified with independent architecture,
code and security review. Hosted, shaped and real-WAN acceptance remain open.
Scope is the existing pinned-mTLS object
profile and its single reliable stream; packet-profile behavior is unchanged.
There is no new application frame, capability, codec, retry or time allowance.

## Observed failure

PR71 candidate run41763703472, source
`cc67cda93dee05b3e2b3fae6786f377a65b03001`, accepted16 sessions and failed
session17 (`wan80-10-t2-combined`) at the server process wait. The client
reported20 successful operations and successful finish; the server reported
`Stopped(ConnectionLost(TimedOut))` from its terminal `SendStream::stopped`.
Capture/qdisc acceptance had not run for that session. The original artifact
is retained; neither client success nor the earlier sessions qualify the
failed pair. Its encrypted capture does not establish which ACK was lost.

## Existing wire and completion boundary

The initiator writes its existing zero prefix and FIN, reads the responder's
zero prefix/FIN and confirms acknowledgment of its own send. The responder
reads that existing end/FIN, performs its existing durable readback, writes its
zero prefix/FIN, then confirms acknowledgment of its final send.

Initiator completion must additionally keep the actual QUIC endpoint and its
I/O alive until the responder's application close with code0 is received.
Only after its terminal send is acknowledged does the responder invoke the
existing code0 close and await `Endpoint::wait_idle`. All waits retain the
original caller absolute deadline and cancellation token. Deadline/cancel,
nonzero application close, transport failure, or interrupted completion fence
the channel and remain errors. Neither role may report successful finish by
ignoring a missing terminal acknowledgment. No fixed grace sleep or renewed
timeout is permitted. Existing stream, mTLS pin, ALPN and length checks remain.

The responder's positive boundary is its terminal stream acknowledgment;
the initiator's positive boundary is that responder's observed code0 close.
The responder's endpoint drain retains delivery custody through bounded QUIC
closing, without claiming that arbitrary process death/network partition can
guarantee remote knowledge of success. Peer/process loss remains an error.

## Focused executable gates before implementation

Use real authenticated QUIC endpoints with a bounded owned localhost UDP proxy.
Once the responder reads the initiator's end prefix, gate initiator-to-responder
datagrams before the responder emits its terminal end/FIN. Forward the reverse
direction unchanged. This withholds the final ACK from the actual responder,
while letting the old initiator receive end/FIN and complete. The proxy records
actual received/forwarded/held counts and releases its bounded retained queue
only on an explicit test signal; it is not a successful-write socket shim.

The primary test must observe a held datagram and a still-pending responder,
then require the initiator to remain pending until release. The old source
must fail this semantic assertion before product edits. Cleanup releases the
gate and joins both endpoint tasks/proxy under a watchdog before assertions,
so retained failure evidence cannot leave owned tasks running. Follow-up
tests require cancellation/original-expiry while awaiting responder close,
nonzero close refusal, and normal exact batches with unchanged framing counts.
Any intentional timeout used to observe a pending future is a bounded test
assertion, never a product grace sleep or larger operation budget.

Source starts from the immutable `origin/main`86ca checkpoint; exact full commit,
source hashes, commands, original failure, RED and GREEN receipts are retained
under `docs/network/evidence/2026-11-03-object-finish/`. No object-service,
measurement driver, MVM, benchmark acceptance or wire changes are authorized
by this correction.

## Executed checkpoint and remaining loss boundary

The initial real-UDP-gate regression failed against exact baseline
`86cae0a509deb44043265c0dd16128fd43509cb1` plus test only: two datagrams
(1453 bytes) were held, the responder still awaited its terminal ACK, and the
old initiator reported success. Both tasks were released/joined before that
assertion failed. The minimal implementation then passed that gate. Extended
qualification includes discarding the entire held initiator ACK flight and
recovering through actual QUIC retransmission, without a second stream/frame.

One dropped encrypted close-phase datagram was followed by a valid remote
close and successful completion. That is not proof that the discarded packet
contained the first CLOSE frame. An initially incorrect test expecting failure
for that condition remains retained as a fixture-expectation failure. Separate
withheld-close and all-post-FINACK responder-flight-loss cases require the
original deadline and a poisoned channel. Thus missing terminal close remains
honest bounded failure, not inferred success or universal close-loss recovery.
No TLS secrets/key log or payload decryption is used by the fault proxy.

The pinned Quinn0.11.12 public Connection API has no dynamic terminal-only ping
or keepalive setter. `TransportConfig::keep_alive_interval` is configured
before connection creation and defaults off. No profile-wide keepalive,
undocumented key-update surrogate or dependency fork is introduced here.
See the primary [endpoint drain documentation](https://docs.rs/quinn/0.11.12/quinn/struct.Endpoint.html#method.wait_idle)
and [transport keepalive API](https://docs.rs/quinn-proto/0.11.19/quinn_proto/struct.TransportConfig.html#method.keep_alive_interval).

Counters are snapshots after the relevant finish boundary: the responder now
includes its endpoint drain and the initiator includes its observed remote
close. Independent capture can include subsequent teardown traffic. The object
service's timing description changes only to identify this actual new window;
no measured-byte field or benchmark acceptance predicate is modified. The
original failed candidate and all later qualification receipts remain separate.
