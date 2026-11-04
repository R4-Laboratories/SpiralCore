# Installed resident Code/Rust/Security review

2026-11-04. **HOLD: caught-unwind fencing gap in this exact source.**
The reviewer authored the resident contract and external component test, not
the product implementation or its private unit tests. The external test is
not claimed as independently reviewed by its author. No builds or tests were
run for this source review.

| Exact reviewed input | SHA-256 |
|---|---|
| src/installed_parent/resident.rs | `652aa688d5ddf9a772ac648594308cfe3d97c27a1f19e3bd95f8930885270426` |
| src/installed_parent/resident_tests.rs | `c066e4cb093939a114947c692f99fd067aa75eef71a2704d1fe4e6f6ab167808` |
| src/installed_parent.rs | `d23f4a00bf95a0fb0e96208b5e43da9432d6b2551f1b20d829229c5be4cce45a` |

## Blocking finding: caught future panic does not seal the owner

`Live::poll_once` invokes each stored canonical future directly. If that poll
unwinds and an embedding catches the panic while retaining InstalledResident,
there is no persistent fatal transition. `closed` remains false, the provider
remains open, and the original operation remains available to poll again.
`begin_launch` can also admit the other slot. Whole-inner quarantine is currently
performed only by Drop, which this caller has not invoked. An unwound async
future cannot safely be assumed resumable; partial native effects remain
unknown. This violates the reviewed contract's panic/uncertainty admission fence.

Minimal correction: record a monotonic fatal/quarantine state around actual
stored-future polling. On unwind close the SAME provider and supervisor launch
admission, prevent later operation polls and new launches, and retain the whole
original inner owner. Resume the panic if preserving existing panic semantics;
do not convert it into unused proof or release any slot/charge. A narrow private
regression should inject a panicking owned future and catch the outer unwind,
then require new admission and reentry refusal, unchanged actual charges and
original backend retention. It must not drop/refund the uncertain future or
claim native cleanup. Preserve this first reviewed revision and the new RED.

## Other reviewed invariants

The ordinary cancellation path stores operations in slots before their first
canonical poll. Dropping a `drive_once` await drops only its borrowed poll_fn,
not those operations. Both slots are polled before returning an event, and the
returned event rotates the starting slot. Stop requests latch through launch;
they do not abort or replace that original future. This establishes bounded
cooperative fairness, not preemption of synchronous native stop work.

Private IDs contain slot/actual grant serial and stale lookups compare both.
The third occupied-slot refusal precedes provider preparation. Tenant is fixed
to the actual LocalBackend local tenant. Backend operations capture the same
private backend Arc; no name/PID backend reconstruction or legacy stop_machine
is used. Supported profile binding remains the original native role; the Mock
client in the component test only produces a genuine canonical pure refusal.

Operation storage is reserved before arming the original launch owner. Its
capacity is the maximum concrete launch/stop-outcome/stop-pending future layout,
and that reservation remains with the slot until completion. Later cleanup
uses retained identity, permitting normal close without a fresh reservation.
The initial reservation covers the concrete Live box, backend/provider Arc
bodies and two outcome Arc bodies. These claims deliberately exclude captured
heaps, native allocations and RSS; they are not a full endpoint resource model.

Ready results are retained per slot; polling one operation ready does not
discard the other's ready outcome. Pure preflight failures preserve their real
sealed certificate and call the original owner's cancel_unused. Converted
failures retain their binding/owner/backend for stop_owned_pending. Successful
pending receipts and outcome receipts survive parent-completion errors in
Retiring states. No missing record, Mock outcome, error string or empty endpoint
receipt becomes positive native retirement. Current endpoint creation remains
Unsupported; future actual endpoint/store cleanup is still a separate gate.

Positive completion drops the certificate/outcome/receipt together with the
completed parent owner. Incomplete Drop intentionally leaks the entire bounded
inner owner plus supervisor, retaining backend, provider, operations and charges.
Clean try_finish first refuses occupied slots, then drops provider/table storage
before the SAME supervisor's global finish. A Busy global finish retains the
original supervisor for retry. Weak registration permits actual last-owner
drain without resetting installation or accounting state.

Root reports two private unit entries and one external actual two-slot unused
preflight test GREEN, protected at these inputs. Those records do not exercise
caught unwind and do not establish a native positive receipt, guest boot,
endpoint activation, aggregate numeric profile or WAN acceptance. Source PASS
is withheld only pending the concrete fencing correction above and its review.

## Caught-unwind successor: source PASS

Reviewed the narrow correction at resident.rs SHA-256
`b819e4f3c60dedae8504a26ac1c477da8da093040f9b7539306e56af73a3749b`
and private tests SHA-256
`1e28e76e97832b03beb53ae5f944c51df6dec8cc6e7875f7e94a19131b4d6f0c`.
**The blocking source finding is resolved; scoped Code/Rust/Security PASS for
this successor.** The first source/HOLD above remains historical, not approval
of the unfixed version. No additional builds or tests were run by this reviewer.

