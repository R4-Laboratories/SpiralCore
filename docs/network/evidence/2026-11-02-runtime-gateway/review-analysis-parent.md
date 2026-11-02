# Independent results arithmetic review

Root reviewed the offline Rust analyzer independently of its author. It reads
retained JSON, never changes transfer inputs or the measured gateway. Its own
small locked Cargo package uses the existing JSON dependency. The code checks
the complete registered dose and matches each operation's identity/length to
the trial manifest, receiver publication/readback, and transfer-ID batches.
It rejects incomplete captures and retains finite-queue drops as measured cost.

Bidirectional logical bytes subtract monotonic before/after batch and prefix
counters. These exclude QUIC overhead, which is counted only by full-session
captures. Receiver stage CPU joins all batches belonging to each operation;
sender CPU is not added twice to whole-process totals. Client wall time is used
directly rather than subtracting CPU to invent a send-only interval.

Group cumulative arithmetic charges prepared state and actual auxiliary setup
before deliveries. Connection prefixes charge authentication once and retain
preceding groups. Explicit receiver CPU scope labels distinguish setup-inclusive
prefixes from operation handlers. Summaries retain all three matched trials and
compute each candidate/baseline ratio before the median. Zero denominators stay
null. Combined endpoint-stage CPU is summarized directly, not by adding medians.

Four focused arithmetic/receipt tests and strict Clippy passed. A separate
diagnostic reached all 12 actual Raw operation joins in the incomplete first run
and then rejected its real capture mismatch. Full analysis also rejects that
run's nonzero terminal exit. The diagnostic's only synthetic value is explicitly
marked missing completion metadata, used for test reachability rather than
qualification. No partial performance claim was produced.

Scope: source/arithmetic review passes. Completed campaign execution, original
archive/source/binary custody, and reported numeric comparisons are separate
checks performed on the actual final artifact. The small CPU-scope label repair
does not alter arithmetic; the final analyzer run validates its executable use.

Final execution closed those pending checks: the release analyzer returned zero
on run `41373931230`, admitting all 24 sessions/312 operations/288 deliveries.
The parent independently joined raw client/server JSON for all 12 shaped sessions;
summed CPU and client wall matched the analysis exactly. Report tables and paired
percentages were checked against analysis rows, including first-delivery setup,
both CPU domains, cold-seed losses and the unshaped Runtime elapsed loss. Four
tests and strict Clippy passed again on the final analyzer source after the
CPU-scope labels were completed. Original archive/source/workflow/executable
verification and fresh pinned archive restoration all passed as recorded in
`qualification.json` and the archive receipts.
