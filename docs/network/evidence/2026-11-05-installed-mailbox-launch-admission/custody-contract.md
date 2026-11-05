# Candidate custody contract for the installed MVM mailbox

Scope is cold installed Linux Firecracker under the original signed admission.
Canonical mailbox format, operation capabilities, receipts, and per-message
authority remain with MVM. Coordination is tracked in tinylabscom/mvm#5057
and delivery in SpiralCore#43. No port allocation or production claim.

## Original preparation and ownership

1. Hostd `plan_admission::start_admitted` admits the fixed selected profile
   through the original `host_lifetime_parent::bind_current` path. The selected
   path requires the original `HostLifetimeAnchor` and live provider. After
   ordinary launch gates, the original `bind_config` result must supply a
   prepared, one-shot mailbox lease before backend dispatch. Missing, foreign
   or repeated preparation refuses. Existing providers default to unsupported.
2. `mvm_core::host_lifetime` holds the prepared lease on the original anchor.
   `InstalledParentProvider` derives it from the same admitted plan, resident
   and reservation. Preparation validates session binding and holds actual
   charges before effects. It carries an in-process factory across the
   dependency boundary. It takes no UDS, port, key or resource claim from
   caller config, and forms no strong ownership cycle.
3. Runtime moves the original lease into `InstalledSlot` before attempting
   socket bind or worker start. It retains the original owner through every
   partial failure. The bound socket is in the verified private per-VM state
   directory. Only the Ready owner supplies the path and creates one cold
   `Mailbox`/`GuestDials` spec entry via `mvm_vmm::host::spec_map`.
   Ordinary, standby, restore and raw callers get no installed mailbox entry.
   Native start waits for readiness. The Firecracker symlink is just a carrier.
4. Reuse canonical `LocalMailboxService` and a resident aggregate
   `BrokerContext`/`HostBudget` for host, tenant, sender and mailbox admission.
   Use a dedicated worker and independent queue/credits. Charge workspace,
   cache and worker resources once through the same managed aggregate budget.
   The first Standard profile uses one actual-I/O token bucket per attachment
   across both directions, 4 MiB/s with 8 MiB burst. Budget waits are cancellable.
   FlowMux retains its own separately admitted policy, rate and resources.
5. `InstalledSlot::stop` first closes mailbox admission and wakes blocked
   work, then obtains the original Firecracker exit proof, then positively
   joins the original worker and verifies exact socket cleanup. Parent
   retirement requires the matching receipt and rejects Pending, Live or
   Uncertain mailbox state. Deadline, panic, stale ownership and cleanup
   failure retain custody and charge. No reset, silent fallback or PID-based
   inference can substitute for positive retirement.

## Checkpoints

- Host-side test-first. Test the exact prepared lease and bounded UDS service
  with the native activation gate closed. Cover default off, missing or
  foreign permit, duplicate take, over-budget refusal before effects, ready
  path, malformed/oversized/unauthorized frames, actual bidirectional limiter,
  blocked-read wakeup, disconnect, stop/join failure and idempotent retirement.
- Activated signed guest. The existing development guest client is useful
  evidence only. Build/pin a signed guest with an actual Mailbox vsock client
  and bounded guest bootstrap. Under the installed original owners verify
  exact call/cast bytes with no guest NIC or mvmd, typed replies, cancellation,
  early/late duplicate controls, isolation while mailbox is saturated, cleanup,
  and unchanged FlowMux authority. Then repeat with two guests, including
  partial-boot and lost-reply recovery, before WAN and platform acceptance.