`drive_once` now guards both requested cleanup scheduling and every polling turn
with catch_unwind. On an unwind it records monotonic quarantine, closes the SAME
provider and supervisor admission, moves out and deliberately forgets the entire
inner owner and supervisor, then resumes the original panic payload. Thus a
catching embedding retains only an unavailable facade; it cannot regain/poll
the uncertain operation, admit another launch or report global completion.
The original backend, provider, future and all remaining reservations stay
together. Normal cancellation still does not enter this branch. This is
deliberate fail-closed custody, not successful asynchronous cleanup or recovery.

The new controlled-future regression catches the operation unwind and requires
new-launch refusal, drive refusal, finish refusal, exactly one poll, zero drops,
unchanged actual ledger usage and surviving original backend. Dropping the
remaining facade must still leave those charges and future retained. Root
reports the predecessor produced a genuine RED at the new-launch assertion;
successor protected execution and strict Clippy were still running when this
delta was reviewed. Their outcomes must be recorded separately. This controlled
panic qualifies resident policy; it is not an injected native-process failure
or a positive native retirement witness. All previous endpoint/resource/native
acceptance exclusions continue to apply.

## Lint-only successor: source PASS carried forward

Reviewed exact resident successor SHA-256
`d09fb8eac46a48246a894645a2ac763e810196dead0f5f481e4f112bcacb998e`
against the preserved b819 source (its original hash rechecked). The entire
delta is two explained `clippy::large_enum_variant` allowances on private State
and OperationResult, plus three let-chain regions addressing five nested-if
findings. **Scoped source PASS carries forward.**

The allowances preserve fixed-two-slot prepaid inline custody; they introduce
no blanket lint suppression, new boxes or cleanup-time allocations. The
let-chain conditions preserve their original left-to-right short circuiting:
cleanup is scheduled only for the same stop-requested eligible slot; polling
occurs only for the same present, not-already-ready operation; postlaunch stop
runs only when requested. Ready storage/removal, error retention and return
order are unchanged. The unwind guard and quarantine disposition are untouched.

Root separately reports the prior b819 protected three-unit plus one-external
GREEN with nine source inputs/two binaries matching. Its strict Clippy failure
is retained as a failure. Final successor protected rebuild/Clippy were pending
at this source review; neither is inferred from this style-only verdict. No
tests, builds or product changes were performed by the reviewer.

## Independent private-test fitness and final executed receipts

**Private-test fitness PASS** at final resident_tests.rs SHA-256
`3f58b74f1b3524533afa45c1cfa9cdde11b7ff9d816863795b503fcc4c2cf824`.
The delta from 1e28 strengthens incomplete-Drop exclusion from any error to exact
OwnerError::Busy; it does not weaken product behavior or change the fixture.
The unit-only `prepare` bypasses process-global provider installation explicitly,
while using the actual private provider, actual managed reservations, original
backend Arc and genuine prepared Root grants. It is a driver-policy fixture,
not an alternate production constructor or canonical admission bypass.

Gate retains the original operation in its boxed inner field. Dropping the
borrowed drive future leaves both gates undropped and each polled once. Manual
release plus an explicit new drive then runs genuine pure-preflight refusal;
the gate intentionally does not promise runtime wake delivery by itself. The
assertions qualify the same two operations' cancellation retention and fair
polling, not arbitrary asynchronous scheduling. The separate Drop case requires
actual usage equality, surviving backend Weak upgrade and exact kernel owner
Busy. Panics is a controlled replacement operation: its one-poll/zero-drop
checks exercise the facade's unwind policy, not native effects. Instrumentation
adds wrapper allocations outside the production size model, openly excluded.
No fixture manufactures a positive native or endpoint retirement receipt.

Read the compact final-receipts archive, SHA-256
`0329f78131f2e126be8ebc27e933891650ad76ca40ab90e0d4a88040e3677fbb`.
It contains actual three-unit PASS (128 filtered siblings), one-external PASS,
both release-build exits zero and Clippy `-D clippy::all` exit zero. The latter
retains six actual unconnected endpoint/disk dead-code warnings. All nine
before/after source entries match, as do protected lib executable
`d5770bd1cd4a56409ac818a722cffe894fcc2d14cd4766c24a4bc1b45ffcb6a4`
and external executable
`2833c2b4d09d1d40050b45bf987c4da8d39a5492fb68c17ac01f2c940ad95ffa`.
Product remains d09. These are observed retained receipts, not reviewer reruns.
The external test was authored by this reviewer and independently reviewed/run
by Root; no independent test-authorship claim is made here. All native/profile/
capacity/WAN exclusions above remain open.
