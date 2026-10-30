# Independent recovery of member custody

This is an additive custody review of eleven retained archives. It does not repeat a transfer, change a scientific receipt, qualify an incomplete capture or alter runtime source `9dce4c57d1f41cef7365b297cd690849afce34e7`. Original empty manifests and incorrect receipts remain unchanged. Their per-member-match assertions are withdrawn; their authenticated ZIP size/digest evidence is separately revalidated.

`review.rs` uses Rust `zip` only to enumerate actual central-directory members and lengths. It does **not** use the historical member list as its iteration source. For each member, `/usr/bin/tar -xOf ARCHIVE -- MEMBER` uses macOS libarchive to decode the member. A Rust reader compares that output directly, chunk by chunk, against the existing remote file. Exact filesystem inventory, positive per-archive member count, per-member lengths, byte equality, extractor success and final total of 613 visits are mandatory. SHA-256 is accumulated over the compared bytes. Each child has a 30-second deadline; streams cannot exceed their declared member length. Archive/member size is capped at 256 MiB, per-archive decoded bytes at 1 GiB, entries at 64 and directory depth at eight. No archive member is extracted to disk.

The explicit eleven directories are compared with the supplied mapping. Ten archives must contain 56 members and the failed preflight archive must contain 53. Every original manifest must still be empty. The existing custody record selects an exactly matching retained artifact API record; actual ZIP bytes must match its size and SHA-256, frozen source and repository identity. These are local checks of previously authenticated API responses, not a new network authentication event.

The separate `compare-author.rs` reconstructs each manifest from the independently compared member records, requires every line to equal the author's corrected manifest, and compares all manifest digests, visit counts, archive lengths/digests and uncompressed-byte totals. Its input size assumptions are limited to the small generated JSON/manifests in this owned correction packet.

Five fixture tests ran before the real recovery: exact bytes succeed; changed same-length bytes, missing member, empty iteration and wrong expected count reject. The actual historical compressed shard-0 manifest was then checked with `--assert-old`; exit 1 reproduced the independent gate failure before the successful real recovery. The author's separate historical reproducer records the omitted-redirection false success. Final formatted checker tests also passed five of five. No compiler or positive-input setup failure occurred.

The final executable/source hashes, retained dependency hashes, all command outcomes and negative results are recorded in `qualification.json`. The independent checker compared **613 members / 425,385,845 bytes** across eleven archives. All eleven ZIP size/digest checks and all direct member byte comparisons passed. The author's corrected manifests agree for all 613 lines. An independent metadata census found 45 earlier manifests with 56 lines each and exactly the eleven affected empty manifests. Those earlier 45 archive contents were not rechecked by this run.

Commands, run from the local correction packet's `independent/` directory:

```sh
/Users/casey.allard/.cargo/bin/rustfmt --edition 2021 review.rs compare-author.rs

/Users/casey.allard/.cargo/bin/rustc --edition 2021 -D warnings --test review.rs \
  -L dependency=/Users/casey.allard/spiralcore/target/debug/deps \
  --extern zip=/Users/casey.allard/spiralcore/target/debug/deps/libzip-1ebc74230b0f80e1.rlib \
  --extern serde_json=/Users/casey.allard/spiralcore/target/debug/deps/libserde_json-f11651c4b5834b4e.rlib \
  --extern sha2=/Users/casey.allard/spiralcore/target/debug/deps/libsha2-11f7c61dcc994809.rlib \
  -o review-tests
./review-tests --test-threads=1

/Users/casey.allard/.cargo/bin/rustc --edition 2021 -D warnings -C opt-level=2 review.rs \
  -L dependency=/Users/casey.allard/spiralcore/target/debug/deps \
  --extern zip=/Users/casey.allard/spiralcore/target/debug/deps/libzip-1ebc74230b0f80e1.rlib \
  --extern serde_json=/Users/casey.allard/spiralcore/target/debug/deps/libserde_json-f11651c4b5834b4e.rlib \
  --extern sha2=/Users/casey.allard/spiralcore/target/debug/deps/libsha2-11f7c61dcc994809.rlib \
  -o review
./review --assert-old /Users/casey.allard/spiralcore/local/broad-wan-20261029/campaign/compressed_localized_32/shard-0/evidence-remote-files.sha256
# Expected and observed exit 1. This command was not treated as a successful custody check.
./review /Users/casey.allard/spiralcore/local/broad-wan-20261029 > independent-review.json

/Users/casey.allard/.cargo/bin/rustc --edition 2021 -D warnings -C opt-level=2 compare-author.rs \
  -L dependency=/Users/casey.allard/spiralcore/target/debug/deps \
  --extern serde_json=/Users/casey.allard/spiralcore/target/debug/deps/libserde_json-f11651c4b5834b4e.rlib \
  --extern sha2=/Users/casey.allard/spiralcore/target/debug/deps/libsha2-11f7c61dcc994809.rlib \
  -o compare-author
./compare-author /Users/casey.allard/spiralcore/local/broad-wan-20261029/qualification/artifact-member-custody-correction > author-crosscheck.json
```

Use a new output directory when reproducing; do not overwrite the preserved reports or binaries. The original reader, test executable and comparator remain in the local `qualification/artifact-member-custody-correction/independent/` directory. Only Rust source, JSON, method and checksum files enter the repository packet.
