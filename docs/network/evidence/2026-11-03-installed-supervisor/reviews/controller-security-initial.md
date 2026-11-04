# Installed supervisor controller: initial security source review

**PASS for the controller/budget component at112eeeab.** No blocking finding
in the reviewed production delta. No worker admission API exists in this slice;
this approval cannot be carried forward as worker/native activation approval.
Source-only review; no builds, tests, remote operations or product edits.

Exact SHA-256 pins:

- `src/installed_supervisor.rs`: `a58b833f02ddf7e51bec6b8c746efe5aa62a79520ef9799cbcbb38bc1daf6ae0`
- `src/host_budget.rs`: `0a717f421138f864952366550188e0efdbfb2530f4adb6030d035f1e7f034af1`
- `src/installed_owner.rs`: `cb22012ceeba38688c966b9d8b4d3733eeb2dd669984bdf039780aa967934b02`
- `src/orchestrator.rs`: `9053833bfb00af0ad8bce024fbf1215093d396a1d2a0c24fd18235f58c9d39e5`
- `src/lib.rs`: `b093a042bd84ba07f12a4acc71cfed26b31207c37e3de189d1492d0f7e3a2b9b`
- Supervisor API: `dde742094728caa5ea5bd71397212c1c3bbe95a6413aa033e28583adf77189a4`

The non-Clone controller consumes one owner, freshly checks its fixed profile
before begin, then privately creates one installed Shared and authority Arc.
Authority has no budget back-reference and exposes no owner/Arc/lock getter.
Scopes, links and frames already retain Shared, so they transitively retain
owner EX even after controller/budget handles drop. Explicit finish does not
extract the owner; idle aliases retain exclusion until final release. Drop only
closes admission and never publishes Clean.

Charges and scope creation test the closed flag under the same ledger mutex.
Close clears that flag then takes/releases the ledger mutex, ordering completed
in-flight charges before the zero-usage check. New admissions cannot resume
after the barrier. Existing releases remain valid. No owner/filesystem I/O is
performed under the accounting mutex and no owner-to-ledger nested lock cycle
is introduced. try_finish closes first, refuses accounting faults/nonzero usage,
then invokes original owner.finish on the retained epoch. Independent SH keeps
it Busy; retry remains closed and preserves that epoch. Other owner failures
fence the controller; no reset or implicit reopen occurs.

The public installation-bound query is descriptive, not a grant. The legacy
session_with_budget guard rejects binding before Stage/filesystem/activation
work. Ordinary public new still makes independent orchestrator ledgers and
snapshots remain orchestrator-scoped. Native-host cfg surrounds authority imports,
fields and constructors; guest/Windows budget paths do not name missing native
types. The owner's public profile query is fresh read-only configuration identity,
not credential, owner extraction or launch authority.

Read retained boundary-green.txt/exit0: six existing budget/owner unit checks
plus three controller lifecycle checks PASS on local macOS. They cover sole
outliving frame exclusion/Active retention, closed aliases and last-idle-clone
owner retention, profile mismatch before epoch0 changes, and independent-holder
Busy followed by same-epoch explicit finish. These are component checks, not
native guests or a broader Linux/Windows qualification. Absence RED remains
retained. No check was rerun by this reviewer.

Mandatory next-slice prerequisites remain explicit: add the private bounded
worker/owned-reaped-Child ledger before any activation API and gate finish on
it. Current budget poison/underflow sets Shared.fenced without clearing the
authority's open bit; all current budget admission checks both, which is safe
here, but a future worker-grant path must observe/propagate that budget fence
too. Checking only authority.admission_open would be insufficient. Live private
grant channel, exact role/slot/epoch, Stage One/verifier closure, actual worker/
VMM leases and shutdown failure handling need their own implementation/tests.

This PASS establishes no installed launch default, actual MVM custody,
scope=host, full B6a/B6b, increased retention, WAN or whole-goal completion.
