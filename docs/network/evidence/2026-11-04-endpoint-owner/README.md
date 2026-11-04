# Original endpoint-store handoff

2026-11-04. Linux x86_64 on Wick, Rust 1.97.1. This development checkpoint
implements the installed parent's endpoint owner and resident receipt consumer.
It is component evidence. **No guest, endpoint helper, transfer or WAN run was
started by these tests.**

The parent now issues one actual canonical `PreparedEndpointPartition` from
its original guest grant, one endpoint memory/worker reservation and the real
disposable-store inventory. The same guard Arc is retained by the grant and
canonical partition. Its weak back-reference avoids a grant/guard ownership
cycle. Store paths and device/inode/serial/session facts come from the actual
prepared store. Session identity is derived at binding from the real admitted
plan/nonce and original installation/generation/grant; it grants no authority.

Issuance becomes one-shot before resource/filesystem effects. Partial or
uncertain cleanup retains the original guest, endpoint and disk owners. An
unproved Drop cannot refund them. The resident retains the sealed endpoint
receipt outside its guard, avoiding another strong cycle, and consumes the
outcome/pending receipt once before parent completion. Disk cleanup runs
outside the outcome and grant-state locks. A failed cleanup keeps proof and
owners for retry; ordinary admission close does not grant new resources or
prevent identity-validated cleanup.

The endpoint profile still has **no production activation constructor**. The
selected bootstrap/runtime/codec/carrier resource model and native integration
must be qualified first. Tests alone use private synthetic limits. Default
registration keeps the previous 4 KiB footprint and refuses endpoint preparation
before effects when the qualified profile is absent. This implements the owner
path without treating a quota DTO or caller Boolean as permission.

## Executed checks

| Check | Result and scope |
|---|---|
| API absence before implementation | Release compile exit 101: seven missing module/field/method errors |
| Initial component | Two endpoint, three resident and four retirement-policy tests passed |
| Foreign-supervisor regression | Release build exit 0; test exit 101: the private configuration seam accepted a foreign disk inventory |
| Original-pool identity fix | Rejects the foreign supervisor before inventory allocation or mutation |
| Cleanup-entry regression | Release build exit 0; test exit 101: a caught synchronous request-stop cleanup unwind left admission open |
| Guarded cleanup successor | The same existing unwind guard now covers synchronous stop scheduling and cleanup |
| Final component | Three endpoint, four resident and four retirement-policy tests passed (11 entries) |
| Final strict Clippy | `--lib --tests -- -D clippy::all`, exit 0 on the same final source |
| Source/executable custody | Seven selected sources and the protected executable match before/after those tests and the local publication copies |

The three new cases prove absent-profile refusal without new credit/effects;
foreign-supervisor refusal; and actual store/guard identity, one-shot issuance,
close/missing-receipt refusal, acyclic ownership and conservative uncertain-Drop
credit retention. Private fixture admission is explicitly **not** a canonical
admission or positive native retirement witness. The existing driver panic
test intentionally emits its caught controlled panic and passes its fence and
ownership assertions. Test scopes overlap earlier checkpoints.

The added cleanup regression obtains a real unused certificate from the first
stored canonical operation, then injects a completed-slot diagnostic Drop panic
during `request_stop`. The first positively unused grant may release; the
second original unpolled grant/backend remains retained, with one guest and
1 GiB logically charged. Catching the panic cannot reopen admission, drive or
finish. The independent review retains the original HOLD and successor result.

The actual locked public dependency graph is unchanged from the
[resident checkpoint](../2026-11-04-resident-service/README.md): MVM
`f708539f0c2317acf2e74f8f7833d6072a5abafe`, shared core
`ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1`, adapter
`dee6e52a712c7b3dc12ea09be8bc16d3754b08f7`. No local dependency overrides.
The active RawValue/bootstrap and reader-deadline successors require their own
combined graph qualification.

## Open integration gates

Positive sealed endpoint receipt → durable store retirement → original native
parent completion remains unexecuted. Retry after an injected disk barrier
failure, full configured resource-model activation and original native shutdown
need integrated witnesses. Reader-deadline setup/shutdown fixes remain active
work. No claim of full installed service, sustained Call capacity, bounded
whole-process memory, fleet fairness, platform/WAN or upstream readiness follows.

No new networking bytes, CPU, memory/RSS, transfer latency, energy or send-only
result is asserted. Logical reservation values and build/test elapsed times do
not supply those measurements. Compact official test/failure receipts are
retained; heavy builds and protected executables remain on Wick. No VM disks,
payload datasets or build tree were copied to the Mac.

## Reproduction records

- [Exact commands](commands.json) and [seven selected source identities](source-before.json).
- [Initial API/ownership/failure receipts](endpoint-owner-receipts.tar.gz):
  SHA-256 `15fd7ecd3fde7e5a70fc9c43749e26b30692e2a2bb7f7569a6f03fd233602c66`,
  19 members, 3,468 bytes; [member hashes](endpoint-owner-members.json).
- [Cleanup-unwind RED and final protected tests/lint](endpoint-owner-final-receipts.tar.gz):
  SHA-256 `fad5b6581b468b5daacd63b052f5e0564588f15f21bfbaa905854a7ce047f100`,
  11 members, 2,099 bytes; [member hashes](endpoint-owner-final-members.json).

The archives retain distinct source phases; only the final guarded records
qualify the final successor. They contain no executable or payload store.

[Implementation contract](../../../api/installed-parent-endpoint-v1.md) ·
[Independent source review](../2026-11-04-resident-parent/reviews/installed-endpoint-owner-code-rust-security.md)
