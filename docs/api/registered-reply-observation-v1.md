# Registered application Reply observation — additive Rust API

Reviewed architecture: [C8 contract](../specs/2026-11-03-persistent-session-state.md#approved-c8-optional-registered-reply-observation).
This is an additive local Rust interface in the shared adapter, with no new wire
message, serialization, grant, receipt type or protocol version.

```rust
pub fn observe_registered_application_reply(
    &mut self, binding: &MailboxBinding, encoded: &[u8], now: ServiceTime,
) -> Result<Option<SendHandle>, ServiceError>;
```

The binding, trusted time, canonical authentication, size/kind/content constraints,
current context and registered deadline/immutable Reply checks remain those of
`ReceiptTracker::observe_application_reply`. Exclusive mutable tracker ownership
is required. The returned handle is local metadata, never admission authority.

- `Some(handle)`: one complete registered request match, with the existing
  application-replied fact and commitment updated. It does not invent a
  destination receipt, acceptance level, application delivery or durability.
- `None`: genuinely unknown request under the authenticated tenant/domain/peer.
  Apart from existing expiry maintenance, allocate no identity, payload or
  observation state. This is not a signature, policy or parsing failure.
- Known correlation OR causation with failed content/capability/reply-address or
  other complete-match constraints remains `Unauthorized`. Apply this guard
  only when no complete match exists; another request may share correlation.
- Ambiguous complete matches remain `Conflict`; known expired/conflicting
  requests and all verification/policy/clock/limit/state errors remain errors.
  No generic error is converted into `None`.

The existing strict API calls this observer once and maps `None` back to its
existing `Unauthorized`, preserving compatibility and one canonical verification.
No new resource ceilings or rate exemptions apply.

The caller first performs canonical service/receipt admission. An authenticated
`Late` Reply receives its actual prepared negative receipt and contributes no
current progress. A `Duplicate` may be unmatched, including replay of a formerly
Late identity; `None` permits its actual receipt only. `AcceptedEphemeral` requires
`Some`. Only a handle equal to the current pending send contributes current Reply
state. Every frame consumes the existing 16-event allowance; no ACK is generated
for receipt-control frames, and no retry is introduced.

Contract tests cover matched/unknown Replies, unchanged strict behavior, invalid
authentication and known semantic mismatch, expiry/commitment conflict, unchanged
unknown-state accounting, and caller Late followed by replayed Duplicate. These
are functional requirements; executed results belong in the evidence packet.
