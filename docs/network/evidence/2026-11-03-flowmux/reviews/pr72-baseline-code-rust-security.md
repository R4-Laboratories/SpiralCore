# PR72 bounded independent baseline review

Reviewed frozen production head57c09d9253f9fb509044b578dd278e777f744a13,
its eight pure-component modules, contract and actual seven-native-row custody.
Disposition for that source: **HOLD**, despite33 tests/native success and91.12%
mapped coverage. A concrete lifecycle defect was found: explicit fatal cancel
could still release previously verified buffered/queued bytes because the read
path checked live scope but ignored fatal transport state. One authenticated
real-stream regression reproduced5 returned bytes after cancellation while scope
remained valid. This is distinct from legitimate drain after clean half-close.

The remaining reviewed boundaries have no additional blocker in this bounded
scope. Raw from_connected APIs explicitly require caller selection/trust and do
not authenticate or open a route; the advertised authenticated profile instead
checks actual mutual TLS role/SPKI/exporter and trusted enrollment. Complete
scope facts/namespace bind the context, direction and connection; generation/id/
expiry checks occur on use, including buffered application delivery. Codec epoch
slots remain reserved0, not invented MVM authority. Enrollment is bounded32 and
re-enrollment revokes old authority IDs. Raw peer labels or a hash alone confer
no permission. Actual signed-plan/route/DNS/rate/audit host integration remains an
external requirement; these component tests do not prove it.

TLS1.3/RPK/ALPN uses rustls's existing signature verification, exact pinned public
keys and one shared TLS connection for clones, with early data/resumption disabled.
Record parsing bounds headers/body before allocation; shared object decoding,
identity verification and publication remain in the one existing core. Application
and artifact purposes are separate, direction/index transfer IDs do not reset
inside a connection, and only verified Published advances readiness. Local queue
capacities and32-frame/original-deadline limits are explicit; terminal/error
paths retain counters and fence reentry. General core/aggregate installed-memory
limits and long-session performance are not qualified by this33-test release.

The repair author (this reviewer) then implemented the requested reason-state
fix, so **this document is not independent approval of that authored delta**.
Root separately reviewed the five-file repair. Instrumentation subsequently
caught an author refactor mistake: removal of the original Tx mutex barrier
before inspecting local_closed on EOF. The initial independent review was
explicitly corrected, and Root approved the exact restored source b5df190a.
Those review records and failed run remain preserved rather than backdated.
See [repair and reviews](../validation/pr72/cancel-fence/README.md) and
[final coverage](../coverage/pr72-restored/README.md).

Final successor1e69a99d has34 ordinary/34 selectively instrumented tests and
strict Clippy passing, with1954/2145 mapped lines91.10% and repair-only48/49
97.96%; no branch coverage is claimed. Fresh successor native matrix/custody is
required before merge. Later45-test TLS/Mio and Small/Ordered work is excluded.
No native MVM, installed aggregate-budget, sustained/WAN or whole-program release
approval is given by this bounded review. The earlier Windows terminal-reset
fixture-only review/author involvement is disclosed; production baseline review
remained independent of its production author.
