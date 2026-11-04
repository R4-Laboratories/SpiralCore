# Parent launch-owner retirement: scoped Code/Rust/Security review

2026-11-04. **PASS for this source component; no actionable blocker found.**
Reviewed the actual file contents against public development checkpoint
`665e23349e8cf3c644af989b2069815f7885b779`, not the current Git index's tracked-file
selection. The reviewer proposed the prior contract addendum but did not author
the product or tests. No build, test, native process or VM was executed by this
reviewer. This does not approve a later failed-startup finisher or endpoint factory.

| Source | SHA-256 |
| --- | --- |
| `src/installed_parent.rs` | `f652fa5078d4b02c75c2a6308f1b6c42e3e6c99156878839efc7c15cf94b0c14` |
| `src/installed_resources.rs` | `02a7e34868816ec751e249997622979eed4bf8d81dfcad96ece9234c46f6c1e6` |
| `tests/installed_parent.rs` | `5a2034fbb6488955cbc04449ebec40b879033eec5dbc28c79188b2d4eaf8d7ba` |

Paths are relative to `crates/spiralcore-mvm-live`. The reviewed contract is
`docs/api/installed-parent-retirement-v1.md`, SHA-256
`901a7f76b386416d21ac0ba7fb51544511bf336cfd5ee897a33688fdae63e942`.

## Identity, locks and completion

`finish_retired` first borrows the actual canonical outcome's sealed
OwnedRetirement. It compares the full original HostLaunchBinding and the Arc
identity of the SAME VmGrant coerced to CoreCurrentParentPermit. Equal copied
metadata, another slot/provider or an ordinary admitted outcome cannot satisfy
those checks. The published MVM8b833 receipt type and `with_owned_retirement`
behavior were checked in the earlier sealed-wrapper source review; no new DTO
receipt constructor is added here.

The outcome mutex is released before original ledger validation or lifecycle
locking. No filesystem, endpoint or disk cleanup occurs inside that callback.
Ledger validation releases its own mutex before the lifecycle mutex is acquired;
the existing bind path releases the lifecycle mutex before calling current checks.
The delta introduces no reverse nested lock order. Scope is synchronous bounded
identity bookkeeping, not an async teardown transaction.

The private lifecycle record is on VmGrant, shared with the future producer,
rather than a disconnected completion Boolean. Sealing requires canonical
admission and either NeverIssued or CleanupComplete. Preparing, Issued and
Uncertain refuse without resetting their state. Retired is monotonic and is
checked by both preparation/current-permit paths, so positive completion cannot
leave an old grant launch-current. Repeated matched completion is idempotent.
The concrete endpoint method still inherits Unsupported; four future endpoint
states are currently constructed only by policy tests. Their retained production
dead-code warning is an honest integration gap, not a suppression or activated
endpoint profile.

The owner starts armed. Every refusal, poisoned lifecycle or unwind before
completion leaves its conservative Drop disposition intact. Only successful
receipt, retained-identity and lifecycle checks set completed. Completed Drop
releases an Arc normally; it does not take a reservation from another holder or
refund twice. The outcome anchor and any other actual grant owner continue to
retain the SAME reservation until their last Drop. No guests=0 claim follows
while those owners remain live.

## Cleanup-safe original resource identity

`check_retained_identity` deliberately does not require launch admission open and
does not do filesystem I/O or issue new permission. It validates the original
pool's slot and serial, bounded tenant lookup, and non-underflow of both global
and tenant usage by that slot's exact claim. All lookups are checked. Missing or
inconsistent accounting fences the same pool; existing fenced state and mutex
poison refuse. Existing quarantine retains installation exclusion on those
accounting failures. Normal close therefore permits already-authorized cleanup
without reopening admission or swapping in a replacement resource pool.

The original reservation Arc prevents legitimate concurrent last-owner removal
of its slot during the short check-to-seal interval. Other safe ledger operations
cannot replace that live serial. A concurrent unrelated accounting fault remains
monotonically fenced and cannot become positive global installation retirement.
This is cleanup identity, not a new current-owner filesystem attestation.

## Evidence boundary

