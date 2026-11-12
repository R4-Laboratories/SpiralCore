# Optional timing for the sustained FlowMux witness

Status is a focused diagnostic contract for source `8a109ae4966b0f19ef0cbb41e0311ee3cb926f6d` on `codex/spiralcore-ordered-scope-main`. Product behavior, authorization, protocol, counts and deadlines retain their existing acceptance conditions. Instrumentation is optional through a default-off `witness-timing` Cargo feature in `spiralcore-mvm-flowmux`.

The sustained ordered witness must still deliver exactly 4,096 blocks in each direction, compare every recovered byte and retain the original 240-second gate. The purpose is to locate elapsed time in existing operations before selecting another optimization.

## Bounded ownership and output

Add one fixed-size timing owner per actual carrier and duplex, shared by their existing clones. Keep counters in fixed AtomicU64 arrays selected by closed enums. No per-event vectors, maps, strings, global registry or background worker. Record count, total elapsed nanoseconds and maximum for each span, with saturating arithmetic and an overflow flag. Counter exhaustion cannot fail, retry or otherwise change the product operation. Convert duration to nanoseconds with explicit saturation.

Feature-off builds contain no timing state, clock reads or recording calls after conditional compilation. Feature-on builds add only bounded state and monotonic-clock/atomic operations. Declare the exact added bytes in the witness and record the feature state in each result. Preserve the existing SmallV1 core admission envelope. If diagnostics require additional fixture resources, reserve them explicitly outside that product charge and report them. Do not silently expand the profile.

Expose immutable scalar snapshots through feature-gated observers, never TLS material, object IDs, capabilities, payloads or filesystem names. Live snapshots are concurrent observations rather than one atomic global instant. Label them accordingly. Final snapshots follow worker completion where existing ownership permits it.

## Recording sites

| Existing source site | Fixed observations |
|---|---|
| `tls.rs::pause` and its callers | Pause count, requested duration and actual elapsed duration by reason. Use closed reasons for handshake, poller ownership contention, TLS-session ownership contention, transient read/write/flush and shutdown. Keep existing park duration and caller deadline. |
| `tls.rs::PollState::wait_checked` | Each original `mio::Poll::poll` requested duration, actual elapsed, event count and outcome. Classify socket-read, socket-write, internal wake, empty timeout, interruption, other error and owner-check refusal with fixed counters. One event may increment more than one category. |
| `tls.rs::with_session` | Successful and contended session attempts, plus time spent in successful session operations. Preserve the existing lock, pump and notify ordering. |
| `duplex.rs::DuplexWriter::exchange` | Request count by existing frame kind, time acquiring its TX mutex, number of contended attempts, `send_object` elapsed time and response-channel wait elapsed/outcome. Record a completed response ordinal only after existing response matching and checks. |
| `duplex.rs::Worker::request` | Time in existing endpoint request processing and response TX lock/send, by request kind. This distinguishes receiver work from sender wait without attributing it all to transport. |

Use a fixed mapping for the existing frame kinds and a single unknown/other bucket. Observation never determines protocol acceptance. Distinguish timed-out, disconnected, refused and successful operations. Record time on every existing return path using a lightweight guard where it cannot affect lock lifetimes. Instrument the two actual TX lock sites rather than changing generic lock behavior for unrelated mutexes.

A requested wait is the original computed timeout argument immediately before the call. An actual wait is the monotonic elapsed duration around that call. Scheduling overshoot can make actual exceed requested. Empty-poll returns, interruption and spurious wakes preserve their existing retry paths. Telemetry must not add polling, retry, sleep, notification or fencing decisions.

## Interpretation

Sender lock acquisition, send and response wait are sequential spans for one exchange. Their aggregates can overlap the receiver handler, TLS pumping, polling, pauses and other exchanges on the second direction. Poll elapsed includes scheduler effects. These metrics are neither independent CPU time nor send-only wire time. Do not add nested or overlapping totals to manufacture total latency.

Use `cpu_time::ProcessTime` as a test-only dev dependency already present in the locked dependency closure. Sample process CPU at setup start, setup completion and witness completion or failure. It measures the process including both local peers and diagnostic work. Report wall time separately. Retain existing Linux tick/RSS observations as optional platform-specific fields, labeling unavailable fields explicitly.

## Witness record and qualification

Update only the existing `ordered_stream` witness to retain feature-gated timing observers, TLS counters and atomic written/read/ACK ordinals per direction. Print one bounded snapshot on success and before a failure is returned or asserted. Include actual CPU and wall samples, elapsed deadline budget, scope-check status and existing retention snapshots. An ACK ordinal denotes an existing validated object response; it is distinct from the application's byte-read ordinal. Do not introduce an additional ACK or consult counters for correctness.

A bounded failure reporter must not wait for product mutexes that may be stalled. Atomics are always readable; attempt existing diagnostic locks only with nonblocking access and report unavailable instead of retrying. Capture diagnostics before existing teardown can discard them, preserving original cleanup and panic behavior. No keys, credentials, envelope contents or environment dumps enter output.

Qualify counter arithmetic/saturation, closed bucket mapping and return-path recording with small focused tests. Reuse existing TLS readiness/closure and duplex frame matching tests to show feature-on and feature-off acceptance agree. Run the same sustained test with `--features witness-timing -- --show-output` on the failing native role, retaining the 4,096/240-second gates. Compare at least one matched feature-off execution before inferring product performance. Feature-on clock and atomic overhead is part of its measured result and must be stated. A diagnostic pass establishes only the exercised component scope.
