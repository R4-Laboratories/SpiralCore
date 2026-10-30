# Independent portable-bundle verification

This review reads the existing outer tar without extracting it or modifying any original input. Its scope is packaging integrity and consistency with the frozen manifest, selected Git revisions and retained artifact custody. It is not a new performance measurement, packet replay, inner-ZIP member requalification or external GitHub authentication.

`independent-review.rs` implements a separate, narrow ustar reader in safe Rust. It validates header checksum, `ustar` format/version, regular-file type, empty link target, safe unique names, bounded declared sizes, exact payload SHA-256, zero member padding and the two terminal zero blocks. It reads all remaining archive bytes and rejects nonzero trailing data. The complete archive SHA-256 includes headers and padding. Expected-member input must contain exactly 68 unique entries; the successful parse must visit every one. It retains only the four small control files in memory. The 1.225 GB outer archive is never loaded as one buffer.

The control files are compared byte-for-byte with the frozen staging copies. `MEMBERS.txt` must list exactly all 68 names. `SHA256SUMS` must bind the other 67 entries and contain no duplicates or self-reference. The embedded inventory must identify twelve registered cases with five measured artifacts each, the separately labeled preflight failure and the separately labeled producer. All 62 archive digests, sizes and run/artifact identities must match their retained custody references read from the fixed records commit. Historical per-member claims are not reused as proof; their additive correction remains part of the record set.

For each inner Git archive, the checker obtains a fresh `git ls-tree -rlz` for the exact commit and explicit selected roots, comparing every path, mode, blob ID and byte count with the inventory. It then regenerates `git archive --format=tar --prefix=...` as a stream, hashes it without saving a duplicate, and requires byte-identical size and SHA-256 to the included inner archive. This binds all 5,563 compact-record files and 67 measured-source files to their recorded revisions. The report digest is checked against its Git blob as well.

Six small tests ran before the actual archive check: a correct ustar succeeds; corrupted payload, truncated archive, missing member, duplicate member and zero-work expected input reject. The actual archive check and tests are distinct. The author used bsdtar member extraction; the independent verification uses its own sequential parser. Both compare against the same frozen manifest and pinned source/custody definitions.

The local root is `/Users/casey.allard/spiralcore/local/broad-wan-20261029`. The measured revision is `9dce4c57d1f41cef7365b297cd690849afce34e7`; the report/evidence revision is `8e46471f8aacf74a931b0152ebacf656f7690376`. The expected manifest is `review-bundle-build/members.json`, SHA-256 `c74f6826b79cc38943c34267bb2f939cfa5ab1e8e2f1fdaa948c49e6d46011f2`. Results, compiler/dependency identities and actual outcomes are bound by `independent-qualification.json`.

Actual commands, from `review-bundle-build/`:

```sh
/Users/casey.allard/.cargo/bin/rustfmt --edition 2021 independent-review.rs
/Users/casey.allard/.cargo/bin/rustc --edition 2021 -D warnings --test independent-review.rs \
  -L dependency=/Users/casey.allard/spiralcore/target/debug/deps \
  --extern serde_json=/Users/casey.allard/spiralcore/target/debug/deps/libserde_json-f11651c4b5834b4e.rlib \
  --extern sha2=/Users/casey.allard/spiralcore/target/debug/deps/libsha2-11f7c61dcc994809.rlib \
  -o independent-review-tests
./independent-review-tests --test-threads=1

/Users/casey.allard/.cargo/bin/rustc --edition 2021 -D warnings -C opt-level=2 independent-review.rs \
  -L dependency=/Users/casey.allard/spiralcore/target/debug/deps \
  --extern serde_json=/Users/casey.allard/spiralcore/target/debug/deps/libserde_json-f11651c4b5834b4e.rlib \
  --extern sha2=/Users/casey.allard/spiralcore/target/debug/deps/libsha2-11f7c61dcc994809.rlib \
  -o independent-review
./independent-review /Users/casey.allard/spiralcore/local/broad-wan-20261029 \
  /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore \
  > independent-review.json
```

Existing debug-profile dependency rlibs were reused; the helper itself was compiled with optimization level 2. Verification duration is not a performance result. Reproduction on another host requires adapting paths, resolving the pinned `serde_json` and `sha2` dependencies, and retaining new compiler/executable identity. The source accepts root/repository arguments but expects the named frozen build manifest and staging control files beneath the root. Use a fresh output directory rather than replacing preserved receipts.

The Git consistency portion requires a local repository containing both pinned commits; the portable tar deliberately does not include the Git database. The four staging control files can be obtained from the outer archive when reproducing in a fresh directory. The expected 68-member manifest is included in the separate review packet.

The source, input member manifest, result, qualification, method and checksums are preserved outside the frozen tar in the repository review packet. Local executables remain alongside the original local verification files; no executable is added to Git. No receiver caches, full vendor/toolchain cache or live capability secrets are supplied by this verification. The original scientific limitations, including one incomplete measured capture and the separate failed setup, remain unchanged.
