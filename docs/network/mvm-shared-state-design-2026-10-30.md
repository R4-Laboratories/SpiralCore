# Proposed ADR: MVM mailbox gateway and deterministic shared-state recipes

Date: 2026-10-30. Status: **PROPOSED; source inspection only.** No MVM adapter,
cellular-automaton codec, new WAN experiment or energy measurement was implemented
or run for this note. The user requested direct MVM/vsock investigation and a
precise treatment of ternary updates and a shared CA/Kuramoto base. Byte reduction
and exact reconstruction remain primary; elapsed time and both endpoint CPU
remain required measurements.

## Snapshot and scope

- SpiralCore: `fdc7586214260d9d09bf63820614dd62a0bade5b`, including the published
  [SC-OPT-007 negative result](sparse-pruning-results-2026-10-30.md). Its candidate
  was not adopted; active transfer source remains the preceding baseline.
- MVM: `708dd28dbf257d3a677be132353b1cd3e154285e`. This supersedes the older MVM
  snapshot for the integration findings below, without rewriting historical audits.
- Scope: current object-transfer core, mailbox contract/crypto/transport seams,
  typed vsock services, FlowMux separation, and proposed shared-state semantics.
  This is not a whole-MVM security audit, deployment qualification or new review
  of every historical SpiralCore research artifact.

## System as built

SpiralCore is an application-level exact object-transfer engine with a verified
cache, explicit raw/zstd/DAG/sparse/prefix representations and experimental TCP
and HTTPS adapters. It does not currently forward arbitrary IP packets, serve as
a transparent router, or expose MVM's canonical mailbox service. Its standalone
crates have separate Cargo manifests. The production paths are
[`spiralcore-transfer`](../../crates/spiralcore-transfer/Cargo.toml),
[`client`](../../crates/spiralcore-transfer/src/client.rs),
[`selection`](../../crates/spiralcore-transfer/src/selection.rs),
[`endpoint`](../../crates/spiralcore-transfer/src/endpoint.rs), and
[`wan`](../../crates/spiralcore-transfer/src/wan.rs).

