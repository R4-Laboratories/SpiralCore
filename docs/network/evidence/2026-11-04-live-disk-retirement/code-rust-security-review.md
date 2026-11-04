# Live disk retirement primitive and sealed wrapper: scoped source review

2026-11-04. **Code/Rust/security PASS for the source boundary below; no new
blocking finding.** This is independent source review of the newly added live
cleanup path, not transfer of the older inventory approval. The reviewer
diagnosed the retry failure and proposed the private progress distinction but
did not implement the reviewed product or tests. No build, test, VM, or native
consumer was executed by this reviewer.

Hashes were captured before inspection and rechecked unchanged afterward:

| File | SHA-256 |
| --- | --- |
| `src/installed_owner/endpoint_disks.rs` | `be85f32586132cf2f515812832680baa2fc301e66144f05994f554d95c24a0a3` |
| `src/installed_owner/endpoint_disks_tests.rs` | `3a5221e2429fb981e885434f1934daf391270d64f923082ffe621339a1c4559c` |
| `src/installed_disk.rs` | `c074f17f220e7f11cc6dcaef8463ac5a3e87df512f938ab059959de331b1250a` |
| `docs/specs/2026-11-04-endpoint-disk-inventory.md` | `817d8f43267063a16bb92bf7bc7d65082ae84564e75081bab4ee118a33184d10` |

The first three paths are relative to `crates/spiralcore-mvm-live`.

## Retry and filesystem ordering

The retained original failure was a genuine OS-behavior assumption, not grounds
to relax the test. The lead's captured diagnostic reported `nlink=2`,
`namespace_synced=false`, inventory phase3 after successful unlink followed by
the injected pre-parent-sync error on macOS. This review inspected the successor
source; it did not independently execute that witness.

The new private state transition is correctly placed immediately after successful
`unlinkat`, before the fault hook and parent sync. Live cannot infer removal from
name absence or serialized phase3. NamespaceUnlinked allows retry of the original
parent sync only with the matching phase3 entry and absent name. NamespaceSynced
records a successful parent barrier and alone can resolve a missing inventory
entry after final-publication uncertainty. Complete is assigned only after the
publisher returns success. No link-count inference remains in this live path.

The current owner/key/generation/Active phase is checked before cleanup. The
retained directory's device/inode and normalized full inventory entry identify
the original store. Existing namespace traversal is descriptor-relative,
NOFOLLOW/private, bounded and confined to the recorded store/device; the root
is revalidated through existing publisher/owner checks. The durable phase3
publication precedes removal. Parent synchronization precedes removal of the
inventory record. Publisher failure retains the original object and phase for
retry or conservative recovery; incomplete publisher names remain refused.
After Unlinked/Synced, an unexpectedly present namespace refuses before another
walk/unlink, preserving recreated bytes. Complete returns idempotently only
after the current owner/epoch check. This adds no claim against arbitrary hostile
same-UID concurrent writers beyond the existing private single-writer model.

The focused tests preserve post-remove and final-publication fault cases,
idempotent completion, another live holder/store, missing Active and phase3
namespace refusal, missing inventory before original parent sync, and recreated
namespace byte preservation. Their source is meaningful for these boundaries;
the lead's earlier executed results remain separate evidence.

## Sealed receipt, identity and quota lifetime

`retire_with_endpoint_receipt` first requires the exact retained partition guard
by Arc identity, then compares the receipt's **full projection** to the expected
projection. Its store match additionally pins versions/profile, installation
key/profile/generation, original parent serial, session, path, device/inode,
reservation serial, tenant, and exact cache/staging ceilings. Other projection
fields are covered by full projection equality; the store helper does not
pretend to reconstruct plan or native backend authority from bookkeeping fields.
Mismatch returns before owner filesystem I/O or quota mutation.

The actual MVM type was inspected at the existing Git pin
`8b833aee8be5c1be9ce0d1197b6b98d4d03625f6`, including the VMM-owned definition
and client reexport. `EndpointRetirement` has a private original handoff field,
no DTO constructor or Clone, and is extracted once only after matched local
drain and successful status collected from its actual Child. Guard comparison
delegates to `PreparedEndpointPartition::retains_guard` (`Arc::ptr_eq`).
Supporting source hashes:

- VMM `network_endpoint_spawn/owned.rs`:
  `620bf7f40394ddceaf0de6db69e48bff58ce403364d46654aa0fbc7e3f926f58`.
- Runtime `workload_runner/runner/installed.rs`:
  `5b804a20e158cc382c0be3bc6abb954a0507cccc3a38f56088d2a4031c6f9df2`.
- Core `host_lifetime/endpoint.rs`:
  `4e6516b898122dcd1dc6d715d52bb578538123814295d5c6d778106f06c096da`.

The immutable receipt borrow remains live across the owner operation. That
operation uses the original authority's owner mutex without requiring launch
admission to remain open: normal close therefore permits already-authorized
cleanup. Only success takes the store's same ManagedReservation. Every error
retains it, the actual DiskStore and the shared 512 KiB metadata reservation;
there is no release/reacquire or new cleanup grant. Repeated success takes an
already-empty Option, avoiding a second refund. Unclean Drop retains the existing
quarantine behavior. The completed retained store identity is not boot authority.

## Limits of this PASS

The canonical-feature test mutates17 projection/store fields and verifies the
binding predicate and unchanged real owner ledger. It deliberately does not
manufacture a sealed receipt. Consequently this review does not claim a real
positive sealed consumer, the installed resident's final call ordering, native
MVM endpoint retirement, final source release qualification, crash/power-loss
testing, or whole-platform support. Those remain explicit integration/execution
gates. No further product change is requested by this scoped source review.
