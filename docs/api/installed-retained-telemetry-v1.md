# Installed retained telemetry: original-owner handoff

This API is a component of #42/#43/#49. The Core callback described here is
additive. Its default refuses with `io::ErrorKind::Unsupported`; it does not
activate a Root catalog, native profile, collector or deletion policy.

## Immediate canonical seam

`EndpointPartitionGuard` has a fallible `prepare_retained_telemetry` callback
receiving its full `EndpointPartitionProjection`, a borrowed state path and a
borrowed `File`. `PreparedEndpointHandoff::prepare_retained_telemetry(path, file)`
uses only the projection and guard already held by that original handoff.
It checks current holder/anchor, current parent and partition before calling
the owner and again after a successful callback. It neither duplicates the
File nor accepts a replacement guard, record capacity or authorization bool.
An error is not a no-effects certificate: original ownership remains with the
provider and the surrounding failed-start transaction.

The concrete Root callback must compare the supplied File and path with the
SAME original `RegistrationBinding` state-directory pin and original parent
reservation, tenant, session and full partition identity. The path/File are
inputs to that check, not authority. Missing original state, identity mismatch,
replacement, poison or unsupported roles refuse. No public constructor from
metadata, descriptor getter, new protocol frame or crypto envelope is added.

The installed spawner invokes this on its original handoff after canonical
state-directory creation and original host-service validation, before records
key provisioning, sink File effects, collector threads or Ready. It must use
the strict owned key path and actual preopened verified File with canonical
`CappedJsonlSink::over`, retaining the existing record encoding and cap. Legacy
provisioning does not become a fallback. The current Root/catalog callback and
actual native producer composition require their own implementation/review.

## Retained ownership decision

Keep the records in their canonical per-VM state location for the first
profile. A separate original retained owner holds credit from the SAME
aggregate pool. For ciphertext cap R=DEFAULT_RECORDS_BYTE_CAP (8MiB), checked
C−R−K is the disposable payload-cache share of total cache C. K is Core's
`INSTALLED_TELEMETRY_SIDECAR_BYTES`, currently 32 KiB for logical sidecars.
Require C≥R+K before Preparing, reservation or effects. Deleting the disposable
namespace refunds only that disposable share; R+K stays held after VM/endpoint
retirement. R+K is not a physical-filesystem or whole-state-directory size
claim. Parent and installed child must use the same checked split; the child
producer coupling remains a separate qualification gate.

The installation supervisor retains the fixed-capacity retained-owner table
independently of the VM grant, endpoint guard and resident Live/backend table.
The entry retains the actual original reservation and directory/file identity.
Use weak references back to grants/tables to avoid a receipt/guard cycle. A
VM-slot retirement may complete while records remain live debt; global finish
still refuses until actual debt is durably settled. No zero-accounting exemption
or counter reset is allowed.

The bounded durable catalog belongs to the original installation owner and
serializes with generic transitions. Use distinct telemetry phases/domain,
not disposable-cache recovery: CreationIntent → Active → Sealed → DeleteIntent
→ AbsentDurable. Publish intent before effects and synchronize file/changed
parents before committing captured identity or removal. A sealed endpoint
receipt allows post-writer-join file synchronization and transfer of original
retention custody; it does not authorize record deletion. Missing or replaced
Active/Sealed data, ambiguous creation or failed barriers retain/fence credit.

## Cleanup, restart and deletion boundaries

Canonical `remove_runtime_dirs` currently recursively deletes runtime state.
It must refuse protected retained records before deleting any state or
shortened socket namespace. A fixed marker/record/key presence is a denial
signal only. Publication and actual cleanup require the SAME canonical
cleanup/protection lock: a marker check followed by uncoordinated recursive
removal is insufficient. With flock, separately opened verified descriptions
or an additional process mutex are required; duplicated descriptors for the
same open description do not provide thread mutual exclusion. Nonblocking
contention refuses while retaining custody. Existing per-VM lifecycle locks
precede this guard; no managed ledger lock spans filesystem I/O.

Restart first supports closed debt adoption after original owner/holder
exclusion: validate the bounded catalog and exact directories, import the full
original ceilings before any new admission, and provide inspection/disposition
custody only. Do not call normal begin over old retained debt or enable new
launches from copied catalog fields. Continued launch across a generation
while retaining old debt needs a separately reviewed durable transfer design.

Signed `post_run.destroy_on_exit` means VM teardown; it does not authorize
telemetry/audit deletion. Upstream transient volume/session receipts likewise
cover attempt-owned volume/lease/secret resources, not telemetry, and cleanup
Ok can retain resources. No current inspected authority grants telemetry expiry
or deletion. The initial integration therefore retains records by default;
explicit disposition must later bind a canonical authorized owner/policy,
exact original catalog generation and audited operation. No unsigned bool,
Drained, PID/EOF, successful cleanup call or lost process supplies this grant.

## Focused qualification

Tests precede the Core implementation and must distinguish API absence from
behavior. Component tests cover default refusal, same guard/projection/File
borrowing, original-owner callback rejection of foreign/replaced directories,
and current parent/partition checks before and after delegation. Their private
fixture directly constructs component state; it is not native admission or a
resource/exit proof. Exact Linux source/binary qualification is recorded in
the associated retained-telemetry API packet when executed. Numeric profile,
Root catalog, restart adoption, actual collector transfer/stop and native/WAN
acceptance remain separate gates.

Actual-file custody additionally requires a second original-owner binding after
strict key provisioning and before collector writes: the same handoff must
bind the borrowed original records and wrapped-key Files against its existing
CreationIntent/state pin, retain their actual identities and publish Active.
The matching `bind_retained_telemetry_files(path, state_file, records_file,
wrapped_key_file)` method now delegates through the same original guard and
projection, with fresh original-custody checks before and after success. This
requires separate second-API qualification and is not part of the first
method's original three-test checkpoint. Both defaults refuse unsupported providers. A directory pin alone must never be reported as record
or key File ownership, and the writer must consume the same verified records
File through canonical Sink::over rather than reopening its path.

The [component packet](../network/evidence/2026-11-04-retained-telemetry-api/README.md)
records genuine missing-API compile REDs before each method, final Linux release
six-test GREEN, separately protected six-test GREEN and scoped strict Clippy
GREEN at the exact two-method source. It also preserves the initial command
routing/locked-resolution failure. This qualifies Core delegation only; all
provider/catalog/producer/native and independent-review gates above remain.
