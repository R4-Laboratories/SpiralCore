# Public evidence downloads

The [GitHub evidence release](https://github.com/R4-Laboratories/SpiralCore/releases/tag/evidence-2026-11-02)
provides the current results without a Hugging Face account. Source code, compact
raw observations, analysis and reviews are already in this public repository.
The release adds the larger receipts/captures needed for independent inspection.
All six release assets were downloaded anonymously and verified against their
recorded sizes and hashes. [Publication verification](evidence/2026-11-02-public-readme/publication.md).

These are **experimental evidence packages, not installers**. Each package
preserves the original bytes of included entries and records its original archive
identity. Nine archived executables are excluded across the four packages because
their complete third-party redistribution notices were not bundled. Original
binary hashes remain in receipts; rebuild from the exact public source pins.
The original complete archives remain preserved privately, without rewriting them.

## Choose a track

| Track | Public download | Measured SpiralCore source | Report |
|---|---|---|---|
| Current MVM Combined: actual guests, encrypted mailbox/vsock, reboot readback | [mvm-combined-public.tar.gz](https://github.com/R4-Laboratories/SpiralCore/releases/download/evidence-2026-11-02/mvm-combined-public.tar.gz) | `9ceaeef5070c87fd17b8519a1e8630b7f7888fd6` | [MVM results](mvm-combined-results-2026-11-02.md) |
| Current Combined object gateway: local and shaped QUIC, CPU-conscious comparison | [cpu-conscious-public.zip](https://github.com/R4-Laboratories/SpiralCore/releases/download/evidence-2026-11-02/cpu-conscious-public.zip) | `30792f17d0a596c4344e4780ebd4b8f455e3bedb` | [Object results](cpu-conscious-results-2026-11-02.md) |
| Normal traffic: fast/adaptive packet selection, calibrated constrained link | [packet-selection-public.zip](https://github.com/R4-Laboratories/SpiralCore/releases/download/evidence-2026-11-02/packet-selection-public.zip) | `9dd283ca366d6075bd6b423c3a42aa928e222ce2` | [Packet results](packet-selection-results-2026-11-01.md) |
| Earlier paired IPv4/IPv6 TUN gateway: exact TCP/UDP flows | [tun-gateway-public.zip](https://github.com/R4-Laboratories/SpiralCore/releases/download/evidence-2026-11-02/tun-gateway-public.zip) | `199475fa8a9b54cd8ea87488b5dc120be2fc5279` | [Initial TUN results](gateway-results-2026-11-01.md) |

Download [SHA256SUMS](https://github.com/R4-Laboratories/SpiralCore/releases/download/evidence-2026-11-02/SHA256SUMS)
beside the packages. Original and public archive sizes/hashes, retained-entry
verification and exact excluded paths are recorded in the
[publication manifest](evidence/2026-11-02-public-readme/release-exclusions.json).
The release tag identifies the publication; each experiment's measured revision
is listed independently above.

## Verify and inspect

From a new download directory, download the checksum file and the packages you
want. Check only the downloaded entries; downloading all four allows:

```sh
shasum -a 256 -c SHA256SUMS
unzip -l cpu-conscious-public.zip
tar -tzf mvm-combined-public.tar.gz
```

Extract into fresh directories without overwriting your checkout. Included
public-subset manifests distinguish retained entries from the archived binaries
that were deliberately omitted. Original full-archive manifests are preserved
as historical records and may reference those omitted files; they are not
complete-subset checks. Read the per-package publication note first.

The packet captures use 96-byte snapshots from isolated test underlays: packet
metadata and potentially short encrypted QUIC prefixes. Public certificates are
test certificates, not private keys. No private-key, credential-bearing initramfs,
receiver-store or VM-disk files are included. [Publication review](evidence/2026-11-02-public-readme/artifact-review.md).

All original numerical observations, failures and source identities remain
unchanged. None of these public-packaging operations reran a performance experiment.

## Historical WAN and Hugging Face preservation

The earlier [real Internet WAN report](broad-wan-results-2026-10-29.md) and its
[compact observations](evidence/2026-10-29-broad-wan/) remain public in Git.
Its 7,747,629,705-byte original preservation archive contains raw network metadata,
stores and old build output; it was not part of this four-package review and has
not been republished wholesale. [Historical restoration and custody](artifact-storage.md).

The existing [Hugging Face dataset](https://huggingface.co/datasets/caseyallard/spiralcore-experiment-artifacts)
remains private. These are immutable archival references, not public download links:

| Original complete archive | Hugging Face revision |
|---|---|
| `2026-11-02/mvm-combined-9ceaeef.tar.gz` | `33ca33bb6e9db26e73fafff02924e9e8ee8cea75` |
| `2026-11-02/cpu-conscious-41465450384.zip` | `ba462609bc705928c91260090a8cc30cef377743` |
| `2026-11-01/packet-selection-41364240626.zip` | `4ee46e37e094cc0cdc34e4f3d256c5121bee8986` |
| `2026-11-01/gateway-41361459962.zip` | `8074002b3803378e5b655a284b0c2fff44338fc2` |
| `2026-10-29/wan-campaigns-20261029.tar.zst` | `e9778965ab4d93714d88eab9a91ad363076cccc4` |

The root [MIT grant and exclusions](../../LICENSING.md) apply to original
SpiralCore contributions. Public distribution does not relicense imported code,
research, third-party components or their notices.
