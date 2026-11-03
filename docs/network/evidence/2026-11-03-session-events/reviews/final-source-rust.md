# Final frozen-source Rust/code review

Date: 2026-11-03. Verdict: **APPROVED for exact-source qualification; no blocking
Rust/code findings in the complete scoped product diff.** This approval permits
the planned validation and measurements; it is not a test, performance, merge or
full C8 acceptance result.

## Scope and identities

Reviewed `origin/main` at `231ab32103f58e52c206d6e5917d883de4e3a928` through
HEAD `b368eb0f2a20368894fe36742c27be4bc7636138`. Scope is the actual three-file
product diff below. `campaign.rs` was also checked: its session module declaration
is already present and unchanged in the base, so it is not part of this diff.

```text
f72d2f84a675cd7221dd94fa4a10c7d84331fb7c04a95d7200d782795d07c305  crates/spiralcore-mvm-adapter/src/receipt.rs
8540ea581d6b7e323b6502e945031cb187cdf47ead20a26eaa242fa8e0d8969c  crates/spiralcore-mvm-live/src/campaign/canonical.rs
8d8f27e50fe0a6dca43c317b26f8a3eeffce5ab2b60f6f060d442b2153010e13  crates/spiralcore-mvm-live/src/campaign/session.rs
```

This consolidates the earlier additive observer, caller progress/FIFO,
Late/Duplicate and receiver replay reviews. The complete final diff was checked
for additional or conflicting changes. No product edits, builds, tests or
coverage runs were performed by this reviewer. `git diff --check` passed for the
scoped product diff.

## Whole-diff conclusions

- The optional registered-Reply observer preserves one canonical verification,
  complete-match precedence, known correlation-or-causation mismatch errors,
  ambiguity/expiry/commitment errors and strict None-to-Unauthorized compatibility.
  Unknown authenticated requests add no retained identity/observation state beyond
  existing expiry maintenance. The API creates no receipt or admission authority.
- Caller progress is selected only by the authenticated current SendHandle.
  Receipt outcome ranking preserves Duplicate's distinct meaning. Late and
  unknown Duplicate return their actual canonical receipts without current
  progress; AcceptedEphemeral requires a registered match. Unknown verification
  failures are not reclassified as benign stale traffic.
- Caller payload ownership remains in the charged canonical service until current
  receipt, current Reply and successful required acknowledgment send are all
  established. ACK invocation is marked uncertain before I/O. The original
  deadline and canonical handle/generation checks precede payload consumption.
  FIFO Pending continues receiving within the retained 16-event allowance;
  Replied(None) and terminal states cannot fabricate completion.
- Receiver repeated-Call admission is preceded by authenticated lookup and exact
  current cached Reply comparison. After duplicate receipt I/O it rechecks the
  cache and requires the original tracker handle before replay. It does not
  reexecute object work, allocate Reply options, reseal or advance sequence.
  Only a current authenticated eligible Reply receipt can release that cache.
- Receiver errors retain a latch set before the initial read, fencing later
  invocations before another read. One exchange permits 16 complete input frames
  and eight Reply-send invocations, counted before transmission. Caller/receiver
  failures do not create a fresh identity or skip a FIFO gap.
- The corrected immutable ExchangeDeadline binds the original receiver Instant,
  signed Unix expiry and admission owner-monotonic expiry. Fresh guards cover
  initial receipt reservation, work entry, post-work preparation/registration,
  the initial send boundary, event/cache release and duplicate receipt/replay.
  No clock or expiry is refreshed on a repeated event.
- Extra ciphertext/result copies and active loop state remain bounded per
  exchange; canonical payload/cache charges remain owned by the existing
  service. Permits are stack-local, with no self-referential stored borrow.
  Socket waits and sends remain outside broker dispatch; no accounting lock is
  held across transport I/O. No unsafe Rust, new wire format or authority model
  is added by this product diff.

The initial receiver snapshot had a real first-Reply fixed-deadline gap missed by
my initial review and correctly raised by independent security review. The
correction at `f2b1cc8` is included in the hashes above; its recheck is appended to
`receiver-replay-rust.md` and independently resolved in
`receiver-replay-security.md`. Earlier approval is retained as history, not used
to deny or erase that finding.

## Qualification boundary

Receiver error/fence/event/send tests were being completed separately. Their
final identities and execution records, the full exact-source suite/Clippy,
coverage, paired metrics, native cells and platform CI remain qualification
evidence to record. This source approval does not substitute for any of them or
claim that instrumented tests are performance evidence.

The guards refuse phase entry/completion after expiry; they do not preempt
ongoing object work or guarantee time cannot pass during blocking I/O. Explicit
resume, timer-driven request retransmission, full C8, >2,048 Calls, long retention,
WAN and remaining release acceptance are not established by this source review.
