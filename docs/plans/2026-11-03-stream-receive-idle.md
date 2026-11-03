# Plan: zero-prefix receive idle for C8 recovery

Base: main `62692428cd1e9effc51c7930f7b1f34cc1519277`, isolated branch
`codex/spiralcore-session-idle`. Authority: #43 under #42/#49 and the remaining
C8 recovery item in the persistent-service plan. The reviewed
[Rust API contract](../api/stream-receive-idle-v1.md) governs implementation.
This is within the user's approved autonomous roadmap; wire format, privileges,
endpoint roles, quotas and deployment scope do not expand.

Use the existing serialized transport. A second reader/writer architecture adds
ownership and lifecycle work. Short strict timeouts close the stream. A bounded
zero-prefix Idle result instead releases the existing mutex between polls while
preserving one absolute frame deadline once any bytes arrive.

1. **Contract RED**, `crates/spiralcore-mvm-adapter/tests/service_tcp/idle.rs`
   and its explicit module declaration in `tests/service_tcp.rs`. Add portable
   scripted SocketIo tests and one bounded real-TCP smoke: Idle then exact later
   frame and authorized writer; early retryable errors; prefix/body progress
   past idle but before original expiry; fatal deadline/cancel/EOF/length with
   partial counters; invalid local inputs; closed/unready/contended state.
   Observe missing API RED before product code. Dependency: contract. Risk: high.
2. **Prefix boundary**, `crates/spiralcore-mvm-adapter/src/live.rs`.
   Add a small private read-progress result and an optional idle bound only
   before offset zero becomes positive. Preserve existing read helper wrappers
   and strict None-mode behavior. Keep the first patch near 50 source lines;
   retain checked counters and absolute-deadline precedence. Dependency: 1.
   Risk: high.
3. **Frame and public method**, the same file. Share existing framing/length/
   allocation logic; add ReceiveOutcome and receive_or_idle, validating window
   before locking and starting idle only after ready checks. Split parser and
   method edits into small patches. No LiveError variant, new wire, socket clone,
   queue, timer or consumer change. Rerun focused tests to GREEN. Dependency: 2.
   Risk: high.
4. **Boundary qualification**, test files above and
   `docs/platforms/mvm-mailbox-portable.json`. Require unchanged strict behavior
   and exact partial/complete counters; update the actual nonzero service_tcp
   target minimum after tests are established. Never treat an immediate mocked
   TimedOut as proof that a positive idle interval elapsed. Every worker has a
   wakeup/join guard. Dependency: 3. Risk: medium.
5. **Source gate and delivery**, compact evidence under
   `docs/network/evidence/2026-11-03-stream-idle/` and issue #43. Use the existing
   Wick target, two build jobs and retained protected binaries/profiles. Run
   focused/full locked release, strict Clippy, compatible default guest build,
   seven native client rows and selective >=80% scoped line coverage. Obtain
   independent Rust/security review and recorded quality-gate checklist before
   protected PR merge. Keep raw failures, unmapped coverage and unavailable gates
   explicit. No instrumented performance or new WAN claim. Dependency: 4.
   Risk: medium.

Record byte counters and CPU/wall for a small unchanged normal-transfer
confirmation if framing's legacy hot path changes; use the existing Rust engine,
protected source/binary pins and matched zstd/Combined inputs. Register any such
cells before execution, retain regressions and copy only compact records. This
seam alone does not qualify retry throughput or long idle sessions.

Success is the reviewed additive API with exact framing and fatal-error
compatibility, positive bounded Idle and preserved connection/counters, and
portable executed evidence. Then separately design original-identity retry and
uncertain Call/Reply/ACK recovery. Receiver fencing, pipeline one, the 2,048-Call
guard, original authorization/lifetime and event/attempt bounds remain unchanged.
