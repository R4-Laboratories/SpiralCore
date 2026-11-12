# Native Windows private object storage v1

Status: the baseline has [native component evidence](../network/installed-supervisor-results-2026-11-03.md#platform-scope).
The 2026-11-10 quota-scan addition requires its own native qualification;
Windows-target compilation alone does not establish that behavior. Delivery
belongs to SpiralCore #46. This is an object receiver/client gateway storage
profile, not a native Windows MVM backend, packet attachment or WAN result.

## Public behavior and ownership

Keep the existing `Endpoint`, `Authority`, `Context`, `Limits`, object IDs,
publication record and protocol unchanged. One internal filesystem module
provides private directory creation/validation, private file opens/creation,
owner locking and verified durable publication. Unix and Windows backends
implement that interface; no second codec, store format or authority is added.

The Win32 implementation lives in the independent Windows-only
`spiralcore-storage-windows` crate. Its safe file-operation interface returns
typed `Storage`, `Unsupported` or `Limit` failures, mapped by the existing
endpoint filesystem module. This isolates reviewed FFI while the entire
transfer/protocol crate retains `#![forbid(unsafe_code)]`. The backend uses
`std` and pinned `windows-sys` only and denies unsafe operations outside explicit
unsafe blocks. Native qualification reports the four Windows endpoint tests
and two backend ACL/junction tests as distinct nonzero scopes.

Storage remains local to the authenticated namespace and authorizes before
publication and before/after object readback. Existing resident, active,
namespace, cache, history, publication and staging limits remain enforced by
the same endpoint. Retry, cancellation, stale-base integrity and restart use
the same application state machine. Platform errors refuse through existing
Storage/Unsupported errors; no fallback to an insecure or ephemeral store.

Initial Windows profile: native Windows Server 2022/2025 x64 and Windows 11
ARM64, a local fixed NTFS volume with persistent ACL support, running as the
provisioning token user. Remote shares, UNC/device paths, other filesystems,
reparse points and unqualified OS/ABI combinations are unsupported. Device
power-loss testing is not inferred from process restart or CI success.

## Private filesystem boundary

New Windows directories/files are created with an explicit protected DACL
granting only the current token's user SID full access, with that SID as owner.
Inheritance from the parent cannot broaden access. Existing stores are
validated, never silently repaired: owner must match that SID, DACL must be
present/protected and contain only the permitted allow entry. Null, absent,
inherited, additional or unrecognized access entries refuse. Trusted
same-user/administrator forgery and kernel compromise are outside this profile,
as trusted same-UID/root mutation is for the Unix store.

Open Windows objects using native handles with OPEN_REPARSE_POINT; inspect the
actual handle, reject any reparse tag and non-disk/nonregular object, and require
one link for files. Validate each existing ancestor as a real directory while
retaining its handle during the operation. Attribute/security-only ancestry
handles do not establish rename exclusion under Windows sharing rules. Bound normalized
drive-absolute paths to 4096 UTF-16 units and 128 components; reject alternate
streams, parent traversal, device/UNC prefixes and ambiguous trailing dots or
spaces. Newly created child names remain the endpoint's fixed/hex names.
Require the original input's explicit drive plus root, and inspect its raw
segments before normalization can erase traversal or aliases. Relative,
drive-relative and root-relative roots refuse before filesystem I/O.

File reads validate the actual opened object and keep write/delete sharing
closed for the read. Root owner locking remains the existing kernel File lock,
held for the Endpoint lifetime. Duplicate owner opens must refuse. Handle
privacy/reparse/identity errors never become successful content reads. Unix
opens keep the existing 0700/0600 behavior and also refuse hardlinked files.

## Publication and recovery barriers

Preserve the canonical `SCPUB001` record and all exact length/digest checks.
Write a bounded exclusive private staging file in the destination's directory,
flush its complete contents with `File::sync_all`/FlushFileBuffers, close it,
reauthorize, then publish by same-volume MoveFileExW with WRITE_THROUGH.
Do not use COPY_ALLOWED, delayed moves or a cross-volume copy fallback. Initial
private directories are first created empty under a unique private sibling
name, then exposed using the same write-through move. The profile marker is
also written/flushed privately and exposed through write-through publication.

No Windows directory FlushFileBuffers failure is treated as a successful
barrier. Windows uses the documented write-through move contract; Unix retains
file sync, atomic rename and directory sync. Successful new publication also
opens the final private file and checks complete length, record identity and
object digest before acknowledging. Idempotent publication verifies the full
existing object and flushes that file; it performs no new namespace mutation.
Never acknowledge on an unsuccessful write/flush/move/readback/authorization.

Only owned reserved stages are eligible for collection, under the exclusive
root lock, after type/privacy validation. Failed operations clean up only
their own freshly created stage. A crash during initial store provisioning
may leave an incomplete private store which refuses reopening; no automatic
reset of an unknown nonempty root. Empty directory-creation staging names hold
no payload and do not authorize cleanup outside the selected store. Publication
failures retain the existing endpoint's refusal/retry semantics; this contract
does not claim atomic revocation or guaranteed outcome after power loss.

### Quota scan addition, 2026-11-10

Windows accounting uses a synchronous scalar-only scan. After initial root
capture through the existing drive-absolute checks, enumeration and child opens
use the actual retained root and namespace handles. Shared ancestors retain
their previous traversal checks and get no new write exclusion. Only owned
directories are pinned with list/traverse/read-control access and read-only
sharing. An incompatible owned-directory writer/deleter refuses the scan.
There is no weaker fallback and no on-wire change.

Native opens use one validated basename with `NtCreateFile.RootDirectory`;
enumeration uses bounded handle-based directory records. Publication
opens preserve their original generic-read/read-control access, sharing,
effective SID, protected ACL, reparse and hardlink checks. Each held file must
match the full enumerated file ID on the original volume before its length
contributes to checked totals. The effective SID is queried afresh and must
equal the opening identity. Only aggregate/scoped byte totals and a count leave
the native function; its pins, paths and content handles remain private and end
before mutation. Ordinary content reads retain their existing implementation.
The admitted NTFS profile requests completed synchronous opens. An unexpected
`STATUS_PENDING` is a fail-stop invariant violation: the process aborts rather
than unwinding storage still potentially referenced by I/O. This is not support
for asynchronous metadata operations or timed cancellation. A separate child
test exercises that path without issuing actual pending I/O.
The [design and native gates](../specs/2026-11-10-windows-quota-batch.md) record
the stricter pin behavior and the unqualified platform/performance cases.

The Windows primitive definitions follow Microsoft documentation for
[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew),
[GetSecurityInfo](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo),
[FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers),
[MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)
and [volume information](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getvolumeinformationbyhandlew).
WRITE_THROUGH specifies completion of the move on disk; actual device/cache
behavior remains a separate physical qualification boundary.

## Focused evidence and native gate

First record a real portable refusal regression before changing filesystem
code, then qualify Unix regression behavior and the Windows implementation.
Existing endpoint tests supply positive durable transfer/reopen/readback,
authorization/revocation, cancellation, quota, poisoned-base and owned-stage
coverage. Add focused filesystem tests for private handle opens, hardlink and
reparse refusal and publication/reopen exactness. Windows-only ACL tests must
prove broadened/inherited access is rejected without repairing permissions.

Compile/typecheck Windows source when available as intermediate evidence;
cross-compilation, WSL and passing Unix tests are not native Windows results.
Root will add an explicit nonzero native test target to the existing seven-row
CI, retain original artifacts/source identity, and independently review this
trust boundary. The Windows native storage gate stays NOT_RUN until those
rows execute, and #46 remains open for its remaining roles/backends/WAN gates.
