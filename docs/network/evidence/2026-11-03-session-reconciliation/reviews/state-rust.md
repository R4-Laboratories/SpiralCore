# Retained explicit-retry state: independent CODE/RUST review

2026-11-03. **PASS — no blocking finding in this frozen state-helper slice.**
Reviewed the 51 added production lines at
`86383931eb8883323e41695448f351d7f1028554` and the two explicit_retry unit tests
from `40060f70`, against the contract committed at `0a6ceec9` and platform
amendment `26787bff`. This is a partial state-source approval, not consumer,
public-entry, release, Clippy, CI or full C8 qualification.

```text
20ae5413ab61cf015dab8c833bed25ee6f033eaff5563e186ef642c42e6342de  crates/spiralcore-mvm-live/src/campaign/session.rs
3175e6784699ab6e66e27909c9d25a93a137e6f1377f12aa93082dd5078429c3  crates/spiralcore-mvm-live/src/campaign/session/tests.rs
bdd2441b0d757e28c3f9b7bf7c6fd7b75f7d380ca8f0372f55e24e47524794ec  docs/api/session-reconciliation-v1.md
```

The existing send_started latch is preserved. A newly allocated PendingCall has
zero send invocations; only successful original start_send changes it to one,
after the existing fence/latch/handle guards. Repeated original start_send still
refuses. check_retry requires an invoked original send and both retained handles,
rejects a fence and rejects count eight. start_retry checks that guard and uses
checked addition, so the original plus seven retry starts consume the entire
eight-invocation budget. There is no reset/refund and no change to the cumulative
sixteen-event state. These state operations do not claim actual transport writes.

prepare_retry first maintains canonical service and tracker expiry, then applies
the retry guard and unchanged original monotonic/Unix remaining lifetime. This
ordering permits expired active-credit cleanup even when the eventual request
refuses. A clock/maintenance error can take precedence over the later fence/state/
send-limit/lifetime guards; that is an explicit consequence of maintenance first,
not a silent lifetime refresh. Fixed request options, payload/prepared/encoding,
Call handle, Send handle, receipt ranking, authenticated Reply wire/ACK facts and
FIFO state are not replaced or cleared by the helper.

The helper calls the existing tracker register_send using only the retained exact
encoding and requires its returned handle equal the retained SendHandle. For the
same retained entry/attempt the canonical tracker revalidates current bindings,
generation, peer key, commitment and lifetime without adding an attempt or
clearing observations. No destructive service.poll_call is introduced. In
particular, charged Reply plaintext stays in the canonical service until ordinary
completion extraction; there is no second retained payload or borrowed permit.
Expiry maintenance may legitimately release expired resources. No policy lock,
socket I/O, thread, unsafe Rust or replacement sequence is added here.

The first unit test obtains a real independently prepared canonical Reply,
authenticates its tracker handle, admits its plaintext to the service and checks
the exact charge before retry preparation. It proves unchanged wire/header,
handles, lifetime, tracker snapshot, receipt/Reply/ACK facts and payload budget
through seven retry starts, while accumulating sixteen events. Ninth start and
seventeenth event refuse; final canonical polls extract the exact binary payload
once and release its charge. The test explicitly labels itself state/accounting
proof, not seven actual sends or successful ACK delivery.

The second test covers missing registration, missing initial send, an established
fence, exact signed Unix expiry and original monotonic expiry with wall rollback.
It asserts typed failures and retained identity/count/event fields. Expired
cases require active_calls zero, exercising the maintenance-order correction.
There are no background workers or join obligations in these unit fixtures.

Root reports missing-API RED with 15 E0599 diagnostics, then one passing and one
failing test because initial Unix-expiry refusal left one active credit. That is
a substantive retained failure, corrected by moving maintenance before guards;
root reports final two-test GREEN at the reviewed source. These execution results
were not rerun by this reviewer and are not expanded into a release claim.

Integration still must enforce callback/fixed Instant lifetime, decide retry need
from current completion/FIFO facts before consuming commands, burn immediately
before the authoritative adapter call, and fence on terminal/uncertain outcomes.
prepare_retry/start_retry themselves do not perform transport eligibility or
final service-state validation; the existing adapter retry API remains the
pre-I/O authority. Passing this helper gate cannot qualify the unfinished public
entry, caller/receiver protocol, resume, Windows consumer or end-to-end recovery.

Only this review file was written. Product diff whitespace checking passed. No
product/test edits, builds, tests, broad audit or additional qualification ran.
