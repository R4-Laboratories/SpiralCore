# First receiver replay slice: independent Rust/code review

Date: 2026-11-03. Verdict: **APPROVED for the scoped first receiver slice; no
blocking Rust/correctness findings.** Negative fence/bounds tests remain part of
executed qualification. This is not approval of receiver resume, timer-driven
request retransmission or full C8 completion.

## Exact reviewed identities

Checkout HEAD during review: `0292ecc50275f6a1e037bd640e0f53b86dd8b178`;
receiver production changes were uncommitted. SHA-256:

```text
e1c27fb594be4f7ecf2f7caf563279c6900741f5e1a36d467d04622fb0b3d6bd  crates/spiralcore-mvm-live/src/campaign/canonical.rs
c99ef4eea2a7a69ec8e07e448a732c091e393ec89b207dded6c91933c38c9446  crates/spiralcore-mvm-live/src/campaign/session.rs
4dcbc7b63401de44d8fc0e8f8daed11587f032ebfe135942c4c121d3664fe925  crates/spiralcore-mvm-live/src/campaign/canonical/receiver_tests.rs
04673e39509bdd724bf6c931a029b966804b150cac0aa99e8d5caa574cc9200f  docs/specs/2026-11-03-persistent-session-state.md
```

Scope: receiver loop, incomplete latch, fixed deadline, exact cached Reply
precheck/replay, tracker reconciliation, acknowledgment release, event/send
limits, and the existing exact-replay/different-Call fixtures. Adapter APIs were
read to verify their actual ownership, deadline and generation semantics. No
product edits, builds, tests or coverage runs were performed by this reviewer.
`git diff --check` passed for the two production files.
The fixture hash above identifies the initial reviewed snapshot. Separately
owned negative-test extensions were being written concurrently and are not
covered by this source verdict; they require their own completed test identity.

## Source conclusions

- `receiver_incomplete` is set before the initial read. Every early return or
  unwind keeps it set; a later invocation fails before consuming another frame.
  It clears only after current authenticated acknowledgment, cache release and
  successful metric/counter bookkeeping. No stack disposal implies rollback.
- One Instant deadline starts before the first read and is capped once by the
  authenticated request expiry. It is not refreshed after object work or a
  duplicate. Fresh fixed-deadline and original Unix-expiry checks precede
  acknowledgment/cache release and cached Reply resend. Canonical preparation,
  cache lookup, receipt tracking and send authorization retain their existing
  owner, policy-generation and lifetime checks.
- The initial Call consumes event 1; the loop permits only 15 further complete
  frames. Reply-send accounting starts at 1 before the first invocation and
  increments before each resend, allowing at most eight. Receipt-control events
  remain processable after the eighth Reply; a further normal replay refuses.
- Before admission, `retry_reply_prepared` authenticates the candidate and must
  return a cache whose encoded bytes equal this invocation's original Reply.
  A different/new Call therefore cannot acquire service state merely because
  original active Call credit has been released. Admission then must return its
  actual Duplicate receipt.
- After duplicate-receipt I/O, a fresh cache lookup checks the original request
  and both cached binding generations again. Its encoded bytes must remain exact;
  registration must return the original SendHandle. There is no object receive,
  object execution, Reply-option allocation, resealing or sequence advancement
  in this branch. Prepared/cache copies remain bounded local ownership.
- Controls undergo canonical authentication, followed by `take_receipt` for the
  original current SendHandle. Pending observations cannot release the cache.
  Only eligible signed outcomes for that exact retained Reply reach
  `acknowledge_reply`; false release or remote refusal returns failure. Duplicate
  is not promoted to a durable acceptance claim.
- Socket read/send calls are outside broker dispatch. Prepared receipt and send
  permit borrows are stack-local; service/tracker/cache ownership remains intact
  on uncertain send failure. No accounting mutex or broker lease spans I/O.