MVM's Rust workspace separates the data-only `mvm-contract` from `mvm-core`
crypto/transport logic, the guest `mvm-agentd`, and host/VMM networking. The guest
has no NIC; its host process opens policy-admitted network connections. Existing
FlowMux is the path for general TCP/UDP networking: its host
[`OpenTcp` handler](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-hostd/src/supervisor/flowmux/open_flow.rs#L147)
applies policy before opening the admitted connection. Mailbox messaging is a distinct
application interface, not an alternate packet tunnel. See the pinned
[workspace](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/Cargo.toml),
[README](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/README.md#L16),
[network ADR](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/specs/adrs/042-single-flow-vsock-networking.md),
and [mailbox ADR](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/specs/adrs/057-mailbox-semantics-and-threat-model.md#L353).

The intended data flow is:

```text
authorized sender application / guest
  -> SpiralCore recipe + exact residual
  -> canonical MVM seal + outbound authorization
  -> dedicated mailbox vsock service [not yet present at inspected pin]
  -> host relay -> authenticated WAN transport -> peer host relay
  -> remote dedicated mailbox vsock -> authorized recipient guest
  -> canonical MVM authenticate/open inside the authorized endpoint
  -> SpiralCore reconstruct, verify length/digest, durable object commit
  -> recipient application
```

The codec belongs where plaintext is authorized. The ordinary outside relay
forwards canonical encrypted envelopes unchanged. A trusted-host development
profile may place plaintext differently, but that does not establish host-blind
confidentiality. It must not silently change the requested assurance tier.

## Integration findings and proposed decisions

| Finding | Evidence at pinned MVM revision | Consequence / next action |
| --- | --- | --- |
| HIGH: Dedicated local mailbox service is missing | [`GuestService`](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-net/src/channel.rs#L22) has no Mailbox variant; [ADR 057](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/specs/adrs/057-mailbox-semantics-and-threat-model.md#L353) requires independent service/queues/budgets; [#4629](https://github.com/tinylabscom/mvm/issues/4629) is open | Implement or consume the upstream service contract before claiming a mailbox-connected VM. Do not invent a port or repurpose FlowMux 5253. |
| HIGH: External byte-transport adapter seam is incomplete | [`AuthorizedMailboxFrame`](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-core/src/mailbox/transport.rs#L517) exposes encoded length but private bytes; [`PeerChannel`](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-core/src/mailbox/transport.rs#L586) is send-only; inspected concrete implementation is in-memory | Define an MVM-owned bounded serialization/I/O and receive/admission seam that retains authorization type-state. Do not bypass the type using arbitrary raw writes. [#4633](https://github.com/tinylabscom/mvm/issues/4633) remains open. |
| MEDIUM: Envelope limits do not bound reconstructed objects | [Contract](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-contract/src/protocol/mailbox.rs#L18) permits at most 1 MiB plaintext and 8 MiB encoded envelope, with capability-specific payload limits | Carry bounded SpiralCore frames across multiple messages. Reserve aggregate object, cache, queue and replay-work budgets; do not put a 16 MiB object/HTTP batch into one envelope. |

Verdict: **SOUND-WITH-RISKS for the proposed separation; direct mailbox integration
requires the two HIGH gaps to be resolved before live acceptance.** These are
integration readiness findings, not assertions of upstream exploitable defects.
No database/schema review applies to this scoped change. Architecture,
security/performance and infrastructure specialists independently inspected their
respective source boundaries; no tests were rerun for this note.

Use an explicitly authorized application content type for SpiralCore messages:
encode before [`prepare_mailbox_envelope`](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-core/src/mailbox/transport.rs#L507)
and decode after successful [`open_envelope`](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-core/src/mailbox.rs#L457).
Map only verified tenant/domain/recipient/capability/epoch claims into SpiralCore's
context. Cache existence or a route is not authority. Object commit, mailbox
admission, delivery and application processing remain separate acknowledgments.
Replay/dedup acceptance and retry recovery remain MVM-owned. The current
[`open_envelope` path](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-core/src/mailbox.rs#L506)
records replay state before returning the payload. The adapter must preserve
recoverable accepted-message/error state across missing-base, decoding and crash
failures; a successful open is not durable object publication. Test a crash after
open but before object commit, so rejecting a repeated envelope cannot silently
discard the only recoverable application operation. Retry identity and admission
must follow MVM's contract while SpiralCore object publication remains idempotent.

For a small orchestrator, keep one reusable codec and MVM-owned service contract.
Let the orchestrator supply admitted placement/route hints, not a replacement
envelope, crypto scheme or plaintext cache. Backend endpoint mappings identify
the guest connection context; CID alone does not identify a tenant or VM. Native
VMM devices may each use guest CID 3 and host CID 2; host-side connectors use
per-VM endpoints. Bind logical identity, boot/session and execution/policy epoch,
and test restart, fencing, revocation and stale endpoint reuse. Sources:
[native CID constants](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-vmm/src/vmm/vsock_transport.rs#L22),
[host-to-guest backend adapters](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-vmm/src/vsock_transport.rs#L81),
and [guest-to-host endpoint variants](https://github.com/tinylabscom/mvm/blob/708dd28dbf257d3a677be132353b1cd3e154285e/crates/mvm-vmm/src/host/network_endpoint_spawn.rs#L41).
Host attachment must support the backend's Unix socket or owned-stream mapping,
not assume the host uses AF_VSOCK directly. Endpoint process readiness is distinct
from an authenticated guest session. Detached VM operation must survive the
launching CLI's exit, and snapshot restore must establish a fresh admitted host
binding rather than treat a saved socket path as authority.

## What ternary can save

Interpret the proposed `[[1],[2],[3],4]` stacking as three children and a derived
parent. The current [`dag.rs`](../../crates/spiralcore-transfer/src/dag.rs#L4)
already has three-child parents over **32-byte binary leaves**. It is a ternary
tree, not a trit-valued payload representation. Reusing two cached children can
avoid retransmitting them. Their bytes are already present; a parent hash cannot
decode unknown child bytes. Current encoded node records themselves are 66-byte
leaves and 130-byte parents before surrounding framing, not one-bit changes.

There is a valid one-bit trit update. If the old trit `t` and changed position are
known, a guaranteed change leaves two alternatives:

```text
t_new = (t_old + delta) mod 3, where delta is either +1 or -1.
```

One bit chooses the direction. However, the changed-position set and the
no-change case must be known or encoded. For comparison, a known changed binary
bit needs zero new-value bits: it flips. Three arbitrary new binary bits have
eight states; one bit selects only two. Clock agreement cannot distinguish the
other six without additional shared state or transmitted information. These
claims follow by counting possibilities, not by a throughput experiment.

A fair new encoding compares packed trit positions/directions against packed
binary XOR/changed-bit positions, sparse COPY/LITERAL, cache references and zstd.
Include any binary/trit conversion, seed installation and position signaling.
Do not assume a three-to-one savings factor from tree fanout or numeral base.

## Deterministic shared base and exact residual

The proposed generalization is:

```text
prediction = Generate(rule_version, checkpoint, logical_step, ordered_events)
residual   = original XOR prediction             # same declared byte length
original   = prediction XOR residual
```

The XOR form illustrates exactness; a sparse edit/event representation can replace
it when smaller. A short recipe can regenerate a large object if that object has
the specified structure or is close to a verified prediction. For an independent
uniform random target, XOR with a fixed shared prediction is a bijection and
does not reduce its uncertainty. Merely generating elaborate shared randomness
therefore does not guarantee a small residual. Recipe discovery is also compute
and must be bounded and measured.

Use an allowlisted, versioned declarative rule set with explicit dimensions,
boundary conditions, bit order, integer arithmetic, initial state/checkpoint hash,
logical epoch/step, ordered event identifiers, output length and target digest.
Cap cells, steps, event count, memory and reconstructed bytes before execution.
Never execute received source/cards or an unbounded universal program. A missing
or divergent checkpoint triggers authorized repair/refusal and an explicit raw
fallback where policy allows; no success acknowledgment before exact verification.

The candidate cellular automata have different roles:

| Mechanism | Useful first role | What is not established |
| --- | --- | --- |
| Rule 184 | Cheap deterministic traffic/queue-state simulation and replay | General compression or universal computation from this choice |
| Rule 30 | Reproducible generated test surfaces and a randomness-like negative/control case | A universal short representation for unrelated files |
| Rule 110 | Bounded replay experiments with a computationally universal rule | Cheap recipe discovery, efficient arbitrary computation, or a compression advantage |
| Kuramoto | Optional pacing/coordination experiment, measured under drift/delay/partition | Exact distributed state, event order or physical clock equality by itself |

Primary references: [Fuks and Boccara, deterministic traffic rules](https://arxiv.org/abs/adap-org/9705003),
[Wolfram's Rule 30 discussion](https://writings.stephenwolfram.com/2019/10/announcing-the-rule-30-prizes/),
[Cook's Rule 110 universality proof](https://wpmedia.wolfram.com/sites/13/2018/02/15-1-1.pdf),
and [Choi and Pignotti, delayed Kuramoto synchronization](https://arxiv.org/abs/1910.00980).
Universality describes representable computation; it does not supply the input
information or establish fast reconstruction. No quantum mechanism is proposed.

Exact replicas can reach logical step `k` at different physical times. Require
identical canonical rules/state/events and verify the checkpoint hash; do not
infer equality from close oscillator phases. Multi-writer event ordering needs
an explicit ownership/order/conflict rule. A sorted timestamp is not proof that
both peers have received the same events. Preserve the older
[Kuramoto design](../../projects/spiralcore/warm-cache/KURAMOTO_TIME.md) as research:
its broader distributed ordering claims are not established by the local floating
point [`KuramotoCell`](../../projects/spiralcore/warm-cache/rust/warm-cache-proto/src/lib.rs#L43)
simulation. These mechanisms are not part of the measured current WAN path.

## Implementation and evidence sequence

1. Specify the MVM-owned dedicated service and bounded transport/receive seam
   against #4629/#4633. Keep FlowMux egress/DNS/rate/audit controls intact.
2. Add a thin Rust adapter carrying existing SpiralCore frames as canonical
   mailbox payloads; preserve envelope authorization and object-level quotas.
   First prove ordinary exact cache transfer without introducing a new codec.
3. Qualify contract fixtures, one real guest/host, two guests on one host, then
   two actual hosts and a three-peer recovery case. Test queue isolation,
   cancellation, duplicate/reordered messages, reconnect, eviction, stale epochs,
   recipient mismatch and restart. Distinguish host harness from real VM evidence.
4. Separately register a bounded shared-state codec experiment. Use small
   exhaustive trit vectors and independently reconstructed CA fixtures, then
   representative generated application traces plus unrelated/compressed data.
   Compare current sparse/DAG/prefix, binary XOR, raw and ordinary zstd, including
   a whole-object no-base zstd control. CA-generated data alone establishes only
   its own favorable case, not usefulness for MVM traffic generally.
5. Measure all bytes in both directions, paid setup/checkpoint costs, sync and
   repair traffic, encoding/replay/verification CPU, memory, and elapsed time
   through durable verified commit. Computation contributes to end-to-end elapsed
   time; report both CPU totals separately and do not add them to elapsed time.
   Parallel CPU totals are not a wall-clock duration. Energy requires a separate
   power measurement.

For intuition only, saving 1 MiB buys about 839 ms of serialization at 10 Mbps,
84 ms at 100 Mbps, or 8.4 ms at 1 Gbps. In a simplified serial path, extra compute,
durability and round trips must cost less than saved serialization time for a
latency win. Real pipelining requires measurement. Small semantic residuals still
pay identifiers, permissions, ordering, hashes and envelope overhead; batching may
save more than reducing the final residual from two bits to one.

Alternatives considered: a transparent IP router would bypass the intended
mailbox semantics and is not this integration; a host plaintext codec changes
the assurance boundary; a new generic VM/CA interpreter increases attack surface
and unbounded work; requiring exact physical phase would create unnecessary
failure and availability dependencies. The proposed adapter plus bounded
optional recipe keeps those responsibilities explicit.
