# Installed entry integration: focused security source review

Disposition: **PASS for the reviewed source wiring; native qualification and
final exact-source release review remain open.** Reviewed the uncommitted
integration at HEAD `af48f4d4af1d00939ad988f26560801089baf08f`, using the active
security-reviewer instructions. No build, test, VM launch or broad scan was run
by this review. The channel implementation was authored by this reviewer and is
excluded from this independent verdict; it requires the lead's separate review.

The reviewed installed entry consumes the controller's one-session claim and
uses its launch context for Sender, Receiver and Verify, including Stage One.
An installation-bound budget cannot enter the legacy budget entry. Each real
spawn is adopted into the exact owned-Child ledger before challenge processing;
fresh original-epoch facts are queried before Binding and Go without holding a
worker/accounting mutex across control or stdin I/O. The fixed inherited endpoint
does not transfer owner EX. Role, slot and serial derive from the grant, not CLI
strings or a reconstructed PID. Early startup failures without a returned Worker
are still drained through controller custody.

Normal slot reuse follows positive child reaping, a successful private worker
receipt, matching installed generation/role and the trusted native backend exit
contract. Unknown/error outcomes close admission and retain ledger custody.
Installed cleanup never enters the legacy PID-based cleanup path. Final Clean
still requires zero transport usage, no owned/unreaped worker and canonical
holder EX; no success receipt substitutes for that kernel proof. Linux exports
the canonical typed SH handoff into the reviewed Firecracker hook. macOS keeps
context plus handoff live across synchronous HVF boot and backing-file sync.

Explicit CLI init/reconcile/session-installed commands reuse canonical owner
operations. They accept a nonsecret installation ID and the fixed native profile;
worker-installed has no missing-channel fallback. Existing legacy commands remain
separate: this does not establish universal machine-wide boot enforcement.
Stage One plaintext buffers remain explicitly outside transport accounting.
Actual FC/HVF boot, owner death with a live VMM, orphan reconciliation, exact
release CI and full B6a/B6b remain unqualified by this source-only pass.

Architecture decision: an additive public `InstallationBinding` and public fresh
`active_binding(expected: Epoch)` are appropriate foundation-library metadata.
The caller already supplied root/key and received the opaque epoch from begin;
the query runs canonical `current()` and requires the original Active key and
generation. Returned owned path/IDs/generation are a snapshot, not a credential,
worker grant, owner/Arc/Mutex or lock descriptor. Document that distinction and
preserve all freshness/fence checks; do not add owner reconstruction or boot
authority. This is a legitimate metadata API, rather than a lint-only visibility
exception. Its eventual visibility delta requires exact-source confirmation.

SHA-256 source pins (relative to crates/spiralcore-mvm-live):

| File | SHA-256 |
|---|---|
| src/orchestrator.rs | `3281f09f05687e3936d114c637c9103aac530f39d8bd3cc93683e3960c31364d` |
| src/host.rs | `c53f8a66c3fa9ef80d98371a63430ced4f00080b9e2eb678356248be00d1f883` |
| src/linux_host.rs | `67bbace5f601eb31fd56f4989c99d315f7953fe1f1a00f68c12e1253f95a7df6` |
| src/main.rs | `d1b8982be1d12d5a973095d9a2fc2b1945bb33088cd7986609987aa899d81bac` |
| src/installed_supervisor.rs | `0c5cec1cabbd79d3b31eec86c2d4b37d1125dbd31606f6e3c89f432593f13bb7` |
| src/installed_owner.rs | `4effaf03f9dbad95bb3413ec1607607848fce11618619b32c4be64cd9b766545` |
| src/lib.rs | `93fc2c91aaf817d86195885dde4dda1d4bba7cd2c39df89a7578032f4c4e85f5` |
| Cargo.toml | `90350ae9576cf79c8084f9024d75a7117fc6c54f6712c44b1c6947eac5a10baf` |
