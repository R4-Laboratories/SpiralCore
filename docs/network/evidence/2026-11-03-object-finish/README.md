# Object QUIC terminal custody correction

This is a bounded object-profile fix under the
[finish contract](../../../api/gateway-object-finish-v1.md). It changes no
application frame, stream count, codec, ALPN, certificate policy, packet profile,
retry or time allowance. It does not establish complete WAN recovery.

## Failure and causal regression

PR71 run41763703472, sourcecc67cda93dee05b3e2b3fae6786f377a65b03001,
accepted16 policy-control sessions before failing the17th,
`wan80-10-t2-combined`, at the server process wait. The client reported20
successful operations and finish, but the server failed with
`Stopped(ConnectionLost(TimedOut))` while waiting for its terminal send ACK.
No capture/qdisc acceptance ran for that session; no18th session ran. Original
[source](validation/candidate/source.json), [client](validation/candidate/client.json),
[server failure](validation/candidate/server.json), stderr and accepted-session
log are retained. The original36MB ZIP remains outside Git with recorded
SHA-256 `ec2cf28e2da69194bdce2c432d6ead54aa0b5c79c342803946a2c6b4fe90947b`.

The controlled regression used exact baseline
`86cae0a509deb44043265c0dd16128fd43509cb1` plus the new test only, real pinned
mTLS QUIC and an owned UDP proxy. After the responder read the initiator's end
prefix, the proxy held the initiator-to-responder flight while forwarding the
reverse direction. The old initiator returned success with the responder
still awaiting its final send ACK. Two actual datagrams/1453 bytes were held;
the gate was released and both tasks joined before the semantic assertion
failed. See [RED](validation/finish-red.txt),
[source hashes](validation/finish-red-source.sha256), and
[actual gate receipt](validation/finish-red-json/gated-fin.json).
An earlier mistaken exact filter compiled but ran0 tests; that attempt is
retained separately and is not labeled RED.

## Implementation and observed gates

The initiator retains its actual endpoint/I/O after the existing end/FIN and
self-send acknowledgment until it observes the responder's application close0.
Only after its own terminal send is acknowledged does the responder perform
the existing close0 and await endpoint drain. The common original-deadline/
cancellation wrapper bounds both waits. Early close0, nonzero close, missing
close, cancellation and expiry remain fatal/poisoning. Helpers are private to
the single-connection object endpoint; packet-profile behavior is unchanged.
The object-service change is only its timing description, identifying that
UDP snapshots now include the relevant close/drain window; counter logic and
benchmark acceptance predicates are unchanged.

The final frozen source passes **51 release tests across14 targets**, including
4 zero-test targets and12 object-channel tests. Strict release all-target
Clippy passes. The3 new object-channel test functions contain8 controlled
subcases: delayed/discarded terminal ACK flight, early0/nonzero close,
withheld close, cancellation, one close-phase datagram loss, and complete
post-FINACK responder-flight loss. Exact framing remains4 bytes per end in
the empty terminal exchanges, with no new batch/frame. Actual proxy counts
separate forwarded, retained and discarded datagrams.

Discarding a single encrypted close-phase datagram was followed by a valid
remote close and successful completion. This **does not prove that the first
CLOSE packet was discarded**. An initially incorrect fixture expected failure
for that condition; its11PASS/1FAIL result remains preserved. Withholding the
close, or discarding the entire post-FINACK responder flight, instead produces
the original Deadline and poison. A subsequent fixture-only failure exposed
premature raw endpoint Drop causing proxy ICMP refusal; retaining that owned
endpoint until proxy shutdown corrected custody without changing assertions.
The final focused result is [12PASS](validation/finish-boundary-fourth.txt).

The first all-target command could not compile an existing example because
the crates-only archive omitted its `include_str!` fixture manifest. Adding
only that existing manifest from the exact86ca source resolved this source
export prerequisite. The failed attempt is retained. Final
[release](validation/finish-release2.txt), [Clippy](validation/finish-clippy2.txt),
[source before](validation/finish-final2-before.sha256)/after hashes and
[frozen local source](frozen-source.sha256) record the accepted execution.

[Selective coverage](coverage/README.md) covers26/26 changed production mapped
lines (100%); the whole three-file production scope is965/998 (96.69%). Branch
mapping is unavailable. A missed probe child custody step was corrected by
preserving the child and replaying only its3 tests; the original collection
and correction are retained distinctly. Raw coverage objects/profiles remain
on Wick; no binary, payload, TLS key log or packet decryption is published.

Independent source/security and measurement code/infra reviews pass; their
receipts are retained here. First-CLOSE-specific loss qualification, arbitrary
terminal-loss recovery, hosted platform gates and the final shaped/WAN
campaign remain separate. No constant keepalive, surrogate PING, dependency
fork, sleep or timer increase was introduced. The original failed candidate
remains failed; these process fixtures do not turn it into a passing campaign.

The API document's status header was refreshed for publication after those
reviews; the four Rust source/test hashes are unchanged. Its original test-time
version and manifest remain in `validation/contract-at-test.md` and
`validation/frozen-source-at-test.sha256`. The top-level frozen-source manifest
seals the current publication, including the status-only documentation refresh.

## Separately reviewed measurement prerequisite

The PR assembly also carries the already published b0 bounded qdisc-drain
correction as a separate commit. It waits for actual queue-empty snapshots
under the unchanged strict predicate, retains every bounded observation and
never resets queues or reruns a session. The
[scoped code/infra review](review-qdisc-infra.md) and one pure predicate-check
execution (1 qdisc positive/5 negatives) are retained. This measurement change
is distinct from the QUIC product bug; it does not establish WAN execution or
change the registered Whole-object codec/control source.
