# Frozen sparse encoder oracle

`delta_legacy_cc0199f.rs` and `protocol_legacy_cc0199f.rs` are byte-for-byte
snapshots from commit `cc0199ffd255b9e2e002afe8c96f10462a3e474f`:

| Original path | Git blob | SHA-256 |
|---|---|---|
| `src/delta.rs` | `6c849fa50fee40718950cbaf8ebb1eae327717d3` | `af31026c806c9046d62735b3ebdd03472e667103011a06b6f49ff76edb19e18e` |
| `src/protocol.rs` | `2185c21bc3ae62af812e3976f22a7eb4bd418835` | `65fa50275a1487b937040c0f03de3cdeba8e49ce563e78d0bcb3763006804e6a` |

Both original paths are relative to `crates/spiralcore-transfer`. No imports,
algorithms, public interfaces or decoder code were edited inside these files.
They compile only inside test/example consumers. The consumer supplies the
root `crate::` imports and the pinned `protocol` module so its private `Reader`
is available to the original sparse decoder. This avoids making the production
Reader public or maintaining a second serializer implementation.

Do not run formatting tools directly on these immutable snapshots. Regenerate
them from the pinned commit if intentional provenance work requires it; any
changed hash must be reviewed explicitly.
