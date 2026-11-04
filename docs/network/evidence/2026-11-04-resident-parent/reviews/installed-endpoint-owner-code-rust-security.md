# Installed endpoint owner: independent component review

2026-11-04. **HOLD for one resident cleanup unwind entry; other reviewed
ownership/authority invariants are sound within the stated component scope.**
Reviewer authored the endpoint contract, not this product or private tests. No
Cargo, tests, native launch or broad audit was run for this review.

| Input | Exact SHA-256 |
|---|---|
| installed_parent.rs | `61c88c02d4b05e6a76aefab9d7de19c355fff317ee06bee0e853c805896d0be4` |
| installed_parent/endpoint.rs | `bcb4c6c0818457f41ebb7e98bb2ea79b692632d1dd6349cb5e65848b9b1eaf59` |
| installed_parent/endpoint_tests.rs | `9dba5478517d76f77da2e30369c229d64f3cec5c22058ef8ba55d759a8d3fe25` |
| installed_parent/resident.rs | `a4baa1674c030235766809d35bea6da66b2022806ea6f21c660d8001793e81c7` |
| installed_resources.rs | `ee7ee5faccb4181f18c33c534dfce7aa64de10ea379bf73741142f2ac1fbfa62` |

## Blocking cleanup-entry finding

Resident `request_stop` invokes `Live::schedule_stop` directly. For
RetiringOutcome/RetiringPending (and Unused), schedule_stop calls `complete`
synchronously. The newly connected complete path can execute actual endpoint
store cleanup. This entry is outside `drive_guarded`, unlike requested scheduling
and polling from drive_once. If that cleanup unwinds and an embedding catches it,
the previously established caught-unwind admission fence is bypassed. The slot
may retain custody, but service admission remains open and poisoned cleanup may
be retried through the same facade. This conflicts with the resident's stated
uniform panic/quarantine disposition.

Minimal fix: route request_stop's validated latch/scheduling through the SAME
guarded entry, or make the method latch-only and move all cleanup into guarded
drive (the latter changes current synchronous completion behavior). On caught
unwind retain the whole original owner and fence all new admission/reentry,
without creating a receipt or refund. A small private panic-at-cleanup-entry
regression can qualify this policy; no positive native receipt constructor is
needed. Earlier ordinary operation-poll panic coverage does not cover this
public synchronous cleanup entry.

## Authority, topology and failure custody

QualifiedEndpointProfile and endpoint configuration have no production
constructor. The synthetic profile and direct admitted fields exist only in
private tests. Missing profile is rejected before Preparing, endpoint reserve or
inventory effects. Thus this actual implementation remains safely inactive
pending accepted managed/bootstrap/codec/carrier/worker terms; fixture numbers
are not a production envelope or native-support switch.

The grant is constructed with new_cyclic for its private Weak self identity.
It retains exactly one strong endpoint owner; that owner has a weak grant edge,
the SAME actual VM reservation clone, endpoint reservation and original store.
No strong grant cycle is created. Sealed EndpointRetirement is stored separately
in ParentLaunchOwner; keeping it inside its own guard would form a self-cycle,
which this code avoids. Resource Drop deliberately retains VM, endpoint and
store owners on uncertain disposition; it does not merely leak a quota DTO.

Preparing is serialized under GrantLifecycle before resource effects. Attempt
Drop marks Uncertain on partial errors/unwind; its Resources retain acquired
claims/store. This is conservative, including reserve failures whose effects
might be known absent; it does not forge clean completion. Actual Core partition
construction occurs after releasing lifecycle, so its callback does not
self-deadlock. Core-constructor or later handoff refusal retains the issued
owner; a second issuance refuses rather than adopting absent history.

The immutable projection comes from actual original binding/admission/session,
actual reservation serial and prepared store identity. Session derivation uses
the actual admitted nonce/plan identity plus original installation/generation/
serial; it is an identifier, not an independent authorization epoch. Guard checks
compare full projection, original weak grant, exact retained owner identity and
actual current reservations. Grant and resources locks are not nested across
disk work; the cleanup resources mutex is released before lifecycle completion.

