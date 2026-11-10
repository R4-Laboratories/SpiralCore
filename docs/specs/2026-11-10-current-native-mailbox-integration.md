# Current native MVM mailbox integration

Status is an implementation decision anchored to the inspected upstream MVM checkpoint `d27a20cb1a12a7c21b253a3bf6b7579925a7d1f3`. The [source delta](../research/2026-11-10-current-mvm-mailbox-delta.md) records implemented behavior and remaining #4629 requirements. Reuse its signed-plan MailboxGrant, guest listener 5257, per-VM host helper, derived-key provisioning, audited host receive and canonical encryption. Historical fork `5722264e` is comparison evidence. Its old installed-service source closure is not the port target.

## First source slice

Expose `OpenedMailboxEnvelope` in canonical `mvm-core::mailbox`, with private header/payload fields, borrowed `header()` and `payload()`, and consuming `into_payload()`. Add `open_verified_envelope` with the same authority, keys, context and replay parameters as current `open_envelope`. Move the existing full validation path into it and return the authenticated header and payload only after every canonical check succeeds. Keep `open_envelope` delegating to it and consuming payload, preserving caller behavior and replay recording exactly once. This proof establishes verified bytes under its context, not application admission or durability.

At `transport::authorize_outbound`, derive required operation from the private prepared header. Call and Cast require Send; Reply requires Reply. Verify the supplied context matches that operation, then execute all current identity, time, capability, route, binding and ciphertext checks. A reply capability never authorizes a Call or Cast. Preserve destination-scoped epochs and direction-specific authorization; the two mailbox owners can have different current epochs.

Reuse current `AuthorizedMailboxFrame::as_bytes()`. Upstream already provides it. The adapter can drop its redundant pending/frame invariant check because `ChannelLimits::new` enforces it and its fields are private. Production sending can continue verifying `prepared.envelope()` canonically. Do not add a prepared-header getter solely to preserve fixture convenience.

## Native producer and return mapping

The existing outbox format contains only recipient plus body, and deletes the claimed file before its verdict. It lacks a stable request identity or kind/correlation metadata. Preserve that legacy interpretation explicitly. A reliable Call/Cast/Reply mapping needs a distinct canonical MVM local-service version admitted by the signed launch, rather than guessing a magic prefix inside existing recipient bytes.

Define the version and its data-only request/result types in MVM's protocol module and use its original guest listener/host helper. The producer request carries a bounded request ID, kind, content type, recipient and body, plus correlation/causation for admitted replies. The host binds it to the actual VM, grant, boot and epoch. A guest-supplied request ID is an idempotency key within that scope, not authority. The host supplies cryptographic identities, validates the signed grant and holds the encryption/signing keys.

Retain the guest's claimed request until positive local admission is received. A full queue, disconnect or ambiguous verdict leaves the same request available for retry. New producer writes cannot overwrite a claimed request or an already occupied bounded slot. Use one bounded request slot initially, explicit Full backpressure, and retain identity across reconnect. Conflicting reuse of an ID with different immutable recipient/kind/type/body/correlation refuses. Do not resolve retry identity from ciphertext alone because canonical sealing can use a fresh nonce.

The host retains the first prepared canonical envelope and deduplication disposition under the same bounded per-VM service owner. Admission and reply caches consume shared host, tenant, sender and mailbox credits. Authentication and current epoch/revocation checks run before deduplication can acknowledge a retry. Retries after an epoch change require current authorization and cannot satisfy an old Call. Start with explicit ephemeral acceptance and at-least-once delivery. Durable acceptance requires a separate implemented journal/crash contract.

Add an admitted return operation on the existing dedicated mailbox service for host-to-guest responses and control results. It must preserve independent mailbox queues and use the canonical Reply kind, capabilities and signed destination receipt types. Do not route it through FlowMux or command execution. Service framing/version negotiation belongs to MVM; SpiralCore payloads remain existing object-v1 messages. Reserve control capacity within the same aggregate limits so queue saturation cannot suppress admission results or cancellation.

A Call has a tracked correlation, monotonic deadline and cancellation state. Only its authorized peer's verified Reply with matching correlation/causation can complete it. Late replies release retained payload and cannot match a later Call. Cast returns local admission and causes no application Reply. A signed destination receipt establishes its declared admission outcome; application Reply and object verified commit remain distinct events. Receipt control messages cannot recursively require receipts.

## Qualification boundary

The first integration witness exercises upstream's actual one-way outbox, helper and audited host receive. Label it as that endpoint. Bidirectional object-v1 acceptance follows after the admitted return mapping and retry retention exist. It requires exact object readback, actual traffic both ways, setup/CPU/verified total time, lost-verdict recovery, queue-full backpressure, revoked/wrong-recipient refusal, late Reply disposal and original helper shutdown/reap.

No grant, backend capability or service version means refusal before guest work. Preserve current configured resource limits, service isolation and SmallV1 bounds. The completed contribution still requires the existing supported-role, WAN and fleet-capacity gates. This ADR specifies the next implementation sequence and does not assert those gates passed.

Upstream subsequently advanced to b3dd575b4d95355358f8017d159c69a331bc9c35 with CLI first-use runtime preparation and no mailbox-file delta. Preserve this ADR's d27 source pin. Before native execution through a release CLI, account for or preprovision its signed-runtime acquisition, cache assembly and adoption writes under the original installer owner. Record that work separately from mailbox transfer metrics, with actual lifecycle costs retained.