The exact-replay fixture checks a real object Begin, unchanged cached Reply wire
and Reply sequence, matching signed receipts and directional traffic, one handled
Call, and one cache release. Its comments correctly avoid claiming a dedicated
object-execution counter. The different-Call fixture checks refusal without a
second admission, original cache retention and no extra response bytes. Their
owning guard wakes and joins the receiver on failed assertions.

## Focused qualification follow-up

The separately assigned negative tests should retain evidence for: cached Reply
send failure after a successful duplicate receipt; event 16 exhaustion; eight
Reply invocations with a still-eligible final acknowledgment versus refusal of a
ninth invocation; original deadline expiry; and refusal of a second `handle`
before any additional read after each failure. Check cache/identity retention,
unchanged Reply sequence, actual partial directional counters and zero new
admission/object work. Earlier authenticated control observations must not release
the current cache. These are execution gates, not findings of a source defect.

The implementation run's reported 21 focused passing tests are separate from
this review. Full direct/relayed failure qualification, supported platforms,
retention and automatic request-retry claims remain outside this source verdict.

## Final deadline correction recheck

Reviewed commit `f2b1cc89eb3a7a80cd58b4bf7cd0d18c7453bb07` on 2026-11-03.
Verdict: **APPROVED for the narrow corrected receiver slice; the initial-path
deadline gap is resolved, with no further blocking Rust/code finding in this
recheck.** The earlier verdict and exact identities above remain historical
review evidence, not a claim that the old snapshot lacked this defect.

The independent security review correctly identified a gap my initial approval
missed: after application work, the first Reply preparation/registration/send
path did not enforce the fixed receiver deadline. A near-deadline initial read
followed by object work could therefore emit its initial Reply beyond that local
exchange cap while canonical signed authorization remained valid. This was an
exchange-lifetime defect, not evidence of a capability/signature bypass. See
`receiver-replay-security.md` for the independent finding and resolution.

Exact final reviewed SHA-256 identities:

```text
8540ea581d6b7e323b6502e945031cb187cdf47ead20a26eaa242fa8e0d8969c  crates/spiralcore-mvm-live/src/campaign/canonical.rs
8d8f27e50fe0a6dca43c317b26f8a3eeffce5ab2b60f6f060d442b2153010e13  crates/spiralcore-mvm-live/src/campaign/session.rs
64cf309da6732e595dbd8b6979ff414c26fd71e1be92b70e89a44c0ed8509bf4  crates/spiralcore-mvm-live/src/campaign/session/tests.rs
```

`ExchangeDeadline` now retains three immutable limits: the original receiver
Instant cap, the original signed Unix expiry, and a checked owner-monotonic
expiry fixed at authenticated admission. It rejects equality at any expiry,
owner-monotonic time before admission, nonpositive lifetime and checked arithmetic
overflow. No check updates a clock, extends an expiry or introduces a retry API.

The receiver checks these limits before initial receipt reservation, before
application work, after application work before Reply preparation/registration,
and again with fresh broker time immediately before the initial Reply-send
invocation. The same helper guards event processing/cache release, duplicate
receipt reservation and the fresh cached Reply resend. Subsequent receives use
the retained Instant; no replay resets it. Existing latch, exact-cache/tracker
identity, event/send budgets and I/O-outside-dispatch behavior remain intact.

The three inspected deadline tests cover owner-monotonic expiry despite Unix
rollback, Unix expiry, monotonic regression, an elapsed Instant while supplied
owner clocks remain valid, initial expired/nonpositive lifetime and checked
overflow. The implementation run retains a missing-API RED at `0b06714` and
reports 24 focused campaign tests GREEN. This reviewer inspected source and tests
but did not rerun them. Guard placement plus these boundary tests establish this
source correction; they are not an injected long-object-work end-to-end run.

This recheck added only this review resolution. No product edits, builds, tests
or coverage were performed. Concurrent receiver error/fence/bounds test extensions
remain outside this exact test identity until completed and reviewed. Deadline
guards prevent phase entry/completion after expiry; they do not preempt ongoing
object work or guarantee that time cannot pass during blocking transport I/O.
Full C8, explicit resume and automatic request retransmission remain incomplete.
