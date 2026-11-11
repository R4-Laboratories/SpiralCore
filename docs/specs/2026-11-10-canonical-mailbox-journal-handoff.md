# Canonical mailbox journal handoff

Status is a proposal for coordination with Ari's first bounded [#4630](https://github.com/tinylabscom/mvm/issues/4630) module, following the ownership agreement on [#5328](https://github.com/tinylabscom/mvm/pull/5328). It does not enable durable acceptance. The first reliable local-service profile remains ephemeral and currently refuses activation until its runtime implementation lands.

## Ownership

Ari owns a new MVM-local canonical ciphertext journal, its bounded dedup/recovery module, and focused tests/docs. SpiralCore's contribution stream owns the four #5328 API files, the existing host service/launch/process files, guest-local framing/spool, typed operations, aggregate budgets and lifecycle integration. Shared public contract changes are coordinated before editing. Neither side creates a new envelope, signature domain, transport port or alternate authority.

## Admission input

The service reserves original aggregate host/tenant/sender/mailbox credits and validates current authority before requesting journal admission. Received raw headers, digests, caller-provided flags and addresses never constitute verified admission.

`OpenedMailboxEnvelope` alone is insufficient as a ciphertext-only proof. It contains plaintext and a verified header, but does not retain the exact original outer envelope or its ciphertext commitment. Do not hand that object, its payload, or a fabricated `verified: true` DTO to the journal.

Propose one canonical-core owned capsule, `VerifiedMailboxAdmission`, whose private fields retain the original `MailboxEnvelope` by value and the canonical verified-open result. Its sole factory takes ownership of the bounded envelope and delegates the existing complete canonical verifier once, including its original replay decision. No second ciphertext clone or alternative crypto verifier is required. Its plaintext accessor is available only to the service-facing API. A narrow borrowed `JournalAdmissionView` exposes the exact accepted canonical envelope/ciphertext, immutable authenticated header fields and the verifier-derived ciphertext commitment, with no plaintext accessor or public constructor. Safe Rust callers cannot mutate the owned envelope while a view exists. The capsule is neither Clone nor Deserialize.

The view binds tenant, authorization domain, mailbox, recipient, sender, sender boot, message ID, capability/key identity, canonical execution epoch, expiry and the exact stored ciphertext representation. Reuse the actual canonical types and epoch meanings. The present header has one execution_epoch and a sender_boot; do not invent separate source/destination epoch fields in its wire. Additional locally validated owner/policy generations remain explicitly local context. A proof records verification under that context and is not permission to bypass later admission or send checks.

The journal indexes the canonical immutable message identity within its authorization scope. It compares complete immutable accepted record bytes for Identical versus Conflict; a digest is an index rather than replacement for that comparison. Changes to transport-mutable routing fields need an explicit canonical comparison rule before supporting them. Until then, preserving exact admitted bytes is the conservative contract.

## Journal output and receipt authority

The journal API returns typed New, Identical or Conflict disposition plus a bounded pending/committed record reference. New/Identical is not durable success. A separate private `DurableCommitEvidence` is issued only after the record, dedup disposition and required durability metadata have committed, and the trusted external rollback anchor has committed and been verified against that exact journal head. This evidence is an in-process handoff, not a new wire receipt.

Anchor failure or an ambiguous commit remains Pending/Unavailable. Retain the record and reservation for bounded recovery/query. Do not truncate or delete an ambiguous record and then retry it as new. Recovery verifies chain/head, anchor and dedup state before restoring admissions. Repeated identical requests recover the original disposition without appending a second admitted value. Bound pending operations and recovery work; exhaustion backpressures new admissions.

The original service alone signs the existing canonical `SignedDestinationReceipt`. AcceptedDurable requires verified commit evidence and recoverable key custody as well as current admission/receipt authorization. It must bind the original delivery/message identity, address, epoch and ciphertext commitment. Identical retries use the canonical Duplicate semantics referring to the original terminal disposition. Conflict creates no new accepted record. Object-v1 commit, local producer Retire, application Reply and application processing remain independent events. Receipt carriers do not request recursive receipts.

No journal or anchor I/O runs under the broker dispatch or service state mutex. Reserve and capture generations under a short dispatch, perform slow I/O outside it, and reenter to validate the matching transaction, current authority and commit evidence before exposing a result. Revocation or a generation change after a write cannot erase history or justify an acknowledgment under stale authority. Retain a committed record/terminal disposition for reconciliation even when the current request can no longer be acknowledged positively.

## History key custody

The current separate encryption root plus host-local per-launch secret is not a durable history recovery provider. Canonical teardown attempts logical overwrite/unlink of the launch secret on stop/replacement; failures remain explicit. A signing seed is not a fallback decryption authority. Durable admission remains disabled when history custody is unavailable.

Ari can own the initial `HistoryKeyCustody` trait proposal alongside the journal contract. Its operations must identify tenant/domain, mailbox, canonical epoch, encryption algorithm and key ID; produce a bounded recoverability lease/reference before durable acceptance; and support recovery, expiry, rotation, revocation and retirement under the intended journal retention policy. The journal receives opaque custody references, never plaintext keys or the raw launch secret. The actual encryption/wrapping scheme, provider trust boundary, external anchor and deployment persistence guarantees require review before implementation claims. This proposal chooses no key scheme and makes no host-blind or physical-erasure guarantee.

## First bounded module tests

Ari can implement the module behind a narrow admission-view adapter and fake anchor/custody providers while the canonical-core capsule is coordinated. Production constructors remain inaccessible; test-only builders must be feature-scoped and unable to appear in ordinary builds. Initial tests cover New/Identical/Conflict, exact record/dedup binding, torn append, crash before/after journal commit and anchor commit, wrong/stale anchor, ambiguous result recovery, bounded full state, authorization-generation mismatch and unavailable/retired custody. No AcceptedDurable result or signed durable receipt is possible with a failed/unverified anchor or missing custody lease. Real provider and service integration qualify separately.
