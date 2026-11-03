# Registered Call retry with retained Reply ownership

SpiralCore adds an explicit `LocalMailboxService::retry_call_prepared` operation.
An endpoint can resend its original registered Call while that Call is Pending
or while the service owns its unconsumed Reply. The Reply and its memory
reservation stay in the service until ordinary one-time extraction. This closes
an API gap needed when a Reply arrives before the destination receipt for its
original Call. [Source PR #64](https://github.com/R4-Laboratories/SpiralCore/pull/64).
Source merged at `5fb35d928a10a9cb924c78f1d574199b8f8772c5`; its tree
`e5c32fa15b132ca1ad28cfdc5914787079b18df2` equals the reviewed and hosted-tested tree.

The existing `send_call_prepared` remains Pending-only. Both methods share one
immutable validator and the same canonical authorization and transport path.
Original service/binding/handle identity, policy generation, semantic request
commitment, owner-monotonic deadline and signed Unix expiry remain mandatory.
Consumed Replies, terminal Calls and expired or unauthorized requests refuse.
The retry method does not poll, consume or clone the retained Reply, advance an
accounting clock, register another Call, allocate a sequence or reseal a request.

The producer retains the exact original prepared envelope and tracker attempt.
The service checks the existing semantic commitment; it does not introduce a
second store of outer-envelope identity. A successful write is a transport fact.
An error can still accompany a completed or partial write, so publication,
destination admission, application Reply and receipt acknowledgment remain
separate facts.

[API contract](../api/registered-call-retry-v1.md) ·
[Implementation plan](../plans/2026-11-03-registered-call-retry.md) ·
[Evidence packet](evidence/2026-11-03-call-retry/README.md).

## Focused behavior and source identity

Contract commit `a69ac2de` preceded the test commit `7414ba28`. The retained
missing-API RED exits 101 with two E0599 diagnostics; it is a compiler absence
result, not an executed behavioral failure. The first implementation passes
three tests. The expanded frozen target passes six test functions, including
the two existing strict-send tests:

| Gate | Observed result |
| --- | --- |
| Pending retry | Original encoded Call bytes sent unchanged; local and shared accounting clocks remain unchanged |
| Reply already admitted | Strict send refuses; retry preserves charged Reply bytes/messages and exact original request |
| Reply consumption | First poll returns the exact binary Reply and releases its charge; second poll returns no payload; subsequent retry refuses |
| Validation refusal | 27 cases across Pending and unconsumed-Reply states refuse before any connection or send |
| Transport failure | Five faults retain the original request and charged Reply; later explicit retry and exact payload extraction succeed |
| Uncertain outcome | One fault writes the authorized complete frame and then reports Partitioned; the error does not imply no bytes were sent |

The fault case uses the existing authorized frame/route interfaces and real
in-memory transport. It does not establish partial socket-write recovery or
WAN behavior. The test assertions compare exact byte strings and lengths; no
application object-transfer benchmark was added.

Frozen crate source is `4d34c6a722ec5223bf3e25431ab0b73e11f39d54`.
Source PR head `d14271ddec5424326603572693c15aeb1a2f2e48` adds review documents
without changing those crate files. Product `service.rs` SHA-256 is
`5563e59811266047b582651aa6e08682ff3c368d417c2aabdbee8930a1217794`;
tests are `a81c0ff09047261e0430af853ad030a10fa67f0a574a143cbaade9d393d991fb`.
The portable manifest changes only the `call_send` minimum from two to six.

## Qualification

On Wick, locked Rust 1.97.1 release with adapter features `live,conformance` and
all targets passes **127 tests**, with zero failed or ignored tests. Strict
affected-adapter Clippy passes with warnings denied. The 285-entry crate source
manifest verifies before and after. Independent Rust/security and test/manifest
reviews, followed by the scoped procedural audit and final local confirmations,
are retained in the packet.

One isolated coverage run passes 125 tests across 19 fresh instrumented binaries.
The all-target release count additionally includes two example tests outside
coverage's `--lib --tests` selection. Changed mapped executable production lines
are **31/31 (100%)**; the entire mapped production `service.rs` scope is
**2,010/2,169 (92.67%)**. All 159 uncovered whole-file lines remain recorded.
Branch mapping is unavailable; these are line measurements, with no repository,
semantic-path, branch or MC/DC percentage claim. The test scopes overlap.

Each instrumented executable was copied outside Cargo's target and protected
before execution. Profiles and exports were isolated per binary; test/export
stderr and compiler warning counts are zero. Raw profiles, LCOV and executed
instrumented binaries remain on Wick. Compact public records are retained here;
no Mac Cargo target, payload store or VM disk was created for this slice.

Both [hosted Linux](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41649886275)
and [portable-client](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41649886300)
workflows pass at that exact head. The tested merge `1746e57c` and reviewed head
have identical trees. Seven native OS/architecture client rows pass **92 checks
each (644)**, with **eight additional Unix storage checks**. They cover Linux
x64/ARM64, macOS ARM64/Intel, Windows Server 2022/2025 x64 and Windows 11 ARM64.
Windows durable storage is not executed. All seven original ZIP/API digests and
236 inner checksums verify; original archives remain in the compact packet.

Hosted Linux also passes **92 live release** and **135 shared checks = 8
object-v1 + 127 adapter**, strict Clippy, default guest compilation and synthetic
analyzers. Four zero-test result scopes remain explicit in the
[qualification receipt](evidence/2026-11-03-call-retry/qualification/portable/verification.md).
Counts overlap across targets/stages/platforms. A native client process is
distinct from a native MVM backend, durable gateway storage or a WAN test.

## Performance boundary and remaining recovery work

The normal object campaign does not invoke this new API. There is no new claim
about codec bytes, sender/receiver CPU, transfer time, WAN speed, energy or
end-to-end recovery. Previous measurements keep their own source and workload
identities: see the [last strict-transfer/native checkpoint](stream-idle-results-2026-11-03.md).
No repeat performance campaign was run for an unused API.

The next endpoint integration must explicitly reconcile a missing receipt or
Reply using the original identity and finite event/send bounds. Idle alone must
emit no resend. A receiver must authenticate historical Calls and older controls
without reexecuting object work or fabricating a Reply. General lost ACK after
caller completion and closed-stream resume require additional ownership work.
No consumer, timer, reconnect/resume, retention increase, larger pipeline or
2,048-Call guard change is enabled here. Full C8 and issues #43, #49 and #42 remain
open, along with installed host ownership, aggregate plaintext/cache budgets,
scale, remaining platform/WAN qualification and the final MVM contribution.
