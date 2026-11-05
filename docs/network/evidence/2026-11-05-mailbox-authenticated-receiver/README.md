# Canonical receiver continuation after an admitted replacement

The canonical receiver now takes a private one-shot callback for an original
owner to supply an authenticated replacement. It calls that hook only during
the pending Reply ACK window, after a clean frame-boundary EOF and original
Reply cache validation. The hook receives the original exchange deadline and
cancellation token. Replay revalidates the original cached wire and tracker.
The receiver returns the replacement carrier only after verified ACK and
cache release, allowing the owner to retain that carrier for subsequent Calls.

The locked Linux release suite passed 43 tests. The direct loopback and actual
duplex relay fixtures both check exact replay bytes, one application result,
and final ACK. An explicit callback refusal produces zero replacement frames
and zero replay invocations. The first run's refusal fixture joined a gate
that was intentionally never handshaken and treated the resulting cancellation
as a failure. The successor joins the gate with its shutdown path and still
checks its final captured bytes. Scoped Code, Rust, and Security review passed.

Apply [the patch](receiver.patch) to the exact
[closed-carrier fixture candidate](../2026-11-05-mailbox-closed-replacement/).
The [test record](validation.txt) pins the source and release binary. The
[loopback and relay records](loopback-and-relay.jsonl) include frame counts,
envelope/framing bytes, CPU intervals, and elapsed time. These are local
component measurements.

The fixture substitutes a trusted callback decision. The readiness exchange
by itself grants no peer authentication. The installed resident must still
quiesce the original carrier and verify the enrolled guest and exact session
through the original Firecracker worker before returning a ready carrier.
Live V2 admission, installed same-VM recovery, and WAN qualification remain
pending.
