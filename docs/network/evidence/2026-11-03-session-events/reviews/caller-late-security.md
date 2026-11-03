# Caller Late/Duplicate security review

Date: 2026-11-03. Verdict: approved for this caller-only integration; no blocking
finding in the reviewed source. Manual scoped review; no builds or runs executed.

| Reviewed file | SHA-256 |
|---|---|
| `campaign/canonical.rs` | `6490c076c54a1725e9042ff8bdb975dd6a48e0bcbf1bcba2efdb35b227e62952` |
| Adapter `receipt.rs` | `f72d2f84a675cd7221dd94fa4a10c7d84331fb7c04a95d7200d782795d07c305` |

The canonical receipt-return path authenticates normal frames before disposition
handling. Late sends its actual prepared receipt without tracker correlation or
current-progress mutation. Duplicate with no registered request likewise sends
only its actual receipt; a registered observation must match the current handle
before changing current Reply state. AcceptedEphemeral requires registration.
No generic Unauthorized exception is swallowed. Known mismatch, expiry, conflict
and verification errors remain failures in the optional tracker API.

Duplicate does not invent an acceptance level, including replay of an identity
originally classified Late. Canonical processing of a known Late FIFO position
may unblock already authenticated current work; the ordinary fresh-deadline and
canonical poll gate still controls completion. Event/control budgets, bounded
Reply-wire retention, service-owned charged payload, acknowledgment uncertainty
and original request identity remain intact. No new wire or receipt-of-receipt
is introduced.

Receiver repeated-Call handling, cached Reply replay, explicit resume and timed
automatic request retry are not approved as completed by this caller review.
The companion receiver contract is a next implementation slice, not runtime
evidence. Full C8, long-retention, WAN and larger-window gates remain open.
