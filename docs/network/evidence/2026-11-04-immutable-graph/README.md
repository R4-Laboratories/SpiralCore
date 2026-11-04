# Immutable development graph and canonical parent component

This is a development qualification checkpoint, not the full Ari/MVM release.
It removes local path overrides for the selected Live integration graph and
executes its seven original-parent tests against published immutable sources.
No guest, native backend, service/WAN transfer or performance trial runs here.

| Layer | Immutable source |
| --- | --- |
| Shared Transfer / FlowMux / gateway core | SpiralCore `ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1` |
| MVM Core / Contract / Hostd / Client / Runtime / backends | Fork `8b833aee8be5c1be9ce0d1197b6b98d4d03625f6` |
| Adapter | SpiralCore `656fead215d5d1ad1513e83fbf4dc7a654c1d52e` |
| Live | Checkpoint Cargo.toml and refreshed Cargo.lock alongside this receipt |

The directed graph is pure core→MVM→adapter→Live. Adapter explicitly depends
on the original core Git revision rather than its newer sibling-path identity;
MVM and Live use that same core. This avoids duplicate protocol/Rust type copies
without patching dependencies to local paths. The same Git URL spellings and
hardware-SHA feature closure are retained. Access's analogous manifest pin
is preserved, but this build is not an Access acceptance run.

The first metadata attempt failed on old locked Mio1.2.4 versus the FlowMux exact
1.2.1 requirement. Tokio1.53.3 requires Mio^1.2.0, so an explicit lock update to
1.2.1 satisfies both without downgrading Tokio. The first successful metadata
still contained old and new MVM Core/Contract identities via the old adapter.
That duplicate graph is retained and not accepted. The adapter pin layer repairs
it. Final metadata verifies ONE identity for each critical package and one
MVM revision across the entire selected package set. `single-graph.json` records
actual IDs; original metadata/build JSON are losslessly gzip archived. Metadata
success alone is not a compile or execution claim.

Actual command: Rust1.97.1 Cargo test --locked --release, features
live,linux-host,canonical-host, target installed_parent. One jobs1 compile lane,
no instrumentation or VM. Release build succeeded. Its emitted test executable
was copied outside Cargo, compared/hashed and made nonwritable BEFORE listing
and running; protected SHA256 is
`cca4274710eb38e4bb9f1a873b064742fa321b923655b5abc54042385efe1923`.
All seven listed tests executed and passed, zero failed/ignored/filtered.
The protected binary verifies unchanged after execution.

The tests establish same-pool reservation ownership, early malformed-shape
refusal, unique actual serials across providers, close fencing, real opaque
AdmittedPlan binding, actual Core anchor retention and conservative uncertain
Drop. They do not boot a VM, construct a production endpoint pool or establish
native child/service retirement, whole-process memory, sustained4096Calls,
confinement, cross-host WAN or final supported capacity.

The root source files equal the pure checkpoint except for the two explicitly
updated Live Cargo files. Dependencies are fetched from the named Git commits,
not another contributor's mutable directory. Subsequent MVM child-lease,
factory, helper-deadline and resource-model work is newer and must receive its
own final exact-source qualification. All full delivery gates stay open.
