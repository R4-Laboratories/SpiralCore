# Original reader completion: independent review

2026-11-04. Root reviewed the complete additive diff against shared core
`ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1`, both changed methods and their
existing join/Drop/locking/cancellation consumers, plus the complete real-socket
`reader_completion` fixture.

Reviewed SHA-256 identities:

- `src/duplex.rs`: `29e0e9a80ef9fe99df9122cfd854da8ced3097f36ea7b84e69dd9d18435752f3`
- `src/stream.rs`: `d625173e9e6fd923f2101fb5f9cd3c80c68d757c96d3dee6b771f53d44d7bd6e`
- `tests/reader_completion.rs`: `d155c0f01fda67d7b31e4f82ca974dbdf332e7040498d6d83d56204a5722111b`

**Scoped Code/Rust/Security source PASS.** No replacement reader, owner, grant,
timer, socket route or protocol is introduced. The absolute deadline remains
original. Timeout retains the same handle; completion notification permits only
bounded observations of the final native epilogue. The handle is taken only
after `is_finished`. Physical completion and the original worker's semantic
result remain separate: legacy consuming join returns that retained result.
The shared mapped guard mutex preserves alias custody. A panic remains a failed
cleanup outcome; successful repeated cleanup does not reopen I/O.

The two new fixtures block real reader authority handling, prove the timeout
returns while the original endpoint is alive, release and join that same reader,
and check mapped aliases, idempotence and the retained cancellation result.
The implementation author reports protected two-case GREEN and scoped strict
Clippy; the original absence RED and test-layout lint failure remain retained.
Root requested existing duplex/stream regressions on this exact successor
before source publication. That execution is a separate verification gate.

This source review does not approve the installed relay-group consumer, stalled
TLS setup, auxiliary release, positive native retirement, resource model, WAN
or throughput. Those require their own actual integrated witnesses. The prior
[whole-pipeline deadline HOLD](owned-relays-deadline-review.md) remains until the
installed consumer retains pending-reader custody and passes its successor gate.

## Terminal-close successor and executed regressions

Root reviewed the complete successor product diff and new actual TLS negative
fixtures plus the completion-transition unit. Source identities:

- `src/duplex.rs`: `21d4e0418a1b4e600491b1dbd50691ea33b5e3f1c7db1668f03d98e525386f22`
- `src/stream.rs`: unchanged `d625173e9e6fd923f2101fb5f9cd3c80c68d757c96d3dee6b771f53d44d7bd6e`
- `tests/authenticated_stream.rs`: `2899c5d704ea29c353ded7daf52eeb02bc15a596035e8bbffde09a250b0ed88a`
- `src/duplex/completion_tests.rs`: `cd00665952b184b36b54078937185e9588ddab86df5b0e263b6ad85b99a8db00`
- `reader_completion.rs`: unchanged at the identity above.

**Scoped successor Code/Rust/Security PASS.** The terminal classification changes
only the next-frame `poll_input` error path. Typed UnexpectedEof receives the
existing Truncated treatment only after authenticated remote application FIN,
with local application FIN observed under the same TX barrier and no pending
reply. A remote FIN is accepted only with no active object. Partial-frame/body
errors remain in `receive_object`, outside this exception. Outside the complete
barrier the original error is returned; no error strings, TLS parser, cipher,
object commitment, sequence, receipt or publication rule changes. A fresh
`check_delivery` in actual `complete` checks live scope and the monotonic fatal
fence before marking success. Physical completion does not authorize subsequent
buffered delivery after revocation/cancel.

The existing positive fixture failed in 9/10 original-ca5 and 9/10 reader-only
trials, preserving the baseline defect rather than attributing it to the reader
API. Before the fix, actual pinned-TLS missing-remote-FIN and withheld-publication
reply negatives passed. A separate real-socket completion-transition test
reproduced false success after fatal cancellation, then passed with the fresh
check. That unit sets private terminal state and is not an authenticated wire
or native-MVM witness.

Root inspected the actual final `terminal-green` records: library 9, duplex 9,
mapped 1, paired 8, authenticated 10 and reader 2 entries pass (39 executions),
with zero ignored/filtered in those full target executions. The original
positive fixture additionally passes 10/10 runs; scoped strict Clippy exits 0.
Protected executable/source custody and all prior failures remain in the
component packet. This supplies the requested existing-regression gate for
this exact pure successor; it does not qualify the installed host consumer,
native retirement, full managed resource model, FIPS deployment, WAN or scale.
