# Scale boundary of the canonical mailbox pilot

Source reviewed: `23e57d4829c94b0eaa7990b8e65b946b5f29c26a`.
Three parallel read-only reviews covered architecture/deployment, performance
and security. No new load test, measured capacity or source fix is claimed.
Database review was inapplicable: this scope has no database/schema service.

**Verdict: SOUND-WITH-RISKS for the bounded pilot.** The following are release
constraints for sustained/fleet use, not new observed fidelity failures. They
remain tracked in [#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).

## As-built path and ownership

`spiralcore-mvm-live` selects the guest/HVF/Firecracker witness entry points.
`src/campaign/canonical.rs` prepares one canonical Call through
`spiralcore-mvm-adapter::service_sender`, registers it, transmits it and waits for
the separately encrypted admission receipt and application Reply. Reply
admission sends the fourth envelope. The receiver service authenticates before
admission, then `service_object.rs` delegates to the shared transfer crate's
`ObjectEndpoint`. The relay forwards opaque framed envelopes; it has no guest
NIC and currently follows that serial four-envelope schedule.

The adapter manifest pins canonical `mvm-core`/`mvm-contract` to public fork
`ae12e9d`. It does not copy the codec. `HostBudget` clones share aggregate
accounting through a mutex; binding policy and mailbox owner guards preserve
authorization across admitted operations. Object publication and mailbox
admission remain distinct. Native witness provisioning remains ephemeral and
experimental, rather than production broker ownership.

## Source-supported constraints

| Severity for scale release | Evidence | Consequence and remediation |
|---|---|---|
| HIGH | `crates/spiralcore-mvm-live/src/campaign/canonical.rs:53–55,76–104` constructs a new budget and clock origin per Session; `crates/spiralcore-mvm-adapter/src/service.rs:1523–1535` enforces nondecreasing monotonic time | Library clones share a budget within that session. Independent witness sessions do not yet form one aggregate host budget. A production broker must inject **both** its shared budget and clock origin; sharing only the budget can cause clock refusals. Separate guest processes also need aggregate host supervision. |
| HIGH | `crates/spiralcore-mvm-adapter/src/service_sender.rs:100–114` creates a fresh scope at sequence one; `service.rs:1655–1665,1727` retains its identity charge; `service/ordering.rs:48–54,102–106,198–206` retains cursors and refuses exhaustion | Fresh scopes accumulate until safe refusal even after transient records expire. The pilot cap bounds this; raising it is not sustained-service support. Use stable owned scopes with increasing sequence numbers for persistent traffic, then specify authenticated retirement/epoch behavior before reclaiming replay checkpoints. |
| HIGH for WAN qualification | `crates/spiralcore-mvm-live/src/campaign/canonical.rs:109–205` waits for one complete exchange; `src/orchestrator.rs:316–375` assumes serial forwarding | Sender concurrency alone cannot safely pipeline the relay. Keep pipeline one as the control; standardize a bounded correlated dispatcher/window under the same protocol before enabling interleaving. Four envelopes do not imply four sequential RTTs. |
| MEDIUM pending measurement | `crates/spiralcore-mvm-adapter/src/service.rs:741–770,2307–2346` scans retained state, including a shared-budget scan under its mutex; `:1416–1435` holds authority guards through the callback and `service_object.rs:100–123` invokes reconstruction there | History and large object work may increase unrelated/control tail latency. These are distinct contention domains. Profile scan and wait/hold costs before changing locks; preserve epoch/revocation and publication authority. |
| MEDIUM pending measurement | `crates/spiralcore-mvm-adapter/src/service.rs:1230–1270,1538–1549` verifies canonical crypto/commitments before quota reservation at `:1647` onward | Reserved queue slots alone do not reserve CPU against hostile traffic. Measure authenticated and malformed floods alongside unrelated control traffic. Bounded ingress/verification work must not weaken canonical verification. |

## Immediate bounded experiment

Start with **2, 8 and 32 logical endpoints**, not more booted MVMs, sharing one
host budget and one clock origin. Keep pipeline one and current limits. Run
4 KiB Calls and generic Casts across repeated retention and 60-second rate
windows, comparing fresh scopes with stable increasing scopes. Continue to the
registered duration/resource stop or first refusal; do not reset the service to
hide retained state. Add a hot sender and slow receiver, then one 14.85 MB
reconstruction while unrelated tenants exchange small/control messages.

Record exact payload recovery, accepted/refused counts and reasons, pending and
retained identities, scopes versus entries, aggregate high-water marks, actual
both-direction bytes, CPU, RSS, FDs and p50/p95/p99 admission/Reply/completion
latency. Sample maintenance scan time and mutex wait/hold time. Check cancellation,
expiry, epoch/revocation and independent control progress under saturation.

Separately measure pipeline one at properly shaped 0/20/80 ms RTT with fixed
bandwidth and identical authentication/durability. Distinguish codec/crypto work
from socket wait. These results decide whether metadata lifetime, contention or
RTT exposure is the first implementation target. No numerical throughput,
concurrency capacity or fleet size is promised before that experiment.

Decision: preserve one protocol, canonical authority and all acknowledgment
stages. Broker-owned accounting and safe long-lived ordering precede fleet
claims; measured contention precedes lock sharding, and a versioned bounded
window precedes pipelined native transport changes.
