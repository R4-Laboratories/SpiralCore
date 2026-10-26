# Provenance

## Source identity

- Source repository: [`UOR-Foundation/uor-r4`](https://github.com/UOR-Foundation/uor-r4)
- Source commit: `b1b1ef6f0a5d59d8198207c198431701fff5fd63`
- Export time: `2026-10-26T03:52:45Z`
- Target repository: [`R4-Laboratories/SpiralCore`](https://github.com/R4-Laboratories/SpiralCore), public with default branch `main`
- Export shape: curated current-state snapshot; 344 copied files plus this import's root metadata

The source commit was checked against both a fresh detached worktree and GitHub's `main` immediately before export. No UOR-R4 source file, branch, issue, or research record was modified or deleted by the export.

## Manifest and byte identity

`source-manifest.tsv` has one row per copied file:

```text
source-relative-path<TAB>source-git-blob-object-id
```

Paths are preserved in the target. The manifest was sorted and deduplicated, every source blob was compared with the copied file, and the target was checked for missing and unexpected copies. Root `README.md`, this file, `.gitignore`, and the manifest itself are import metadata rather than copied source evidence.

## Selection

The export contains:

1. All 119 tracked files under `projects/spiralcore/`: 82 wire files, 35 warm-cache files, the project guide, and v68 reference notes.
2. Research records selected from `archive/catalog.jsonl` where the line is `spiralcore-wire`, the mechanisms include `spiralcore-fbs`, or the record path names SpiralCore. Both live and archived record directories are retained when present.
3. Directly related current and archived records for warm cache, full-rung FBS, secant descent, UOROBJ01 conformance, exact arithmetic, K16, pilot diagnosis, and octonion/XOR reviews.
4. SpiralCore/FBS task, pre-registration, roadmap, notebook, mechanism, geometry, figure, and audit documents.
5. Seven archived SpiralCore GitHub discussion snapshots: 2087, 2264, 2279, 2379, 2478, 2480, and 2482.
6. The branch-archive index and 12 SpiralCore-related historical source patches. These patches are inactive evidence, not code to execute.
7. SpiralCore-specific model sources and the current float/FBS implementation and integration points, including the completed full-rung runner and arm kernels.

The selection is deliberately not a repository-wide keyword copy; unrelated UOR-R4 model, harness, compute, and governance files are outside the export.

## History boundary

This is not a full Git-history migration. The available UOR-R4 clone is shallow, and the earlier standalone review repository recorded at `/Users/casey.allard/spiralcore-fbs-review` is not present locally. Current files and their source commit identity are preserved instead of inventing or incompletely rewriting history.

## Exclusions

Not copied: `.git` internals, caches, build outputs, `.DS_Store`, corpora, model checkpoints, generated certificates or private keys, credentials, and unreviewed external binary artifacts. External reference originals—including PDF/ZIP packages and large HTML sources—were not fetched from Downloads or Hugging Face for republication. Existing notes, hashes, review evidence, and private-store pointers remain preserved.

Tracked text receipts, bounded logs, JSON results, and historical patches are included as research evidence. Their presence does not mean their historical GPU, WAN, or model runs were repeated during this export.

## Links and build boundary

Copied research is byte-identical and retains its original links. Some relative links point to UOR-R4 material outside this selective export; the source repository is the full-context authority. Those links were not silently rewritten to disguise missing context.

The two networking crates under `projects/spiralcore/` are the standalone build targets. Files under `crates/` are model-source references and integration context, not a complete independently buildable model.

## Visibility and licensing

The repository was created privately for import validation. The owner explicitly requested public visibility on 2026-10-26; publication is performed after the import merges and fresh-`main` checks pass. No new license is asserted by this import.
