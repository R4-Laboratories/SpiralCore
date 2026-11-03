# Zero-prefix receive idle: implementation in progress

Base main `62692428`. The [written Rust contract](../../../api/stream-receive-idle-v1.md)
and [plan](../../../plans/2026-11-03-stream-receive-idle.md) were committed at
`a882a400` before the missing-API RED `5ec5e9d1`. Product `bc854e06` adds a local
Idle outcome without changing canonical framing, authentication, strict receive
or the public error enum. No consumer or retry timer is enabled.

The initial API RED is compiler absence, not an executed behavioral failure.
The first complete portable gate passed five tests. The expanded gate then
passed **11 tests: four existing TCP compatibility tests and seven new cases**
covering healthy idle/authorized write/later exact frame, early retryable errors,
partial prefix/body timing, absolute expiry, cancellation, EOF/truncation/length,
local invalid inputs, lock/state precedence and real TCP. Exact byte/frame
counters and fatal fences are asserted.

The first expanded run's only failure was a fixture count of six for the
seven-byte `b"\0later\0"` literal. Original payload equality had passed; changing
the expected count to `literal.len()` produced the GREEN rerun. Its initial log
and source hash remain in `validation/`. This is not presented as a product bug
or another RED-to-fix claim. Negative cases that passed immediately are
qualification results. Source-only independent Rust/security reviews are in
`reviews/`; full source/portable/native qualification and measurements are pending.

Idle means a full positive local interval with zero next-prefix bytes consumed,
not peer loss or permission to retry. Partial-frame progress uses the original
deadline; it can never be discarded or returned as Idle. Invalid windows are
nonfencing pre-I/O input errors; contended-lock failure retains the existing
unowned-guard semantics. The original receiver fence, event/send ceilings,
pipeline one, 2,048-Call guard and broader #43/#49/#42 acceptance remain open.
