# Transport diagnostic after the incomplete first pilot

Registered before the separate diagnostic run. Preserve first pilot source
`58990cec4d190f815fccd51df9cadf94029ab78f`, GitHub run `41167011759`, all
partial sender/receiver records and its failed store. Its failure is not a codec
regression or a proven idle-timeout defect. The transport error was discarded.

Use a new frozen revision and executable, new run ID, new capability, new scope
plans and a fresh bounded store. The workflow purpose is `diagnostic`, which runs
**only shard 0: at most six sessions and 66 transfers**. Retain the same three
arms, sizes, sequence, connection cap 32 and 500 ms idle/header policy. Stop at
the first failure with no retry. This is cause investigation, not a replacement
six-pair performance campaign or an acceptance sample.

`HttpClient::last_transport_failure()` adds only fixed allowlisted categories,
optional numeric status, I/O kind and timeout phase; no credentials, payloads,
header values, URLs or arbitrary error text. The measurement row records it only
when that operation actually exchanged a request. The optional receiver
`--connection-log` creates a new separate JSONL file exclusively and streams
bounded metadata at connection close/rejection. Existing admitted-request rows,
codec bytes, authorization, deadlines and error return behavior remain unchanged.

Pin exact source and both binaries. Run focused diagnostic/legacy tests and
Clippy/build before freezing; perform scoped procedural and code/Rust/security
review. No builds, tests or uploads overlap the diagnostic transfer window.
Limit the origin to 1,800 seconds and preserve the 4 GiB disk floor with one
617.2578125 MiB conservative store allowance. Public synthetic data only.

After the run, stop the owned receiver/tunnel, remove temporary GitHub variables
and capability secret, verify the downloaded artifact manifest, and join the
first failed sender row with close/rejection events. Report an unresolved cause
if no failure reproduces or if the retained fields do not discriminate it. Any
subsequent transport change requires a specific reproduction, regression test,
new registration and separate evidence. Do not silently relax 500 ms or add
automatic transfer retries to make the campaign complete.
