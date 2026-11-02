# Owned scratch cleanup

After original manifest/source/binary verification, compact analysis reproduction,
independent arithmetic review and fresh immutable archive downloads, root moved
only these task-owned generated paths to Trash with `/usr/bin/trash`:

- `/Users/casey.allard/spiralcore/local/candidate-cpu-20261102` (159,388 KiB)
- `/private/tmp/spiralcore-hf-candidate-41399314329-5b144co6` (26,452 KiB)
- `/private/tmp/spiralcore-hf-local-profile-iqd8c943` (608 KiB)
- `/private/tmp/spiralcore-hf-quick-41399314515-zlooraca` (21,516 KiB)

The command guard rejected permanent `rm -rf` cleanup before execution; root
used the safer reversible operation instead. This is not a disk-reclamation
claim. No global Trash emptying, other-lab files, user runtime/saves, shared Cargo
target or HF cache cleanup occurred. Archive receipts describe verification-time
paths; this records their later relocation. The three full ZIPs remain verified
at their immutable private Hugging Face revisions, while compact evidence stays
in Git. The original ZIP hashes were rechecked immediately before relocation.
