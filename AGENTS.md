# SpiralCore engineering workflow

## Roadmap and delivery authority

The project roadmap and status source of truth is the
[SpiralCore GitHub issue tracker](https://github.com/R4-Laboratories/SpiralCore/issues).
The MVM delivery program is tracked in
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42) and its linked tasks.

Before implementation, fetch `origin/main`, inspect current repository state and
the relevant live issue, and identify the work being delivered. Preserve local
runtime files, saved sessions, other contributors' work and historical evidence.
Reuse a suitable isolated worktree and a `codex/` branch for Codex changes.

Update the issue at meaningful implementation, test and delivery checkpoints.
Link commits/PRs, specifications and recorded measurements. Keep incomplete
acceptance criteria and blockers explicit. Close an issue only when its full
acceptance criteria have evidence; a partial implementation, source review or
green CI badge is not completion. Keep the parent roadmap consistent with its
children. User-authorized reviewed/tested changes may merge through a PR to
`origin/main`; do not force-rewrite user history.

## Implementation and evidence

Use Rust for active product and measurement code. Preserve the historical Python
research references. Keep one SpiralCore protocol/codec core with explicitly
versioned endpoint mappings and canonical MVM protocol ownership. Do not copy a
codec or invent a competing MVM envelope/authority model for an adapter.

Zero fidelity loss is mandatory. Record exact lengths and byte readbacks, actual
traffic in both directions, setup and sender/receiver CPU, verified end-to-end
time and lifecycle costs. Label planned bytes separately from transmitted
bytes. Keep cold/warm, native backend, shaped network and real-WAN results
distinct. Retain losses and failed gates; do not infer energy or send-only time
from CPU. Keep evidence compact and remove only owned disposable payload stores
after verification, respecting available storage.

Use focused tests before implementation and independent code/security reviews
for trust-boundary changes. Validate the exact release source before merge.

## MVM integration

Follow current MVM canonical contracts and signed-plan/capability/DNS/rate/audit
enforcement. Mailboxes, FlowMux network forwarding and owner access retain their
separate service semantics, queues and authority. A guest NIC, alternate egress
route, opaque-traffic inspection or hidden fallback is not implicitly admitted.

The complete MVM contribution requires an explicit supported role/version/
architecture matrix and executed end-to-end WAN tests for Linux, macOS and
Windows roles. Distinguish native MVM backends from remote clients/gateways;
cross-compilation or WSL execution is not native Windows evidence. Record every
unavailable platform/backend gate rather than calling the delivery complete.

Scale is a release gate tracked in
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49). Two guests establish
a functional boundary, not fleet capacity. Qualify aggregate resource budgets,
long-lived bounded state, concurrency, tenant fairness, backpressure, recovery
and tail latency; record the supported capacity envelope and saturation. Preserve
one protocol when adding negotiated pipelining or batching. Simulated endpoints
do not establish the same number of booted MVMs or authorize larger host budgets.

Implement and qualify the MVM work in SpiralCore, then prepare the authorized
outside-contributor PR to `tinylabscom/mvm`. Formal certification, FIPS/CUI and
host-blind security claims require their own concrete authority and evidence.
