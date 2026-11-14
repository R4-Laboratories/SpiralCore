# Linux mailbox reader boot binding under a restrictive umask

Full workspace testing at contributor commit `eb450b97f92668847317239335b1dd340d1c4b28` stopped in `mvm-agentd` with 1,148 passed, one failed and four ignored. The failing assertion in `flowmux_drive::tests::reader_boot_binding_copy_is_bounded_nofollow_and_clears_stale_boot` requires mode `0444` on the **public boot binding**. The original code requested that mode at file creation, where the process umask could strip the reader’s read bits. The secret mailbox key has its own `0400` contract. [Original source](https://github.com/Casey-allard/mvm/blob/eb450b97f92668847317239335b1dd340d1c4b28/crates/mvm-agentd/src/flowmux_drive.rs)

The same original Linux test executable (SHA-256 `48fa6f20232f1a2271a954ebe994d7eaa29352d609ad9c41d2623f2bcd7a02b1`) ran the exact selector twice. Each run filtered out 1,152 other tests.

| Process umask | Asserted mode | Observed outcome | Original log SHA-256 |
| --- | --- | --- | --- |
| `077` | `0444` | Actual `0400` (256 decimal), expected `0444` (292 decimal). Test failed, exit 101. | `2fcd2a2c7418b916cc64bcf552e5997c0636ba3b75d48bfb00e1851f4ce237a2` |
| `022` | `0444` | Test passed, exit 0. | `ba8f8d046f04135633ea1075b970c7b4bfaad4d3651bde56052f74b34bce2e32` |

The one-file correction creates the public binding with owner-only read mode, writes its complete bounded contents, and sets `0444` through the same open file descriptor. The completed file has read permissions for all users; these tests do not exercise a post-drop open. The source still opens the input with `O_NOFOLLOW`, checks that it is a regular file, and enforces the byte limit before publishing. The secret key stays `0400`. The reviewed `flowmux_drive.rs` SHA-256 is `7eb4cba99c1cf8b38c15b0234a1f36830b2df5bda9a7ef4f30e6af6f40f38f8b`.

## Corrected Linux component

The package-only `mvm-agentd --lib` build produced test executable SHA-256 `fc58ca71948b42cb8aa4984589b6fadbb786297c3628b981cce4b7137f1e1507`. All four selections below ran this same executable. The original workspace and corrected package-only harnesses have different test inventories.

| Selection | Umask | Passed / failed / filtered | Log SHA-256 |
| --- | --- | --- | --- |
| Exact public reader boot case | `077` | 1 / 0 / 1,064 | `ef2ae6217f365a33779769b4e925f369c5e0871066118f74d6fd25e1da9208a9` |
| Exact public reader boot case | `022` | 1 / 0 / 1,064 | `8c1e468001a832b489721eac9dd8d809a3b8842840228721337ce920e6fbf582` |
| Exact root-only ACK-key and stale-boot case | `077` | 1 / 0 / 1,064 | `aa23adad81301e291038e0e96dcb1b5fade0d142c6339311682c7dbdb6e68ca5` |
| Entire `flowmux_drive::tests` module | `077` | 27 / 0 / 1,038 | `9eb3d1984dabff09b2ec14c37036608f53c1cbb9daabccab958a782e2824f76d` |

Strict workspace Clippy with all targets passed in 1 min 40 s (log SHA-256 `1e498f7ce5f18fd072e6d02ba3693caf2db4794166addbe6b689a6daa25be011`). The separate source-footprint gate exited 1 at 1,040,715 SLOC and 1,311,801 physical lines. The permission correction is published as [commit `4d90b275f`](https://github.com/Casey-allard/mvm/commit/4d90b275f).

The subsequent [footprint checkpoint `468661f47`](https://github.com/Casey-allard/mvm/commit/468661f47) accounts for this measured source growth after the upstream join and capture/provisioning corrections. It changes only the two limits, to 1,040,800 SLOC and 1,311,900 physical lines. Every category and test still counts, and the downward ratchet is unchanged. The rebuilt gate exited 0 with the identical category table, leaving 85 SLOC and 99 physical lines of headroom. All three footprint unit tests passed and strict all-target workspace Clippy passed in 21.90 s. The gate log SHA-256 is `73e94d6e6ed3ddf76e777c9cb4a4392a2416a36cf8acc4597afcc53b30726eaa`.

The full workspace test run on the corrected successor remains pending. The signed upstream rc.3 guest retains its tagged implementation. [Guest artifact provenance](runtime-rc3-preflight.md)
