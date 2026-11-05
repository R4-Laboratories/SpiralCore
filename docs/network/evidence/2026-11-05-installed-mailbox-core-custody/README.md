# Original Core custody for the installed MVM mailbox

This checkpoint adds a one-shot handoff from the original installed parent
permit. The Core launch anchor verifies the exact admitted plan and launch
shape. It fences the request before invoking parent callbacks, accepts only
the matching current inert owner, and moves the owner with the original
anchor at most once. A concurrent native claim shares the fence. Ordinary
launches continue through their existing one-shot native claim.

The protected Linux integration test covers missing or wrong parent context,
exhausted fixture capacity, callback reentry, mid-preparation revocation,
duplicate take and native refusal. The fixture charge counts owner drops.
Production quota accounting and the listener and worker are the next step.
The Hostd signed-selection gate still closes selected native dispatch.

- [Replayable Core and test patch](installed-mailbox-custody.patch)
- [Expected test-first compiler diagnostics](red-expected-diagnostics.json)
- [Protected host-lifetime integration results](full-host-lifetime-results.txt)
- [Prior signed launch gate and custody contract](../2026-11-05-installed-mailbox-launch-admission/README.md)

Patch replay reproduced all three source files byte for byte. The original
Core `host_lifetime.rs` SHA-256 was
`429f61d3507e6a402bc6db7d0679d91eb8114a89ad606f334516e10807ac9ef1`.
Independent code, Rust and security review passed for inert custody.
The production provider must prepare only from Hostd's selected opaque plan.
An inert owner's destructor must remain safe independently of the anchor's
other fields if the owner is never taken.

Next is the original funded worker retained in the matching Runtime slot
before bind or spawn. The selected route requires dedicated queue and rate
accounting, verified wake and join, and signed guest support before native
activation. This checkpoint started no guest and measured no transfer.

## Verification

The locked offline Linux release build for the integration fixture failed
first with the expected missing Core API diagnostics. The protected
successor passed all nine host-lifetime integration tests and 11 selected
Core host-lifetime unit tests. Each phase matched 5,410 pinned source
entries before and after. See [phase and executable identities](result.json)
and the [Core unit test output](core-unit-results.txt).
