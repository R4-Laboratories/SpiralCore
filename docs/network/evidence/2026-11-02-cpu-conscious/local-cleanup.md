# Owned local experiment cleanup

After full original manifest/source/binary verification, immutable HF upload,
forced fresh download with exact ZIP-byte comparison, compact extraction/hash
verification, byte-identical analyzer reproduction and independent raw-arithmetic
review, only this task's owned local artifact directories were moved to Trash:

- `/Users/casey.allard/spiralcore/local/cpu-conscious-20261102` (about 128 MiB,
  including the original ZIP, expansion and compact reproduction copies).
- `/private/tmp/spiralcore-hf-cpu-conscious-T8iHax` (fresh immutable ZIP download).
- `/tmp/spiralcore-cpu-conscious-pr.md` (temporary PR body).

The command was `/usr/bin/trash` on those exact paths. Trash was not emptied;
this is working-directory cleanup, not claimed disk-space reclamation. Shared
Cargo targets, other labs, user sessions and the preserved Python runtime were
untouched. Full captures/binaries remain in the verified private HF archive;
Git retains readable analysis and the compact original public receipt bundle.
