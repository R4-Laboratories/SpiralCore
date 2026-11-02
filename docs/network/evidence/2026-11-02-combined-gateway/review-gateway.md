# Independent combined gateway review

**PASS — no actionable finding in the bounded gateway change.** Reviewer:
`/root/mvm_boundary`, reviewing root-authored facade/service code only. The
reviewer's transfer/session implementation is excluded and receives a separate
cross-author review. The active roster checks for code-reviewer, security-reviewer
and rust-reviewer all passed; no conflicting in-progress file claim was present.

Reviewed against the committed combined contract:
`docs/specs/2026-11-02-combined-gateway-design.md`.

- `ObjectSender::with_combined` moves one prediction into one boxed combined
  session. Dispatch moves target bytes and receipts without cloning a second
  session/prediction. Generic construction without a prediction and generic
  target seeding both refuse; checkpoint/schema seeding remains explicit.
- Existing namespace checks execute before every facade operation and permanently
  drop prepared state on mismatch. Invalidation dispatch reaches the combined
  session. No new caller-supplied identity, remote-selected namespace, authority
  bypass, implicit reoffer or ACK interpretation is introduced in the facade.
- Service configuration requires schema plus checkpoint/recipe for Combined,
  rejects inappropriate auxiliary combinations and generic `seed`, and retains
  bounded groups/operations/file reads. Runtime construction and both codec
  preparations occur inside paid group setup. Only one group's sender remains
  active at a time; auxiliary input buffers drop after construction.
- Target and auxiliary operations use the existing authenticated channel,
  mode-specific engine offers and validated publication checks. Separate schema
  and checkpoint lengths now correctly attribute the two setup operations. The
  receiver's durable publication and exact reopened-readback path is unchanged.
- Combined receipts preserve evaluated/pruned costs, readiness at selection,
  selected mode and the exact-repeat marker. Costs remain labeled as the existing
  HTTP-body model, not QUIC/IP traffic. Actual publication-path attribution still
  distinguishes a cache hit from execution of an offered decoder.
- Timing starts before input loading, independent expected-identity hashing and
  sender dispatch, and ends after validated publication. Thus the engine's cheap
  repeat avoids its own residual/hash work; the gateway still pays its independent
  expected-byte hash for every arm. Deferred reporting and complete service timer
  boundaries are unchanged. The group's `retained_charge` is sampled at setup,
  not a process-RSS or later rolling-base peak measurement.

Observed receipts, without duplicate builds: facade RED exited101 for the missing
new API, followed by `facade-green-01` exit0; service RED exited101 for rejected
Combined configuration, followed by `service-green-01` exit0 with both focused
cases passing. The real paired QUIC case switches among Runtime, rolling Sparse
and Raw repeat probes within one connection and reopens exact objects. Existing
negative transport/namespace/output-preservation behavior is retained; final
integrated checks and campaign measurements remain separate parent-owned gates.

Reviewed SHA-256 identities:

| Source | SHA-256 |
|---|---|
| `crates/spiralcore-gateway-core/src/object.rs` | `5f6adb80d2937569d86855ac1f353f6d2b8e491252e0de2b5da7d8ccdc240db9` |
| `crates/spiralcore-gateway/src/object_service.rs` | `a9107e56ffe5b992884fffff5bd98cf3db5657b94c30e7c4e4d8650a74fb48d5` |

No production edit, build, new test or broader audit was performed for this review.
