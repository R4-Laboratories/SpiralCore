# Public evidence packaging and executable exclusions

The credential review in `artifact-review.md` covers the original archive data.
A separate redistribution-notice check found that none of the four historical
archives bundles LICENSE, NOTICE or COPYRIGHT files for its compiled programs.
They contain nine frozen executables, including a static ARM64 Linux guest and
native/runtime dependencies. Copying only current Cargo registry notices would
not establish the exact notice set for every historical binary and linked
component. No such completeness claim is made.

The public release therefore distributes **evidence/capture subsets**, omitting
only the nine frozen executable files. Every other original regular-file payload
is preserved byte-for-byte, including original binary hashes, source pins,
captures, fixture descriptions/checkpoints, measurement receipts, validation logs
and original manifests. The original full archives remain unchanged in the
existing private Hugging Face dataset. Their privacy setting is not changed.

| Public evidence package | Rebuild source revision | Omitted executable paths |
|---|---|---|
| `mvm-combined-public.tar.gz` | `9ceaeef5070c87fd17b8519a1e8630b7f7888fd6` | `frozen/host`, `frozen/guest` |
| `cpu-conscious-public.zip` | `30792f17d0a596c4344e4780ebd4b8f455e3bedb` | `frozen/object_fixtures`, `frozen/service`, `frozen/spiralcore-gateway-probe` |
| `tun-gateway-public.zip` | `199475fa8a9b54cd8ea87488b5dc120be2fc5279` | `frozen/spiralcore-gateway`, `frozen/spiralcore-gateway-probe` |
| `packet-selection-public.zip` | `9dd283ca366d6075bd6b423c3a42aa928e222ce2` | `frozen/spiralcore-gateway`, `frozen/spiralcore-gateway-probe` |

Exact lock identities demonstrate why using only the present working tree's
dependency versions would be insufficient: TUN and packet gateway lock SHA-256
is `a8675e01c5e43e5dbf58cd0ac6ee55301b527b3f6209319a8fcbb5d801c852aa`,
whereas the CPU-conscious gateway lock is
`f8f439678d8e477ff394474fb68e6b4f0bb1fbb151705470a43cab4886f94a67`.
The MVM live lock is
`98ff6e6909befa1c5338794926055101bc8b87cb050b5eaa5ebe7ae1275829f4`.
Those locks and the build instructions are available at the public source pins.

Each public package adds six `_PUBLIC_RELEASE` files: a scope/rebuild README,
the original archive/source mapping, the omitted executable SHA-256 list,
SpiralCore's MIT license, the original UOR object-store MIT notice, and the
original MVM Apache-2.0 license. Including these notices does not relicense
third-party dependencies or historical research, nor claim that they cover
omitted compiled runtimes. Public packages are inspectable without executing code.

`release-exclusions.json` binds each original archive digest to its new public
archive digest, exact source pin, omitted paths/hashes and retained-file counts.
Fresh extraction and `cmp` verify every retained original file against its source
archive extraction. Package hashes are external to avoid self-referential archive
digests. Original manifests remain historical records; their executable entries
are intentionally absent from the subset, and the exclusion manifest explains
that difference.
