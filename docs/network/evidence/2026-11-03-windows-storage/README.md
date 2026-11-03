# Native Windows storage implementation candidate

This packet records the implementation candidate for the lead-approved
[private NTFS storage contract](../../../api/windows-storage-v1.md), part of
SpiralCore #46 and the Ari delivery plan. Native Windows runtime qualification
is **NOT_RUN** in this packet. Linux tests and a Windows-target compiler check
do not establish native Windows operation, MVM support, packet attachment,
power-loss durability or WAN acceptance.

The existing Endpoint, authority/context, limits, object IDs, codec and
`SCPUB001` record remain shared. The filesystem boundary now selects a Unix or
native Windows implementation. The Windows candidate uses protected current
SID ACLs, actual-handle type/reparse/link validation, ancestor handles that deny
deletion, fixed local NTFS validation, private stages and write-through
same-directory publication. Final publication reads every byte back through
the validated file handle with an 8 KiB comparison buffer. Windows cleanup
uses an exact validated DELETE handle; a failed exclusive creation cannot
clean up a pre-existing stage. No failed directory flush is ignored.

## Observed checks

The owned Linux source export and logs are under
`/var/tmp/spiralcore-windows-storage-20261103` on Wick. Rust 1.97.1 used the
existing Cargo target with two jobs and instrumentation flags unset. No VM,
Windows execution or performance campaign ran. Logs, exit codes and original
failures are retained in [validation](validation/).

| Scope | Actual result | Receipt |
| --- | --- | --- |
| Original hardlinked-profile refusal test, before implementation | Semantic RED, exit 101: old endpoint accepted the linked profile | `storage-red.txt`, `storage-red-source.sha256` |
| Same first test after implementation | 1 passed | `storage-first-green.txt` |
| New portable filesystem tests, final candidate | 3 passed, 0 failed/ignored; 43 unrelated library tests filtered out | `storage-candidate-linux.txt` |
| Existing endpoint integration target, final candidate | 21 passed, 0 failed/ignored/filtered | `endpoint-candidate-linux.txt` |
| Actual transfer library and test Clippy on Linux | Exit 0 with warnings denied | `clippy-candidate-linux.txt` |
| Exact Windows filesystem source, MSVC-target compiler-only adapter | Exit 0 with warnings denied | `windows-final-backend-clippy.txt`, [compiler adapter](compiler-only/) |
| Full transfer library/tests MSVC cross-check on Linux | Exit 101: vendored C dependency requires unavailable `lib.exe` | `windows-full-crate-check.txt` |
| Native Windows ACL, junction, durable endpoint and restart tests | **NOT_RUN** pending native CI | [CI gate](native-ci.md) |

The compiler-only adapter includes the exact Windows backend by absolute
source path and supplies only the Error/Result signatures and references to
its callable functions. It contains no filesystem implementation. Its success
does not typecheck the full Endpoint, Windows test module or codec dependency
graph; the unsuccessful full-crate cross-check is recorded separately. No
Windows toolchain installation or Linux C-library substitution was attempted.

The preserved intermediate failures are a missing Windows constant feature,
the Win32 disposition field's required Rust `bool` type, and an initially
incomplete source export missing two existing tracked `include_str!` test
inputs. The feature/import and field type were corrected; the two existing
reference files were copied into the owned export. No test assertion was
relaxed. Their raw logs remain alongside the later successful checks.

## Source custody and remaining gates

[candidate-source.sha256](validation/candidate-source.sha256) pins the seven
owned manifest/source files for review. [executed-source.sha256](validation/executed-source.sha256)
pins the remote source of the initial selected checks. The subsequent path
review correction is documented in [path-review.md](path-review.md), with its
own compiler receipt. [pre-path-review-source.sha256](validation/pre-path-review-source.sha256)
preserves the initial candidate pins. Unix source and manifest hashes remain
identical; the updated Windows backend and additional Windows test require
native qualification. The initial executed test hash also predates rustfmt's
import-line formatting. These scopes are kept distinct.

The selected checks do not represent all transfer tests. Native CI must execute
both the existing endpoint integration target and the new storage library
tests with positive per-platform counts, original artifact/source identities,
and no silently skipped Windows tests. Root owns the workflow and consumer
lock refresh. Independent security review and exact-release qualification are
required before a supported native role can be claimed. Issue #46 remains open
for its remaining role, backend, packet and WAN gates.
