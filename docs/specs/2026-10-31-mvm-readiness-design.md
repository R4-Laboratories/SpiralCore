# ADR: canonical MVM envelope adapter before the guest service

Date: 2026-10-31. Status: selected implementation boundary under the user's
approved mailbox integration plan and current instruction to finish standalone
work and move toward MVM. This document is a contract, not a completed test.

## Decision and scope

Freeze SpiralCore's current codec-selection policy at `b2dacc6f22d36454c1b8d6ce9bab654e71e68d66`.
Implement a separate `crates/spiralcore-mvm-adapter` Rust crate using the actual
MVM canonical mailbox implementation, with an executable host conformance demo.
Reuse `WarmSession`, existing frame encoding and durable `Endpoint`; do not
create a replacement codec, cipher, envelope, receipt, network service or port.
The first running delivery uses MVM's bounded in-memory peer transport and real
signed/encrypted request/reply envelopes. It is not a live guest/vsock or WAN test.

Alternatives: another speculative codec optimization delays integration without
resolving its boundary; a raw socket/vsock prototype would bypass MVM's unfinished
service and authorization contract. The canonical host adapter is independently
useful and can be replaced at its transport seam when the upstream service exists.

## Live upstream pin and callable surface

MVM `main` was resolved on this date to
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b`. The following are source findings at
that exact pin, not assumptions based on issue labels:

| Surface | Confirmed API / constraint |
|---|---|
| [Canonical envelope](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-core/src/mailbox.rs) | `SealRequest`, `OpenContext`, `decode_envelope`, `open_envelope`, `sign_capability`; open verifies cryptography, identity, content, capability, epochs and replay before returning plaintext. |
| [Outbound authorization](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-core/src/mailbox/transport.rs) | `prepare_mailbox_envelope` → `authorize_outbound(OutboundAuthorization)` yields `AuthorizedMailboxFrame`; its bytes are private. `PeerTransport::connect`, `PeerChannel::try_send`, and `InMemoryPeerTransport::drain` provide the executable host route without bypassing that type. |
| [Typed wire contract](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-contract/src/protocol/mailbox.rs) | Call requires `reply_to`; Reply forbids it. Content type and version are explicit. Max plaintext is 1 MiB and max encoded envelope 8 MiB; the adapter imposes smaller limits. |
| [Guest services](https://github.com/tinylabscom/mvm/blob/3104a12c5b071a6851323fa8115d2afaf3f0bb8b/crates/mvm-net/src/channel.rs) | `GuestService` still has no Mailbox variant. FlowMux's NetworkFlow is separate. |
| [Local service #4629](https://github.com/tinylabscom/mvm/issues/4629), [Iroh #4633](https://github.com/tinylabscom/mvm/issues/4633) | Both open when refreshed; guest service and cross-host transport acceptance remain unchecked. They require standalone MVM, independent queues and canonical opaque envelopes. |

`mvm-core` exports mailbox unconditionally and has empty default features. Pin it
and `mvm-contract` as Git dependencies at the same exact revision, with default
features off and contract `protocol` enabled. This does not build mvm-agentd,
mvm-vmm, Iroh, a hypervisor or the whole MVM workspace. Rust 1.97.1 matches the
available toolchain. macOS core dependencies include its Mach-O helper and Apple
security libraries; Linux avoids those. Build footprint remains measured rather
than estimated. Use a crate-local lockfile and the existing shared target with
incremental compilation off. Do not vendor/reimplement upstream crypto.

## Adapter contract

The payload is exactly one existing `SCMOBJ01` encoded `Frame`, no JSON wrapper.
Its contract remains [object mailbox application v1](../contracts/spiralcore-object-mailbox-v1.md).
Use canonical content type `application/spiralcore-object`, version 1. Reject
plaintext above 65,536 bytes before sealing and before frame decoding. Reject
encoded envelopes above 262,144 bytes before canonical decode. Admit one frame
at a time to each bounded transport queue with the same 262,144-byte cap.
An input batch from `WarmSession` is exchanged as an ordered sequence of these
individual request/reply messages; output order and frame identity remain intact.

Each direction owns distinct sender identity, signing key, capability, expected
recipient/mailbox, signed route binding and encryption context. Requests use
Call and an explicit reverse `ReplyRoute`; responses use Reply with no reply route.
Use the canonical Send operation authorization on each outbound/open direction,
as in the pinned upstream mailbox crypto fixture. The reverse capability admits
Reply messages; it is not inherited from request authority. The host harness
controls both endpoint policies; it does not claim to implement MVM's general
workload-facing Receive/Reply admission, call scheduling, rate limits or broker.

At adapter construction and every open, require `required_operation == Send`,
`acting_principal == expected_sender`, and the exact singleton content type above.
Restrict the forward capability to the singleton Call kind and the reverse
capability to the singleton Reply kind. This lets canonical capability checking
enforce the otherwise unreturned inner kind. A mismatched or broader fixture is
refused; it is not silently narrowed after opening.

Sequence for every request, and symmetrically for every reply:

1. Bound and encode the existing SpiralCore frame.
2. Construct the canonical header from trusted endpoint configuration, current
   policy and fresh message/delivery/correlation identifiers; never from payload
   assertions. Call correlation identifies this attempt; SpiralCore transfer ID
   and request sequence identify application work.
3. Call `prepare_mailbox_envelope`, then `authorize_outbound`; send only the
   resulting authorized value through canonical `PeerChannel::try_send`.
4. Drain the canonical ciphertext bytes in the host demo; bound, decode and
   `open_envelope` under current recipient expectations and a bounded replay set.
5. Only after successful open, decode the application frame and derive its
   `endpoint::Context` from the verified expected tenant/domain/sender/recipient/
   mailbox, capability and epochs. Encode fixed-width IDs as lowercase hex;
   include a domain-separated digest of the verified capability grant in
   `authorization_scope`, binding issuer, revocation and grant restrictions.
6. Invoke `Endpoint` under a current exact-context `Authority`. This authority
   must recheck the current policy/epoch/revocation snapshot on every operation
   and immediately before publication, using Endpoint's existing hook; an earlier
   successful open does not indefinitely authorize later work. A successfully
   opened message is not object publication. Construct an authenticated reverse
   envelope only from the actual endpoint reply. The existing client verifies
   transfer ID, request sequence and publication ObjectId before promoting state.

No code path may call `Endpoint` on merely decoded/unauthenticated plaintext.
Route or digest possession grants no authorization. Wrong recipient, revoked
capability, stale epoch, tampering and unsupported content fail before cache
access. One receiver/Endpoint owns the existing aggregate storage/resident budget;
constructing more adapters does not grant independent object quotas.

The upstream `open_envelope` returns payload only, not a verified inner header.
Consequently this first adapter does not promise general MVM call correlation,
reply-route recovery or a durable mailbox service. Its controlled canonical
headers and the existing SpiralCore frame correlation establish the local
conformance exchange. A live service must expose its own verified request/reply
handle rather than trusting a caller-supplied header beside an opened payload.

## Retry and crash boundary

Replay state is bounded and explicitly process-local in this host profile;
capacity exhaustion rejects admission rather than silently evicting identities.
Use at most 4,096 accepted identities per direction in one bounded demonstration.
MVM open records an identity before returning payload. Injected interruption
after open but before endpoint handling must leave no object publication.
Replaying that same envelope fails; the application can explicitly retry the
same SpiralCore frame in a new canonical envelope. If handling committed but its
reply was lost, a new-envelope retry returns the existing idempotent application
result. After endpoint restart, explicit `WarmSession::recover` revalidates the
publication and missing partial progress is restarted under the existing API.

This is application retry conformance, not durable accepted-message recovery.
Do not emit or claim `AcceptedDurable`, exactly-once execution, crash-persistent
replay protection, or automatic retransmission. A future MVM inbox must commit
replay/admission and recoverable accepted-message state atomically. Guest boot,
execution/policy epoch, endpoint identity and routing changes must be supplied
by that trusted runtime, including for a small orchestrator; a CID or host port
is not tenant identity.

## Evidence and delivery boundary

The demo must execute the existing prepared warm selector through the canonical
exchange, with real disk publication and stopped/reopened exact comparison. It
also runs prepared whole-object zstd under the same envelope path as a control.
Record application frame bytes, canonical envelope bytes in both directions,
per-operation process CPU and total elapsed, plus setup and seed costs. The old
600-byte HTTP-body claim remains true of its original transport; canonical MVM
envelopes add overhead, which must be measured and not relabelled 600-byte wire.
Use synthetic 1 MiB and 16 MiB objects, cold seed, localized update and unrelated
replacement, then a mixed persistent sequence. Keep raw observations and exact
source/executable identities. A host smoke demonstrates interoperability, not
new WAN performance or guest isolation.

Required negatives are malformed/oversized frames, ciphertext/signature tamper,
wrong recipient/content/capability, revocation and epoch change, duplicate
envelope, application duplicate with a new envelope, cancellation/backpressure,
open-before-commit interruption, lost publication reply and cache restart.
No key or plaintext payload is retained in telemetry. No FIPS validation is
claimed by using SHA-256/AES/Ed25519 or upstream crypto.

Live MVM delivery remains gated on #4629's dedicated service and a supported
authorized transport/receive seam (#4633 for Iroh). This implementation prepares
that handoff without editing MVM upstream, allocating a port or repurposing
FlowMux 5253. Subsequent acceptance is one guest/host, two guests under a small
orchestrator, two actual hosts, then three-peer recovery with the same contract.
