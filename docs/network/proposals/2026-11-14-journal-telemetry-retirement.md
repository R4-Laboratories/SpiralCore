# Proposed decision: original-owner journal stop with protected telemetry retention

Status: proposed integration decision, not implemented or qualified. Scope: MVM contributor `468661f47e01d0f59c6f1da4e35a968ceccb3ad2`, Linux/Firecracker first; same canonical lifecycle contracts on other qualified backends. Governing issues: SpiralCore #111/#43/#42 and MVM #5466. ADR-057 retains the sole mailbox authority. No wire, capability, receipt or codec change is proposed.

## Observed failure and exact boundary

`clean3-home/paired-transfer-before-stop.json` records a successful transfer substage: 257 exact body bytes, B application ACK, `AcceptedLocalDurable`, 5,792 forward and 1,192 return sealed bytes. Its 1,112 bytes hash to `6febc0b146ffa7edb8acc4648e756b5ffa76d00e67ea9330f07f6cba92c168b2`. The overall attempt failed retirement and exceeded the enclosing deadline. The final paired result was not published. Neither its misleading shell EXIT marker nor later process absence is a successful lifecycle witness.

`mvm-client/src/launch/persistent/journal.rs::stop_persistent_journal` invokes `JournalOwner::stop_and_collect` with an SDK finish callback. In `mvm-hostd/src/mailbox_process/original_journal/stop.rs`, that method orders backend stop, `state.vm_stopped = true`, original kernel-exit observation and Child collection, same-directory verification, epoch-secret disposal, then the callback. The callback reaps runtime state through `mvm-runtime/src/vm/reconcile.rs::reap_unowned_state_dir`.

`remove_protected_runtime_dirs` calls `RuntimeStateProtection::check_removal_allowed`. That gate rejects any telemetry record/key/status entry, even without an installed-retention marker. Both VM directories contain those entries; observed directories and serialization lock satisfy ownership/mode/link/size checks. The source call path therefore locates the refusal at SDK finalization, after the earlier stop/collection stages, rather than an observed Firecracker boot failure. This is a source-supported inference: original successful helper statuses were not exported, so no complete retirement claim follows.

## Existing mechanisms and missing ownership

The ordinary path has no complete telemetry archival transaction to call. `mvm-vmm/src/host/network_endpoint_spawn/collector.rs::spawn_telemetry_collector` drops the actual Child after PID publication; `reap_telemetry_collector` signals a recorded PID and removes its marker. That is not original Child collection. It must not be promoted into a positive original-owner retirement receipt.

The installed FlowMux path already supplies the reusable components:

- `mvm-hostd::telemetry_collector::OwnedEmbeddedCollector::close_and_join(deadline)` retains and joins the three original threads and cancellation stream, preserving custody/permit on timeout or failure.
- `mvm-hostd/src/installed_endpoint/controller/network.rs` orders worker/group/collector close-and-join and only then sends Drained.
- `mvm-vmm::host::network_endpoint_spawn::OwnedEndpoint` retains the actual Child, armed observer, Drained state and collected status. `EndpointRetirement` retains the original partition through parent storage disposition; it explicitly is not a disk-cleanup receipt.
- Installed telemetry records pin original file/directory identities and parent authority in `network_endpoint_spawn/installed/records.rs`. Existing `EndpointPartitionGuard` callbacks reserve and bind retained storage; querying requirements grants no quota.
- SpiralCore's `installed_retained.rs::RetainedTelemetryInventory` and `installed_owner/retained_telemetry.rs` retain bounded encrypted records, key identities and catalog debt. `seal_after_endpoint` and `update_retained_telemetry` do not delete files or refund retained credit. The audit archive operation there is not a general telemetry archive API.

These mechanisms exist, but ordinary journal launch does not currently hold those original telemetry owners or the installed retained-storage disposition. A two-line exception in generic removal would bypass that missing contract.

## Decision

Integrate persistent journal startup and shutdown with the existing original endpoint/collector custody and parent retention guard. Preserve the current admission owner; do not create a PID-derived controller, new signing authority or guest network path.

1. At launch, retain the original telemetry-capable endpoint/collector Child and cancellation/control channel alongside the journal's original VM/helper custody. Before activation, reserve the existing collector memory/thread and retained record/sidecar allowances from the actual shared host/tenant owner. Bind the original state, parent, records and wrapped-key identities. If this custody cannot be furnished, refuse the stronger lifecycle mode before effects. Do not manufacture an installed permit from a filename or projected quota.
2. During stop, retain the existing original VM stop and mailbox Child collection sequence. Close the original telemetry writer through the existing owned stop/drain path, join its actual workers and collect the actual endpoint Child under one fixed deadline. Store each successful stage in the retained owner so retry does not signal a completed owner again. Timeout, panic, stale epoch or uncertain wait preserves handles and charges.
3. Finalization must distinguish process stop from storage disposal. Extend the typed journal outcome to report a protected retained-state disposition bound to the original directories and retention owner. A successful stop with retained ciphertext may be reported as such; it must never claim state removal, full resource release, deleted definition, fresh-generation admission or fully clean installation. Retained record/key/storage charges remain booked. Generic `RuntimeStateProtection` refusal stays unchanged.
4. Reuse the existing exact-directory guard and retention-catalog bindings for durable publication of this disposition. If the supplied parent cannot accept and retain that exact generation, finalization fails and the original journal owner remains in custody. Do not move or unlink telemetry files merely to make generic reaping pass.
5. Actual archival/expiry/disposal is a separate original-owner transaction. Until a qualified transaction exists, keep the ciphertext and wrapped key together under their original private directory and retained quota. Explicitly block name/epoch reuse that could overwrite the retained generation. Recovery reopens the durable original retention record, rechecks authorization and identities, and cannot infer clean state from PID absence.

This changes lifecycle ownership and the public typed stop outcome, so record and review it as an ADR/interface change before implementation. It does not require a new mailbox protocol, cryptography module or cleanup exemption. The full MVM objective still requires original collector collection and resource accounting; a typed retained-state result alone is not completion of those requirements.

## Required focused witnesses

- Actual original VM/helper/collector owners complete stop, Drained, thread joins and Child collection; retained ciphertext/key and charges remain readable/booked under the original owner.
- Existing ordinary removal rejects every protected entry before and after this change, including malformed marker, key-only, records-only and status-only cases.
- Collector blocked write/read, cancellation, deadline, worker panic and wait failure retain original handles; retry collects the same handles without releasing credit early or overwriting failure evidence.
- Same VM name with another caller, original directory replaced/renamed, parent replaced, altered lock, substituted records/key inode and changed epoch all refuse. No reconstructed path/PID may authorize retirement.
- Finish callback failure after VM/helper collection retries only remaining finalization; exactly one original status and one retention transfer are delivered. Repeated stop is idempotent without exposing a second cleanup capability.
- Crash before/after durable retention publication preserves debt and encrypted-key association. Fresh startup cannot reuse a retained generation as disposable runtime.
- Test no-telemetry legacy behavior separately; retain unsupported-platform refusals. Native Linux clean and lost-receipt runs must verify exact payload, byte meters, CPU, time and both positive owner retirement and retained storage. Subsequent macOS and full workspace/gated/BDD checks remain separate acceptance.
