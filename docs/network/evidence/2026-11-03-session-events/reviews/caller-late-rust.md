# Caller Late/Duplicate Rust and code review

Date: 2026-11-03. Verdict: **APPROVED for this narrow caller slice; no blocking
Rust or correctness findings.** This is an independent source review, not full
C8 completion or executed qualification. No product edits, builds, tests or
coverage runs were performed by this reviewer.

## Exact source identities

Base checkout HEAD during review: `815f7459f25227b2361a86a735852cb77750031a`.
The caller change was uncommitted. SHA-256 identities:

```text
6490c076c54a1725e9042ff8bdb975dd6a48e0bcbf1bcba2efdb35b227e62952  crates/spiralcore-mvm-live/src/campaign/canonical.rs
8f34a3ac692993adb6ff7fa0035ee302fd3cc8dd12d9b489f84889d5beee3392  crates/spiralcore-mvm-live/src/campaign/canonical/late_tests.rs
f72d2f84a675cd7221dd94fa4a10c7d84331fb7c04a95d7200d782795d07c305  crates/spiralcore-mvm-adapter/src/receipt.rs
ab032749f8f9f0513bde7f246d4b02319a6e11d820a4e263c938524d5dc92a5b  crates/spiralcore-mvm-live/src/campaign/session.rs
6847dcbbddae3e03f39b986d070fd686763af15f63f85cb68c32e721c9042a6e  docs/api/registered-reply-observation-v1.md
```

The additive observer and pending ownership seam retain their previously
reviewed identities. Review scope was the caller disposition match and the
focused `late_tests.rs` regression, against the approved registered Reply
observation contract and C8 addendum. `git diff --check` passed for these files.

## Findings and reasoning

- The caller obtains both the disposition and the actual prepared receipt from
  canonical `ReceiptReturnConfig::admit` before acting. The untrusted outer key
  selects a verification path but supplies no authenticated disposition itself.
- `Late` skips correlation and contributes no current facts. The caller sends
  its actual canonical negative receipt. Unknown `Duplicate` likewise sends only
  its actual duplicate receipt; it is not upgraded into acceptance or a Reply.
- AcceptedEphemeral requires `Some` from the approved optional observer.
  Verification, known semantic mismatch, expiry and commitment errors propagate.
  `Some` contributes current progress only through the existing exact
  `SendHandle` comparison in `PendingCall::observe_reply`.
- Late/unknown-Duplicate receipt sends leave current ACK progress unchanged.
  Matched current Replies retain Needed/InvokedUncertain/Sent handling, with
  transport outside broker dispatch and no stored self-referential permit.
- Every incoming frame still consumes the cumulative 16-event allowance. The
  original deadline, current receipt facts, service-owned payload and canonical
  generation checks remain in the ordinary readiness/FIFO poll gate. A Late
  event may release an authenticated FIFO position; completion afterward still
  requires the previously established current facts and current service payload.
  There is no new sequence allocation, fallback authority or automatic retry.

The regression uses a real capacity-one canonical pair. It sends one genuinely
unmatched authenticated Reply at Reply FIFO sequence 1, replays its exact prepared
envelope, then sends the valid current Reply at sequence 2 in the same scope.
The peer checks the actual signed Late, Duplicate and AcceptedEphemeral outcomes;
negative observations cannot set its tracked acceptance fact. Exact current
application bytes, one completed Call, current Reply cache release and directional
wire/framing counters are asserted. Raw shutdown handles and an owning guard wake
and join the peer on failure. This fixture does not claim an executed source Call
or cached application Reply for the unsolicited negative input.

Existing RED/GREEN logs under `../validation/late-duplicate-{red,green}.txt` are
separate execution evidence supplied by the implementation run; this reviewer
did not rerun them. Receiver repeated-Call handling, cached Reply replay,
uncertain-send recovery, automatic retry and the remaining full C8 gates remain
outside this approval and require their own implementation/review/evidence.
