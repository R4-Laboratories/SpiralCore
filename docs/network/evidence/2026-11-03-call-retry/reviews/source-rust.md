# Registered Call retry: independent CODE/RUST source review

2026-11-03. **PASS — no blocking finding in the bounded production diff.**
Reviewed product commit `318f05f02db385e928f719f3de6e36dc073f18cc` against base
`a8a4a9a8daba67c00447a067b18632ed515d95b2` and the contract committed at
`a69ac2d`. This approves the additive API source; expanded test/manifest review,
exact-source release/Clippy and hosted qualification remain separate pending
gates. No consumer or recovery-completion approval is implied.

```text
7a58e929401c68f40461a762b72f88b46ef221bc83e33875013005f394e10fcd  baseline crates/spiralcore-mvm-adapter/src/service.rs
5563e59811266047b582651aa6e08682ff3c368d417c2aabdbee8930a1217794  reviewed crates/spiralcore-mvm-adapter/src/service.rs
93d5a71ba3bdcc48fde176ae3a75b62495c14e2d5191cc058e4b6e44adb60dd6  docs/api/registered-call-retry-v1.md
e1b1ea893deba51681cb7a8eb10dcb03242128bfa73f5969daf2b26bb6dcc0bb  docs/plans/2026-11-03-registered-call-retry.md
```

The sole changed production file adds retry_call_prepared and extracts the
original validator into registered_call_send_expectation. Both public methods
retain immutable borrows and invoke the unchanged send_prepared_inner exactly
once with its returned original generation and retained semantic commitment.
The private permission flag is false for strict send and true for retry; there
is no public bypass parameter. Strict send remains Pending-only. Retry additionally
accepts Replied(Some(_)); both reject consumed Replied(None) with Conflict.
Other terminal statuses retain their prior typed mappings.

Validation order is unchanged: service shutdown, invalid/regressed service time,
service/binding ownership, retained ID and exact retained handle, both original
monotonic and Unix lifetime bounds, then status. A Reply cannot waive original
expiry. The helper adds no clock advancement, expiration scan, FIFO change,
registration, active-call credit or receipt mutation. Its wildcard status match
does not bind, clone or take the Vec payload. Service-produced Replied(Some)
retains its reply_budget reservation; retry never accesses or releases it.
Ordinary poll_call remains the sole extraction path in this scope.

There is no outer policy lock. The unchanged binding send path checks cancellation,
encodes and canonically verifies, acquires its policy guard, compares current and
original generations and semantic commitment, then performs existing route,
capability, outbound and size authorization through transport connect/send.
It retains that guard across I/O, preserving existing policy authority without
self-deadlock or a new lock order. There is no broker-dispatch/budget lock added
around I/O. The service borrow prevents same-service local mutation during the
synchronous call; original supplied time and existing transport cancellation
semantics remain the boundary, without a new distributed lifetime guarantee.

The diff adds no retained allocation, duplicate payload, sequence, attempt counter,
timer, wire/receipt/crypto/error change or unsafe Rust. Existing transient encoding
and transport behavior remain. Success is the existing write length rather than
destination acceptance or application completion. Failed/partial transport can
remain uncertain; this immutable API does not claim rollback or self-retry.

Root reports the retained missing-API RED (two E0599 diagnostics) and primary
three-test GREEN. Those executions were not repeated or inferred from this source
review. The concurrent expanded call_send tests are deliberately outside this
verdict until frozen; they must cover exact Pending/unconsumed-Reply retries,
strict refusal and one-time extraction, original lifetime/identity/generation
negative gates, preserved accounting and cancellation/transport failure state.
The portable manifest minimum must match the actual final test-function count
for all seven native client rows. No native performance rerun is required merely
for this API: the normal campaign has no new consumer or measurement claim.

Only this review file was written. Source-diff whitespace checking passed; no
product/test edits, builds, tests, coverage, CI or expanded audit were performed.
Full C8, explicit reconciliation consumers, completed-receiver Duplicate handling,
reconnect/resume, long-session retention, scale and WAN gates remain open.
