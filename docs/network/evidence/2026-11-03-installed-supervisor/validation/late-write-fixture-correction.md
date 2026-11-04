# Deadline fixture correction

**PASS in focused Linux debug and release runs.** The sole fixture change requests retry immediately when the existing initial-complete-send event arrives. It removes the artificial 600ms delay. The signed 900ms lifetime, parent 2s timeout, 1.5s watchdog, typed error, original identity/wire, two-invocation, exact partial-byte, shutdown and refused-reentry assertions are unchanged. Product source is unchanged.

The test file changed from SHA-256 `0c89489a78a875f71996a10e6687465f48f46891916168de306f539792684980` to `3b5278f8a946cea6f9dc4fd50110ae9f43aa435826416e7f74beb0f3b06a972b`. Before syncing it, all 304 tracked crate files matched archived commit `9ff7b247795e70c14243b055e8ac9b1821c09bda`. Before and after both builds, all 304 files matched `late-write-fixed-source.sha256`, which differs from the archived manifest only at this test file. These manifests pin the tracked crate snapshot; they are not a fresh audit of external dependency source.

| Observation | Original focused debug failure | Corrected debug | Corrected release |
| --- | --- | --- | --- |
| Exit / focused tests | 101 / 0 passed, 1 failed | 0 / 1 passed | 0 / 1 passed |
| Actual outcome | `Service(Expired)` | `Unavailable` | `Unavailable` |
| Send invocations | 1 | 2 | 2 |
| Retry starts after seed | None | 472489µs | 253328µs |
| Partial retry payload bytes | 0 | 7 | 7 |
| Scripted write stalls | 0 | 11 | 13 |
| Completed full frames sent | 1 | 1 | 1 |
| Framing bytes sent | 12 | 16 | 16 |
| Worker wall duration | 930526µs | 984788µs | 901807µs |
| Before unchanged watchdog | Yes | Yes | Yes |

Original envelope lengths differ between fresh signed fixtures, so raw accepted-byte totals are not subtracted across runs. Each corrected run independently verifies its own original encoded wire, actual envelope bytes equal original length plus seven, and exactly one complete frame plus a partial retry. Reentry adds no counters or socket calls. Both runs therefore exercise the intended I/O phase. This is scripted `SocketIo` conformance, not OS backpressure or a performance claim.

Commands used Rust 1.97.1, locked dependencies, `linux-host`, normal instrumentation flags unset and two build jobs, targeting the existing `/var/tmp/spiralcore-mvm-service-20261102/target`. The single exact filter was `campaign::canonical::write_deadline_tests::late_explicit_call_retry_partial_write_keeps_original_signed_deadline_and_identity`, with `--test-threads=1 --nocapture`; the second run added `--release`. Debug compile/test durations were 2.21s/1.05s; release 9.39s/0.90s. No full test suite, VM or additional audit was run.

Raw original failure/JSON remain retained. New receipts are `late-write-fixed-{debug,release}.{txt,exit}`, profile-specific JSON, `late-write-fixed-binaries.sha256`, the two source manifests and before/after source-check logs. Each runtime evidence directory was freshly created and the test wrote JSON with `create_new`. The same receipts remain on Wick under `/var/tmp/spiralcore-installed-supervisor-20261103/validation`. No source commit was made by the test author.
