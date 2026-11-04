# Current canonical source alignment

2026-11-04. The adapter and access manifests select MVM77752e1967d20caf8bad710de864a0ccd967f9e6 and the shared Transferca5d9ca. The MVM fork push recovered; this source combines the separately tested unused-preflight and helper fixes, and ports Hostd's shared-core dependency pins to currentca5. The earlier failed pushes and source bundle remain historical records. The new combined/default-current-core graph is awaiting exact-source build/runtime qualification; component tests do not transfer approval to it.

Layering remains shared core -> MVM -> adapter -> Live. The subsequent Live manifest must select this adapter commit and the identical MVM revision, then refresh its lock and verify one package/source identity for each critical crate. Native flags remain disabled while the full endpoint lifecycle/resource model is incomplete. No new MVM envelope, alternate authority or duplicated codec is introduced.
