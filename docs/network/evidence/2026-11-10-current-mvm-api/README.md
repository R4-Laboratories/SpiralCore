# Current canonical MVM mailbox API checkpoint

This packet retains the original `241a2fe9` test and coverage identity. The
[upstream refresh](../../../research/2026-11-10-mvm-mailbox-upstream-refresh.md)
records the newer merged target and rebased contribution; these older counts
must not be relabeled as results from the successor.

The contribution fork publishes [241a2fe94907d160641adaf2ec3a97fc849910b8](https://github.com/Casey-allard/mvm/commit/241a2fe94907d160641adaf2ec3a97fc849910b8)
on inspected upstream `d27a20cb1a12a7c21b253a3bf6b7579925a7d1f3`.
The four-file patch SHA-256 is
`0e5af83ba7abafa95417b5f927ce91b965a0418d52e1b1fe8d50b40593b83790`.
This is the first source slice of the [current-native plan](../../../plans/2026-11-10-current-native-mailbox-integration.md).
The complete upstream delivery PR has not been opened.

## Behavior and compatibility

`OpenedMailboxEnvelope` retains the exact authenticated header with its plaintext.
Only the existing canonical verifier constructs it, after all previous checks
and one replay admission. Borrowed accessors add no payload copy. The legacy
`open_envelope` delegates once and returns the original payload allocation.
The result has private fields and no public constructor or deserialization;
its Debug output reveals neither header nor payload. Verification under the
supplied context remains distinct from fresh admission, application processing
and durable publication.

Outbound authorization requires Send for Call/Cast and Reply for Reply, deriving
the operation from the private prepared header. Capability, route, epoch,
signature and ciphertext checks remain. Wire bytes, crypto and envelope versions
are unchanged. Reply callers need Reply authority; exhaustive matches on
`OutboundAuthorizationError` need the new `ReplyOperationRequired` variant.
The existing `AuthorizedMailboxFrame::as_bytes` is reused. No redundant accessor
or historical installed-service implementation is imported.

## Executed evidence

[Original logs](tests.zip) contain 18 individually checked members; the
[archive receipt](tests.receipt.json) pins their identity and archive SHA-256.
All runs use Rust 1.97.1 on Wick. Tests use public synthetic data.

| Check | Actual result |
| --- | --- |
| Upstream crypto baseline | 18 passed |
| First verified-header test | RED: canonical function absent; then 19 crypto passed |
| Reply-only capability | RED: `SendOperationRequired`; then passed with corrected operation |
| Final crypto suite | 20 passed, zero failed/ignored |
| Private-field compile-fail doctest | One passed; actual compiler error E0451 names both private fields; nine unrelated doctests filtered |
| Strict core all-targets Clippy | Passed with `-D warnings` |
| Contract protocol-only, no default features | Cargo check passed |
| Initial default parallel core run | 2,053 unit tests passed, two environment-sensitive fixture failures; integration targets not reached |
| Controlled serial default core all-targets | 2,113 passed, zero failed, four existing ignored entries |

The initial fixture failures are retained. The reusable file-mode test expects
0644 creation, whereas the initial shell used umask 077. The exact compiled
binary reproduces failure under 077 and passes under 022. The hung-systemctl
fixture invokes unqualified `sleep` while other parallel tests alter PATH.
The same binary passes with a usable PATH and reproduces the immediate-failure
signature with PATH `/nonexistent`. Its original child stderr was suppressed,
so that replay does not prove the precise original child-exit reason.

The serial profile uses fixed Rust/system PATH, umask 022 and one test thread.
It changes no assertions or product code. Its four existing ignored entries
are three systemd live tests, including one that freezes its manager, and one
command that regenerates committed fixtures. They were not executed. Two test
targets contain zero active tests. Whole-workspace and feature-gated acceptance
remain unqualified; this profile is not a blanket parallel-suite pass.

## Focused coverage and custody

The [coverage receipt](coverage/summary.json) runs 23 selected transport unit
tests plus 20 crypto integration tests, all passing with zero ignored. The
transport filter leaves 2,032 other unit cases unexecuted. Changed mapped
production coverage is **43/43 executable lines**: mailbox 37/37, transport 6/6.
Whole production is 354/375 and 340/376 respectively, aggregate 694/751
(92.4101%). Branch coverage is unavailable. All 4,430 source snapshot pins,
353 compiled-source pins and two original/copied binary pins stay stable.

Only this package and its final integration driver are instrumented; dependencies
are not. The first library-only collector attempt produced no crypto-driver
profile and was refused. Its [negative receipt](coverage/attempt-v1/collector-refusal.json)
is retained. Raw profiles, exports and protected binaries stay on Wick at
`/var/tmp/spiralcore-mvm-api-coverage-20261110`; the compact packet is 132,382 bytes.
Its frozen-patch label records collection before publication; the same patch
bytes are committed at `241a2fe`.

Independent code/Rust and security source reviews pass on the patch above.
The current canonical `mvm-mailbox` release helper builds successfully from the
same source. Its 2,892,824-byte executable has SHA-256
`aa9364510a63e7403f50f0df20a9fddaa6781fe8cecda39389b149a6d3ce4d56`.
The protected executable remains on Wick, outside its source tree.

## Remaining native delivery

No native MVM or WAN object transfer is executed by this API checkpoint; it establishes
no new byte, CPU, WAN, FIPS or fleet-performance improvement. Existing SpiralCore
campaigns retain their `ef1067e3` dependency pin. The next consumer uses the
current `mvm-client` launch/grant and original helper, with a rebuilt current
guest runtime, audited receive and observed original-process shutdown/reap.
Retained producer identity, admitted return/Reply delivery, aggregate service
ownership and the full native/WAN/platform/capacity gates remain in issue #43
and parent #42.

The Mac received only this compact evidence, source and documentation. No local
Cargo target, payload store, VM disk, PE or instrumented executable was added.
