# Canonical TCP payload seam proposal — not activated

Historical proposal below; root subsequently authorized these changes and the
network policy/admitted-plan materialization extras. The generic hook, optional
policy and refusal guards are now implemented in the owned MVM worktree; actual
TLS factory activation remains in progress. See [component30](component30.md) for
current qualified boundaries. Original source pins below remain the read-only
proposal baseline, not hashes of current edited files.


Read-only MVM source: `7d031c7f3fbeef62826712d2976a764c62cd5852`.
No MVM source has been edited by this task. The current source has no public
payload mapping hook and no explicit cooperating-SpiralCore-peer selection.

The minimum hook touches these files, subject to lead approval before edits:

1. `crates/mvm-hostd/src/supervisor/flowmux.rs`: optional typed payload adapter
   on FlowMuxAccept/Session; None preserves existing opaque socket construction.
2. New `crates/mvm-hostd/src/supervisor/flowmux/payload_mapping.rs`: public
   typed admitted-TCP facts and payload socket interface. Facts are minted only
   after canonical admission/tool/transform decisions and carry actual connected
   peer, target, route, stream and authenticated session identity. The adapter
   consumes that same TcpStream. It cannot replace its destination or dial one.
3. `crates/mvm-hostd/src/supervisor/flowmux/open_flow.rs`: invoke that adapter
   after connect_first_admitted, before confirm_stream/send_opened/relay. An
   explicitly required mapping failure retires the reservation and emits the
   canonical refusal/audit; no fallback. Typed transform paths stay unchanged.
4. `crates/mvm-hostd/src/supervisor/flowmux/socket.rs`: explicit mapped-payload
   backing with Read/Write/clone/shutdown behavior. This is a reviewed extension
   of the current two-case enum, not an implicit arbitrary transport vtable.

This hook alone is not activation or permission. The actual endpoint projection
is in `crates/mvm-hostd/src/bin/mvm-network-endpoint.rs` at
FlowMuxEndpointProjection::accept_params. It currently forwards gate, recorder,
substitution, VM-wide budgets and signed ingress mappings, with no cooperating
peer field. Production activation therefore also requires a reviewed trusted
selection ABI/materialization path bound to the admitted signed plan and its
current policy/execution identity. A destination being admitted, matching a
digest, a successful probe, or a guest payload assertion is insufficient.

The new SpiralCore crate is an application mapping component, with no route
authority. ObjectChannel already transports the shared Warm/Runtime/Combined
protocol with canonical Published verification. Duplex adds bounded simultaneous
request/response handling, artifact-versus-application delivery and role-bound
stream identities. Canonical current authority, original credit/rate/audit/reset
behavior and actual endpoint tests remain required before activation. A TCP grant cannot activate
UDP; its canonical association and datagram mapping remain separate.

Source hashes read for this proposal:

| File | SHA-256 |
| --- | --- |
| flowmux.rs | e50318bc674ff07211c57117f39526b35d8190d070f49e537398220a39e168a4 |
| flowmux/open_flow.rs | e05f6c5f760228130c4e75e0040dec3da220deec69aaf12a71fdf200487b46d3 |
| flowmux/socket.rs | 60c0cd1486d8684a72998d858fcbc5f1971ea70990ad933f919d8f61c1d595fd |
| bin/mvm-network-endpoint.rs | 9c8117717f3e40903d6e4ec6879dd4449f95855038b6c0702a1ff04c23676013 |

## Signed selection and actual peer authentication

Existing `mvm-contract/src/peer.rs::PeerBinding` contains only canonical peer
name/port and resolved host address/port. It carries no remote TLS/SPKI identity.
No reusable remote pinned-peer verifier was found in the inspected FlowMux,
endpoint and egress-CA path. The existing egress CA serves TLS termination for
guest traffic, not proof of a remote cooperating application. This does not
authorize replacing it with handwritten crypto or an X.509 parser.

Proposed typed optional `payload_mapping: Option<CooperatingPayloadPolicy>` on
both NetworkPolicy variants, serde-default None and skip-None. Its version1 TCP
policy names exact canonical host+port, mapping profile/version, trusted role,
TLS1.3 server name and full SPKI SHA-256 pin, with a peer list bounded at32 and
no wildcard or opportunistic matching. It enriches a destination that canonical
admission already allows; it grants no route. Unknown version, ambiguous duplicate
destination, zero port/pin, invalid name or unsupported role/TLS profile refuse.
None serializes exactly as the previous policy bytes.

Canonical binding already exists: mvm-core policy/network_policy.rs content_sha256
hashes the complete serialized policy; mvm-client/admission/mod.rs records that
identity before signing; mvm-runtime/workload_runner/claim.rs
bind_claim_policy_to_plan checks the materialized policy against signed policy
asset identities. EndpointConfig.network_policy already carries the full DTO.
The field follows existing materialization. Signature/hash-tamper and old-None
canonical-byte tests are required.

Additional implementation paths proposed for signed selection:

- `crates/mvm-contract/src/policy/network_policy.rs`: optional field, validated
  accessor/builder, constructor defaults and serialization fixtures.
- New `crates/mvm-contract/src/policy/payload_mapping.rs` and existing
  `crates/mvm-contract/src/policy/mod.rs`: bounded versioned pure DTO/validation.
- `crates/mvm-core/src/policy/network_policy.rs`: facade exports if needed.
- `crates/mvm-hostd/src/bin/mvm-network-endpoint.rs`: consume the already bound
  policy and install the payload adapter in accept_params.
- New `crates/mvm-hostd/src/supervisor/flowmux/payload_tls.rs`: reuse rustls0.23 already in hostd for
  TLS1.3 on the same admitted TcpStream, explicit ALPN, handshake signature and
  peer identity verification, and pin comparison before mapping activation.
  x509-parser currently exists only as a mvm-core dev dependency; a production
  parser edge requires explicit closure review, not an assumed reusable verifier.

There is no MVM policy/execution u64 epoch field at this seam to copy blindly.
ExecutionPlan has plan_id/plan_version; runtime instance identity is separate.
Bind those actual admitted identities to the per-flow context; do not fabricate
an epoch from an unverified digest. Distinct connections need distinct
authenticated stream namespaces, rather than a static per-peer namespace whose
block index resets0. Exact context materialization and TLS full-duplex ownership
remain contract work before activation. Public NetworkPolicy variant constructors
may require mechanical new-None additions at compiler-identified consumers;
these are not blanket approval to edit them.
