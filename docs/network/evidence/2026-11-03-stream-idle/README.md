# Zero-prefix receive idle checkpoint

[Public report](../../stream-idle-results-2026-11-03.md) ·
[All measurements and losses](measurement-review.md) · [Summary](summary.json) ·
[Source/binary pins](execution-pins.json).

Reviewed source `960e73c5` passed the seven native client rows (88 checks each),
local release/shared/Clippy/static-build gates and isolated coverage of 96/97
changed executable lines (98.97%) / 673/733 mapped production lines (91.81%).
Branch mapping remains unavailable. The original seven ZIPs and all 236 inner
checksums verified; executed source tree equals reviewed head tree. See
`qualification/portable/verification.md` and `coverage/README.md`.

Twelve matched strict-transfer host cells passed 108 publications/independent
readbacks. Two current-source native cells passed 18 publications and 18 extra
keyless readbacks. All exact byte/length/counter comparisons passed. These engines
do not use the Idle consumer. Both archives and all 41 host/native members
verified before removing two successful owned native disks (512 MiB apparent
length); no payload, VM disk, raw profile or new target was copied to the Mac.
Individual regressions and accounting domains remain explicit in the report.

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
`reviews/`. The records above provide subsequent qualification and measurements;
the following history retains the distinct initial RED and fixture failure.

Idle means a full positive local interval with zero next-prefix bytes consumed,
not peer loss or permission to retry. Partial-frame progress uses the original
deadline; it can never be discarded or returned as Idle. Invalid windows are
nonfencing pre-I/O input errors; contended-lock failure retains the existing
unowned-guard semantics. The original receiver fence, event/send ceilings,
pipeline one, 2,048-Call guard and broader #43/#49/#42 acceptance remain open.
