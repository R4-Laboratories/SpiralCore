# Artifact storage and local disk use

Keep source, measurement contracts, compact raw observations, summaries and reviews in GitHub.
The SC-OPT-010 paired JSONL is 863,721 bytes and fits ordinary Git comfortably. Large captures,
receiver-store snapshots and portable review archives belong in external artifact storage.
GitHub blocks individual ordinary Git files above 100 MiB; see its
[large-file guidance](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github).

The owner authorized Hugging Face storage on 2026-10-30. The private
[caseyallard/spiralcore-experiment-artifacts dataset](https://huggingface.co/datasets/caseyallard/spiralcore-experiment-artifacts)
holds the first snapshot. The account is a free user; the published policy on that date includes
100 GB of private storage. No Space, compute job, paid storage add-on or subscription was created.
See [Hugging Face storage limits](https://huggingface.co/docs/hub/storage-limits).

## October 29 WAN snapshot

| Field | Value |
|---|---|
| Repository revision | `e9778965ab4d93714d88eab9a91ad363076cccc4` |
| Archive | `2026-10-29/wan-campaigns-20261029.tar.zst` |
| Compressed bytes | 7,747,629,705 |
| Archive SHA-256 | `f45fa904605a99197992b6ea98ecafa1cd85d72040ae3863a4f6761b3659ac86` |
| Original regular files | 24,677 |
| File inventory | `2026-10-29/source-files.sha256` |
| Inventory SHA-256 | `ecac8fd63ec737547fdbbe648c8ec7669fa444b57a790da523e054a62973ffa7` |
| Original roots under SpiralCore `local/` | `broad-wan-20261029`, `large-wan-20261029` |

The archive preserves both complete historical directories, including frozen executables,
captures, receiver stores, original review bundles and old build output. This first move favors
complete preservation. It is not a requirement to retain regenerable build output in future
archives. Original measurement records and their source hashes remain unchanged; old absolute
paths refer to the former local layout. The [migration receipt](artifact-storage-2026-10-30.json)
records remote verification and cleanup separately from the performance experiment. All
SC-OPT-010 timing finished before archiving or uploading these older directories.

Large artifacts remain private because they include raw network metadata. Authorized reviewers
need access to this dataset; public GitHub results do not depend on downloading it. The repository
revision above is immutable and should be used instead of a moving `main` link.

To restore into a chosen empty working location, using an authenticated Hugging Face CLI:

```sh
hf download caseyallard/spiralcore-experiment-artifacts \
  2026-10-29/wan-campaigns-20261029.tar.zst \
  2026-10-29/source-files.sha256 2026-10-29/archive.sha256 \
  --repo-type dataset --revision e9778965ab4d93714d88eab9a91ad363076cccc4 \
  --local-dir RESTORE
cd RESTORE/2026-10-29
shasum -a 256 -c archive.sha256
tar --zstd -xf wan-campaigns-20261029.tar.zst
shasum -a 256 -c source-files.sha256
```

Restore the two directories under the checkout's `local/` only when reproducing commands that
require their original paths. Do not overwrite current work. Extraction does not run stored code.

## Retention for subsequent runs

- Commit compact raw results and failure receipts with source, executable and fixture identities.
- Delete successful temporary worker stores after exact reopen verification. Current sender
  benchmarks already do this for each worker rather than accumulating the full campaign.
- Preserve only large artifacts needed for reproduction or review. Upload them with an inventory,
  immutable revision, byte count and digest; verify the remote copy before deleting the local one.
- Keep restoration instructions and the remote verification receipt in GitHub. Avoid automatically
  downloading archived data into every checkout.
- Reuse bounded build targets, disable incremental compilation when disk is constrained, and
  discard regenerable build caches once needed executables and source pins are retained.
- Original user runtimes, saved sessions and unrelated projects are outside this cleanup scope.

## November 1 paired gateway evidence

The exact GitHub Actions archive for run41361459962 is preserved at immutable
Hugging Face revision `8074002b3803378e5b655a284b0c2fff44338fc2`, file
`2026-11-01/gateway-41361459962.zip` (21,851,334 bytes). It contains the complete
Linux paired-gateway receipts, header-only captures and frozen executables.
A fresh remote download matched SHA-256
`59238980889273cb9c069f396d8dd273229e81f12e218db5e9b0a1056e49e851`.
See the [storage receipt](evidence/2026-11-01-tun-gateway/artifact-storage.json) and
[restoration instructions](gateway-results-2026-11-01.md#recorded-evidence-and-restoration).
Compact raw observations and all negative findings remain in GitHub.