Outer consumers first validate original native parent receipt and retain the
moved endpoint receipt in ParentLaunchOwner before attempting cleanup. Thus
failed exact endpoint-guard/projection validation or filesystem cleanup retains
the actual linear receipt; retry does not consume it again. Cleanup borrows the
same guard Arc and full projection, uses retained identity after normal close,
and calls the existing original-store retirement wrapper. Only positive disk
completion records CleanupComplete. Parent finish remains separate and requires
original native proof plus that private completed state. Missing endpoint proof
never substitutes for Issued/Uncertain. Incomplete ParentLaunchOwner Drop retains
both original grant and moved receipt.

## Foreign supervisor: diagnosed regression and fix

The original test-only configure_endpoints accepted a supervisor parameter
without proving it owned the original pool. Equal configuration/limits are not
identity: this could pair the parent's memory claim with a foreign inventory,
authority and disk ledger. Root reports genuine build-zero/test-101 at the new
foreign-supervisor assertion. The minimal fix is source-correct:
ManagedResourceBudget::same_pool uses Arc::ptr_eq on the actual ledger Shared,
and configure checks it BEFORE Arc mutation or inventory reservation. It is
cfg(test), matching the only configuration constructor's availability; an
eventual production constructor must preserve this identity gate. The negative
test verifies the foreign ledger unchanged and finishes both actual controllers.

## Test fit and remaining acceptance

The three endpoint tests honestly qualify private policy plus actual store/
ledger behavior: missing-profile no claim; foreign inventory refusal; one-shot
guard identity, close refusal, missing proof retention, no strong grant cycle
and uncertain resource retention. Their fixture admission is explicitly not
canonical opaque admission, and a test-only disarm of an uninvoked launch owner
is identified. They never manufacture a native endpoint receipt. Root reports
final protected three endpoint + three resident + four existing policy entries
PASS, with seven source inputs and executable identity stable. That execution
is distinct from this source-only review and does not exercise the new finding.

Actual partial-store/fault cleanup, positive sealed endpoint retirement, issued
resident consumer execution, numeric profile, native activation, full resource
model, capacity and WAN remain separate gates. Resolve the bounded unwind entry
before carrying component source approval forward; do not infer activation from
the expected successor review.

## Guarded request-stop successor

**Scoped Code/Rust/Security source PASS** for the narrow correction at resident.rs
`cac31c3d3cc5a57fbf531d09c671ae2294d7634cd16f9af34af783a818a254ba`
and private resident tests
`c9958429fa356a1d901ee5f3acb3497bcc590082d10238e4ad307ec6800a9055`.
Other endpoint inputs remain at the reviewed identities above. request_stop
now validates Live availability, then runs original ID validation, stop latch
and scheduling inside the same drive_guarded. Ordinary errors, stale-ID behavior
and scheduling semantics are unchanged. A cleanup unwind now closes admission,
retains the entire remaining owner and resumes the original payload exactly as
the already-reviewed operation-poll path does. The blocking source gap is closed.

The new regression executes a real original pure-preflight failure to obtain
its sealed unused certificate, retains that certificate for first-slot cleanup,
and inserts a controlled diagnostic Drop panic. The other original future stays
unpolled. After caught request_stop unwind it requires new admission, drive and
finish refusal, with the second actual grant/backend surviving facade Drop.
The first proven-unused grant legitimately refunds, so it correctly requires
one remaining guest/1 GiB, not an unchanged two-guest count. No receipt constructor
or synthetic native exit proof is introduced. Root reports predecessor build
zero/test101 at the exact caught-cleanup-admission assertion; successor GREEN
was still running at review. Preserve its outcome separately; this source PASS
does not invent execution, native activation or any missing model qualification.

Root subsequently reports protected terminal execution zero: three endpoint,
four resident (including the new request_stop-unwind case), and four existing
policy entries, **11 PASS**. Strict Clippy `--lib --tests -D clippy::all` exits
zero. Seven selected source hashes and the executed binary identity match
before/after. These reported successor outcomes close the pending component
execution qualification in this review; no reviewer rerun or positive native/
issued-receipt/model activation is implied.