The implementation owner records actual missing-method API RED, missing private
policy types RED and missing retained-identity method RED before implementation.
Reported protected GREEN scope is three private state-policy cases, three
selected resource cases including the existing corruption regression, and eight
parent entries including one API-shape-only entry. Five input hashes and two
protected binaries reportedly matched before/after; this reviewer did not rerun
or replace those records.

The source tests correctly distinguish private policy eligibility from a sealed
native receipt. The API-shape entry proves compilation, not execution of a
positive native finish. No public test receipt constructor, caller success flag,
endpoint quota/profile activation or weak/PID retirement evidence is introduced.
Actual positive sealed/native consumption, endpoint cleanup integration and a
separate original failed-startup completion path remain open. This PASS does not
promote the component to complete resident/native/WAN acceptance.

## V2 pending-retirement delta

2026-11-04 successor source review. **Product delta PASS**, with the separately
noted negative-harness fixture correction required before relying on its run.
The v1 review and its original exact hashes remain historical above. No new
build or test was performed by this reviewer.

| V2 source | SHA-256 |
| --- | --- |
| `src/installed_parent.rs` | `1770b2a034e18a7e6610978a2df4062166bf483217de20d3561dfe744db39017` |
| `src/installed_resources.rs` (unchanged) | `02a7e34868816ec751e249997622979eed4bf8d81dfcad96ece9234c46f6c1e6` |
| `tests/installed_parent.rs` | `e042a0434b6646a9d892edb838a6ea9e9f4907567ae59a3bacb84783831f5458` |
| `docs/api/installed-parent-retirement-v1.md` | `c6bff4705a8bc237f0528757c6edab66626a26b221961723523564daaea9c962` |
| Harness `Cargo.toml` | `1060c78bdefcf3f9c5093b7c6a77ee005d6ca49d57f6f1a1a71796f148675469` |
| Harness `retirement.rs`, initially reviewed | `5ecbe25e23277de32abd7abc1a8b223bb1c223d51142fb314754e8b04bcd15a5` |

`finish_pending_retired` borrows the linear canonical OwnedRetirement and invokes
the SAME `require_original_retirement` and `complete_already_drained` helpers.
The original outcome method executes only the comparison helper inside its
short callback and returns before ledger/lifecycle work. The pending entry has
no outcome mutex to acquire. Both require full binding and identical retained
permit allocation, then the same original ledger and private endpoint state.
The caller keeps the pending receipt across error/retry; no owned receipt is
consumed or manufactured by this API. Completion flags and Drop disposition
remain unchanged on refusal. No fresh launch permission, filesystem check,
endpoint factory or quota increase is introduced.

The actual published MVM8b833 public client reexports OwnedRetirement. Its
`stop_owned_pending` operates on the original backend's retained pending slot,
checks backend identity, stops it and extracts the sealed retirement. This
additive borrowed entry therefore closes the failed-startup API shape without
treating a formatted launch error, sidecar or absent pending record as proof.
The recorded E0599 probe is compile RED, not an executed positive cleanup.

The negative harness uses one consistent Git8b833 MVM dependency identity and
the exact Live crate, with test-support enabled only in its standalone manifest.
It binds a real opaque native-shaped AdmittedPlan to the original grant before
creating an ordinary Mock outcome with the same visible VM name. Its assertions
target absence of sealed custody rather than a conveniently unadmitted grant;
it exposes no public test receipt constructor. Successful mock shutdown still
must not complete or refund that original native grant. This is a negative
component test, not a native guest or positive retirement witness.

One source-level fixture issue was reported before execution: the initially
reviewed harness uses `tempfile::tempdir()` for the installation's parent. The
selected tempfile Unix implementation uses ordinary DirBuilder permissions
unless explicitly configured, so default umask can produce a nonprivate parent.
Use the existing explicit0700 Builder/Permissions fixture pattern before
initialize; do not relax the owner contract. This is not a newly observed test
failure or a product blocker. A corrected fixture hash/delta and actual run must
be recorded before claiming the negative harness passed. Positive native and
pending-startup receipt consumption remain unexecuted gates at this review.
