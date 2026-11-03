# Receiver replay security/source review

Date: 2026-11-03. Initial narrow review: deadline correction required before
approval. Manual source review only; no builds or runtime tests executed here.

| Reviewed file | SHA-256 |
|---|---|
| `campaign/canonical.rs` | `e1c27fb594be4f7ecf2f7caf563279c6900741f5e1a36d467d04622fb0b3d6bd` |
| `campaign/session.rs` | `c99ef4eea2a7a69ec8e07e448a732c091e393ec89b207dded6c91933c38c9446` |

The receiver latch is set before initial input and cleared only after matching
receipt/cache release and successful completion bookkeeping. Errors retain the
latch, preventing a later invocation from consuming another Call. The active
exchange has 16 incoming events including its first Call, and eight Reply-send
invocations including the first, counted before transport. No resume resets them.

Repeated-Call handling authenticates through retry_reply_prepared and compares
the exact current cached Reply before canonical admission. Thus another Call
cannot gain state after the original active Call credit has been released.
The repeated Call must receive its actual Duplicate receipt. After receipt I/O,
fresh request/cache policy verification and tracker registration must recover the
same exact Reply and SendHandle before replay. No object execution, fresh Reply
sequence, resealing or new wire authority is introduced. Earlier/duplicate
controls take only the current tracker handle and cannot release another cache.

Blocking deadline contract issue in this snapshot: after application_phase,
the first Reply preparation/registration/send path does not check the fixed
receiver deadline. A request arriving near the initial receive deadline can
remain canonically authorized after that earlier local deadline; long application
work can then emit its initial Reply outside the promised exchange bound.
Require a fresh fixed-deadline/original-Unix-expiry gate before initial Reply
preparation/send, and before initial receipt send if its fixed cap has elapsed.
Retaining/checking the admission's trusted monotonic deadline also avoids treating
an Instant anchor sampled after broker time as the canonical monotonic identity.
This finding concerns the exchange deadline; it does not assert a grant bypass.

Critical focused gates still needed before release: earlier/duplicate controls,
event/send saturation, different-Call no-admission, uncertain receipt/Reply sends,
deadline after application work, and a second handle refusing without reading
after each error. The observed primary RED/GREEN is implementation-owner evidence.
Unexecuted gates must remain explicit. Receiver resume, autonomous idle expiry,
timed request retries, full C8, long-retention and WAN qualification remain open.

## Deadline correction resolution

Reviewed correction `f2b1cc8`: the deadline blocker above is resolved. No further
blocking source issue was found in this narrow recheck. Product/test hashes:

| File | SHA-256 |
|---|---|
| `campaign/canonical.rs` | `8540ea581d6b7e323b6502e945031cb187cdf47ead20a26eaa242fa8e0d8969c` |
| `campaign/session.rs` | `8d8f27e50fe0a6dca43c317b26f8a3eeffce5ab2b60f6f060d442b2153010e13` |
| `campaign/session/tests.rs` | `64cf309da6732e595dbd8b6979ff414c26fd71e1be92b70e89a44c0ed8509bf4` |

The private ExchangeDeadline fixes the initial Instant cap, original signed Unix
expiry and checked owner-monotonic expiry at authenticated admission. Equality
at any expiry fails; owner-monotonic regression fails. Duplicates cannot refresh
these fields. The receiver checks the helper before initial receipt reservation,
application work, post-work Reply preparation/registration, fresh initial send,
event processing/cache release, duplicate receipt reservation and fresh replay.
Thus application work cannot authorize the initial Reply after the fixed receiver
cap merely because its canonical grant is still otherwise valid.

The inspected boundary tests independently cover Unix expiry, monotonic expiry
despite Unix rollback, elapsed Instant, monotonic regression, nonpositive lifetime
and checked overflow. The 60-second pre-bound synthetic interval avoids a tight
scheduling assumption without extending a product profile; the separate elapsed
Instant case retains its short deadline. The implementation owner reports the
missing-API RED at `0b06714` and 24 focused campaign GREEN tests. This reviewer
inspected source and tests, but did not independently execute them.

This resolves source approval only. The separately owned receiver error/fence/
event/send qualification guards remain execution gates until their own results
are recorded. A guard refuses entry/completion after expiry; it does not preempt
ongoing application work or guarantee time cannot pass during a blocking write.
No product files or builds were changed by this review. Full C8 and the broader
release/goal gates remain incomplete.
