# Object-only QUIC terminal delta

Source/security verdict: **PASS for the bounded terminal ACK race correction**.
Reviewed the three production-file diff against `86ca` and focused test
expectations; no build, measurement, or raw artifact revalidation was performed.

Exact SHA-256:

- object_channel.rs: `af5898b0d790a649ef3b2264e21f26f031f3b824f2f9f0100040a35ae7bd826d`
- transport.rs: `699b2299e0702458fe2dbac0986743c0dafa61a0db091ae1333d9856f65d2827`
- object_service.rs: `a0b618e69430f88b912f0815837d256d514a26757967ec88847e133050a56a0b`
- tests/object_channel.rs: `61696cb24bf1dc089d481fbd6721a75862afb6591e0ff7c7467a8b1746cb2361`

The initiator first requires the original end prefix, received FIN and its own
send acknowledgement, then accepts only remote authenticated ApplicationClosed(0).
The responder consumes the original FIN and waits for its own terminal send to be
acknowledged before closing and retaining the private endpoint through wait_idle.
Both additions run inside the existing absolute deadline/cancellation wrapper;
failure poisons the channel. Early close0, nonzero close, missing close, cancelled
and truncated operations do not become success. The listener is consumed and
disables new server acceptance, and the packet profile cannot enter these helpers.
No changed ALPN, TLS authority, codec, packet-profile configuration or custom frame.

The real opaque UDP gate tests retain both peers until cleanup and compare early
completion against the still-pending responder; they do not fabricate ACK writes.
One discarded close-phase datagram is correctly described without asserting it
was the encrypted CLOSE packet. Full terminal-flight loss remains Deadline, with
no inference of success from local FIN acknowledgement. That is an explicit
remaining loss-recovery gate, not evidence of general WAN/1%-loss acceptance.

The sole object_service production hunk accurately moves the stated UDP snapshot
boundary after finish/observed close (responder drain included). This review makes
no performance claim and does not elevate the supplied release-test/Clippy reports
to independently executed results.

Architecture basis: Quinn documents last-receiver-initiated graceful close and
endpoint retention via wait_idle. See
https://docs.rs/quinn/latest/quinn/struct.Connection.html#method.close and
https://docs.rs/quinn/latest/quinn/struct.Endpoint.html#method.wait_idle.
