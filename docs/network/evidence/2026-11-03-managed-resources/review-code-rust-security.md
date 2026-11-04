# Managed ledger: scoped code, Rust and security review

Verdict: **PASS for the parent accounting component**, with endpoint/native
integration still required. Reviewed `installed_resources.rs` SHA-256
`36efb7298372ff2f92aaf452b1916dabf66da238e9f3e0abe0985f5c5b81e3e3`,
the supervisor delta `8591e211ac395e5888c35c60049934961b2df8235bd3e8e7be1d6f88d81c86a1`,
library export `477313b5a12dd3a396d7204767e7986aac20a1fea019953c3d75fb0d6903ab8a`,
the API contract, eleven resource tests and new production test wrapper
`3087c43422e28b3066fdd9c300b05a7ef4d967e476d31df127051f95fd550090`.
The reviewer did not implement this managed ledger, but previously contributed
unrelated supervisor lifecycle fixtures and consulted on its retained fault
failures. This is source review, not a new execution receipt.

Admission checks all checked arithmetic, global/tenant capacities, slot/tenant
availability, serial increment and peak accounting before one mutex-protected
mutation. A capacity refusal leaves ledger state unchanged. Clones retain a
single private slot/serial charge through the last `Arc` owner; `ManagedValue`
drops its value before its reservation. Fixed tables bound bookkeeping; stable
tenant tags intentionally are not recycled or authorization. The original
installation authority is created only by the controller and retained through
budgets/reservations. Finish closes admission before checking zero transport,
managed resources, positively reaped children and original holder exclusion.

Lock order is sound in this delta: fresh owner validation completes before
taking the accounting lock, and code under that lock accesses only atomics,
bounded in-memory state and `Arc` bookkeeping. Fencing does not acquire the
owner/worker mutex. Checked Drop validates slot/serial/tenant and both
subtractions before refund; faults retain a bounded quarantined authority,
close admission and block retirement. Epoch/storage failure closes siblings;
the underlying owner also retains its own storage fence. No unsafe code,
unbounded table, public pool constructor, reset or manufactured boot/network
permission is introduced.

Existing receipts separately report Linux33/Mac32 standalone entries, Linux143
production entries and strict production Clippy0. The twelve-target production
log includes two zero targets and predates the new `managed_resources.rs`
wrapper; that wrapper is **NOT_RUN** at review time. Its source correctly runs
the same eleven tests against the public native-host export without duplicating
the implementation. Selective coverage221/233 (94.85%) is this module's
production mapped-line scope, not the full crate or prior native performance
source. Managed claims remain trusted declarations, not automatic allocation
interception or whole-process RSS enforcement; the concrete installed bridge
must retain these actual leases for every endpoint/cache/workspace/child owner.

Post-review execution delta: the root executed the unchanged public wrapper
against its owned frozen Apple Silicon macOS production overlay. The retained
`validation/resources-public-macos-green.txt` / `.exit` report eleven passed,
zero failed, release compilation8.24s and test execution0.63s, exit0. This adds
actual public-crate wrapper evidence on that host; Linux wrapper execution
remains **NOT_RUN** because Wick is unreachable. The prior33Linux/32Mac
standalone and143Linux product scopes remain unchanged. No new execution was
performed by this reviewer, and no wrapper or product assertion was changed.
