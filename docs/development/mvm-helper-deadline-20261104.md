# MVM helper source checkpoint backup

GitHub rejected three attempts to publish commit `38d5bd531cdeb607d3511cd24be1bc3b035f538e` to the MVM fork with `commit_refs` / `Internal Server Error`. The prior public fork commit `8b833aee8be5c1be9ce0d1197b6b98d4d03625f6` remains its published baseline.

[The verified incremental Git bundle](mvm-helper-deadline-20261104.bundle) preserves that exact successor commit,23 hash-matched helper source files and49 compact receipt files. [Its manifest](mvm-helper-deadline-20261104.json) records the SHA-256 and required public base commit. It contains source/evidence only, no executable, credential, payload store or VM disk. Existing MVM/imported licenses continue to apply.

From an MVM clone with the public base commit present:

```sh
git bundle verify /absolute/path/mvm-helper-deadline-20261104.bundle
git fetch /absolute/path/mvm-helper-deadline-20261104.bundle refs/codex-checkpoints/spiralcore-helper-deadlines-20261104:refs/heads/codex/spiralcore-helper-deadlines-20261104
```

The successor's `docs/spiralcore-development/deadline-shape.md` records scoped actual tests, retained original failures and incomplete integration gates. A recoverable source backup does not make it an accepted release or an upstream submission.
