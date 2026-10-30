# Archived candidate source

These are byte-for-byte evidence copies of the rejected SC-OPT-007 candidate at
`317c7f75b8b8355fbb2c74b4bad1944e4f0cb1e8`. They are not compiled in the restored
delivery tree. Original paths:

| Snapshot | Original path | SHA-256 |
|---|---|---|
| `delta.rs` | `crates/spiralcore-transfer/src/delta.rs` | `8d24c10c96de770d40d8c1ea24d5ae5427c95576d99936637c886ecf07d12735` |
| `pruning_tests.rs` | `crates/spiralcore-transfer/src/delta/pruning_tests.rs` | `6a34ba6391d14ed104a55c04bb2fdc6d0abc0e1c112bbccd1bf08392387b1e46` |
| `sparse_pruning.rs` | `crates/spiralcore-transfer/tests/sparse_pruning.rs` | `0132b4833de7dc76949c4c39e245de29d983c03735ac53971789f58ac05be35f` |

Use the full measured commit in an isolated checkout to rebuild or run its tests;
these flattened snapshots are not a standalone compilable crate. The immutable
oracle, harness, compiler/feature settings and executable hashes are retained in
the parent packet. Existing raw evidence files must never be overwritten by a
reproduction run.

The active encoder was restored to `cc0199ffd255b9e2e002afe8c96f10462a3e474f`
after the registered CPU gates failed. Running the current harness on that
restored tree compares baseline against baseline; a revision label is provenance
input, not code selection.
