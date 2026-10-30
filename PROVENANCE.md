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

## Owner-supplied additions

On 2026-10-26 the owner supplied 18 attachments and the SpiralCore v69 webpage for explicit public upload. `import-manifest.tsv` records the origin, byte count, and SHA-256 of every addition:

- `references/`: all 18 original attachments, including both original ZIP archives, PDFs, Python source, JSON, text, and the 8-byte continuity file.
- `references/spiralcore/`: four files extracted from `spiralcore.zip` (`spiralcore.py`, `sessions.json`, and two `.spmc` saves), with each extracted SHA-256 checked against the archive member. `.DS_Store`, `__MACOSX`, and `__pycache__` metadata were not imported.
- `index.html`: the self-contained page downloaded from `https://spiralcore.netlify.app/`, 1,026,033 bytes, SHA-256 `770142b4f343842cd0693d362d49f95272a5ded029d9682211a5215d64810fd8`. It has no external script, image, stylesheet, worker, or fetch dependency and can serve as a future GitHub Pages entry point from `main` / root.

Both ZIPs passed `unzip -t`. Two JSON files parsed successfully and four supplied Python files passed `py_compile`; no attached Python code was executed. `pypdf` is not installed, so no full structural PDF parse was claimed; PDF file signatures and exact attachment hashes were verified instead. The source webpage was observed loading in the built-in browser before its exact source was copied.

## History boundary

This is not a full Git-history migration. The available UOR-R4 clone is shallow, and the earlier standalone review repository recorded at `/Users/casey.allard/spiralcore-fbs-review` is not present locally. Current files and their source commit identity are preserved instead of inventing or incompletely rewriting history.

## Exclusions

The initial UOR-R4 snapshot excluded external reference originals, corpora, checkpoints, credentials, generated keys, caches, and binary artifacts. The later owner-supplied additions explicitly include PDFs, ZIPs, binary saves, and the webpage; these intentional additions are covered by `import-manifest.tsv`. Existing notes, hashes, review evidence, and private-store pointers remain preserved.

Remaining exclusions are unrelated corpora and checkpoints, `.git` internals, generated certificates/private keys, credentials, `.DS_Store`, `__MACOSX`, `__pycache__`, and unrelated build/cache output. Tracked text receipts, bounded logs, JSON results, historical patches, and the explicitly supplied binary files are included as research evidence. Their presence does not mean their historical GPU, WAN, or model runs were repeated during these imports.

## Links and build boundary

Copied research is byte-identical and retains its original links. Some relative links point to UOR-R4 material outside this selective export; the source repository is the full-context authority. Those links were not silently rewritten to disguise missing context.

The two networking crates under `projects/spiralcore/` and the separately imported `crates/uor-r4-object-store` are standalone build targets. Other files under `crates/` remain model-source references and integration context, not a complete independently buildable model.

## Visibility and licensing

The repository was created privately for initial import validation and made public at the owner's explicit request after the initial import merged and fresh-`main` checks passed. The owner also explicitly requested these additional public uploads. No new license is asserted by either import.

## Rust object-store import (2026-10-28)

The owner authorized importing the model-independent object-store core from UOR-R4 revision `b1b1ef6f0a5d59d8198207c198431701fff5fd63`. The source crate tree is `d1392e09e07200009a6b50b6f09067c2ef8a853b`. [The dedicated import manifest](docs/provenance/object-store-import.json) records each source blob, original SHA-256, extraction SHA-256, and adaptation. Extraction hashes describe the compatibility-import baseline, before separately reviewed hardening changes.

The five core modules, CLI, example, selected tests, immutable fixtures, and protocol documentation were extracted from Git objects. Model/tokenizer bindings and their tests were excluded. The package retains the UOR Foundation [MIT license](crates/uor-r4-object-store/LICENSE). This license applies to that imported implementation and does not relicense the repository's unrelated materials.

The original `source-manifest.tsv` and `import-manifest.tsv` remain historical records of their original exports. Subsequent changes are recorded in Git and the delivery record rather than silently changing those original identities.
