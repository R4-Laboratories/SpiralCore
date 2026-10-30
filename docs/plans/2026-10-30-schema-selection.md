# SC-OPT-014 implementation plan

The user authorized this step. Execute the approved
[design](../specs/2026-10-30-schema-selection-design.md) after parent commits these
documents; routine choices within scope need no further approval.

1. **Production codec/selection — architect implementation owner.** Add focused
   RED tests in `tests/schema_selection.rs`, then Mode 9, `zstd_whole`, prepared
   selection/receipt and consuming client APIs. Integrate strict buffered
   framing and equally charged Whole/Schema prepared decoder reuse. Preserve
   prior wire formats, selector behavior and endpoint commit/auth semantics.
   Test exact costs, cold seed decisions, malformed frames, resource admission,
   retries/errors and exact reopened bytes. Keep receipts concise.
2. **Harness — measurement owner.** Reuse existing persistent WAN fixtures and
   paired execution. Three current-source arms: paid schema reference, equally
   tuned Whole/Raw and selection. Time selector prepare once, seed only when
   required, then consume preencoded payload. Record the three candidate costs,
   mode, actual seed, per-mode receiver-cache progression and disjoint timing
   scopes. Context construction is charged for every session. Preserve negative
   outcomes and reject incomplete/mismatched source or readback evidence.
3. **Workflow and qualification — parent.** Adapt the bounded existing remote
   workflow and source gates, using the established persistent supervisor for
   receivers/tunnels. Run focused then required full checks once, with exact
   current-object coverage on changed production. One bounded independent
   code/security review; no broad audit or new evidence framework. Freeze the
   qualified source, executables and plans before transfers.
4. **WAN and delivery — parent/measurement.** Run the registered persistent
   comparison on real remote machines with the existing controlled HTTPS relay.
   Include actual setup in all session totals, verify every actual seed/target
   after receiver stop, and compare body bytes, sender and receiver-handler CPU,
   and durable elapsed against the equally tuned control. Publish observed
   results and limitations through the authorized repository process.

Reuse owned build targets with incremental builds disabled and at least 4 GiB
free. Coordinate build lanes; no builds/profiling during timed transfers. Preserve
original user data, other worktrees and historical evidence. MVM remains later.
