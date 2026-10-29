# Independent large-WAN evidence review

All compilation, tests and analysis below ran after the parent reported completion of all three WAN sends. No transfer, endpoint, network, or benchmark was rerun. The production repository was not edited.

The final receipt is `independent-review-with-ip-subset.json`. The earlier `independent-review.json` is retained as the successful initial analysis, before adding the expressly labeled descriptive subset. Both reject a complete 30-pair IP claim. `independent-pcap-review.json` is a separate streaming replay of every captured packet header and campaign total.

## Reproduction

Run from `/Users/casey.allard/spiralcore/local/large-wan-20261029`. The dependencies are the already built, preserved SC-OPT-004 Rust libraries; no dependency installation or production build is needed.

```sh
review_deps=/Users/casey.allard/spiralcore/local/cache-profile-20261029/target-diagnostic/aarch64-apple-darwin/release/deps
/Users/casey.allard/.cargo/bin/rustc --edition 2021 -O independent-review.rs --extern serde_json="$review_deps/libserde_json-1526d65a4d76f4d2.rlib" --extern sha2="$review_deps/libsha2-edcd0ba55ef603fc.rlib" -L dependency="$review_deps" -o independent-review-subset
/Users/casey.allard/.cargo/bin/rustc --edition 2021 -O --test independent-review.rs --extern serde_json="$review_deps/libserde_json-1526d65a4d76f4d2.rlib" --extern sha2="$review_deps/libsha2-edcd0ba55ef603fc.rlib" -L dependency="$review_deps" -o independent-review-subset-tests
/Users/casey.allard/.cargo/bin/rustc --edition 2021 -O independent-pcap-review.rs --extern serde_json="$review_deps/libserde_json-1526d65a4d76f4d2.rlib" --extern sha2="$review_deps/libsha2-edcd0ba55ef603fc.rlib" -L dependency="$review_deps" -o independent-pcap-review
./independent-review-subset-tests
./independent-review-subset /Users/casey.allard/spiralcore/local/large-wan-20261029 /Users/casey.allard/spiralcore/local/large-wan-20261029/aggregate.json > independent-review-with-ip-subset.json
./independent-pcap-review /Users/casey.allard/spiralcore/local/large-wan-20261029 > independent-pcap-review.json
```

All three final compilations exited zero. Four tests passed. Both analysis programs exited zero. There were no compilation or positive-input setup failures. The earlier three-test qualification also passed before the subset-specific test was added. Owned diagnostic build/run/test/negative text logs were removed after their commands and actual outcomes were preserved in this method and the structured qualification receipt. Scientific JSON, raw captures, source and executables remain. The earlier `independent-review` executable and `independent-review.json` are historical pre-subset artifacts; the final source is bound only to `independent-review-subset` and `independent-review-with-ip-subset.json` in the final qualification.

The compiler was `rustc 1.97.1 (8bab26f4f 2026-07-14)`, full commit `8bab26f4f68e0e26f0bb7960be334d5b520ea452`, native `aarch64-apple-darwin`, LLVM 22.1.6. The analysis uses `-O`; this is not a performance comparison. Library hashes are retained in `independent-qualification.json`.

## Independent checks

The receipt checker independently generates all thirty 16 MiB xorshift fixtures, their seed and changed ObjectIds, canonical seeds, task IDs, scopes and global six-permutation order. It checks all 180 operation receipts, the three producer source and executable manifests, platform, selected protocol mode, real base ownership, wrapper lengths, 41,490 frames in each direction, exact expected HTTP-body lengths, and checksum-bound reopen receipts for 3,019,898,880 bytes. It does not rerun the receiver's persisted-byte comparison.

The author aggregate is bound to its schema, campaign, source, row count and per-shard binary identities before metric comparison. The checker independently calculates all 45 shared distributions and 30 paired comparison records. The comparison covers the complete raw arrays, midpoint medians, nearest-rank p95, min/max/sums, per-pair ratios, geometric means and wins/ties/losses. Seed-plus-update costs are added within each trial before statistics. Integer arithmetic is checked; floating-point comparisons permit `1e-12 * max(abs(expected), 1)` for representation differences. Additional receiver-wall and directional-body summaries are retained.

The capture checks match task ID and phase against every canonical operation, verify bidirectional application-body coverage, account for setup plus owned rows plus unattributed packets, and check tcpdump counters, pinned-address evidence and disabled TSO/GSO/GRO/LRO evidence. The separate PCAP reader streams the entire captured files with a 256 MiB input bound, validates classic PCAP and Ethernet/IP/TCP headers, and derives actual IP total lengths and selected-flow directions. It compares every campaign total, packet count and capture endpoint timestamp. It does not independently reproduce TCP connection ownership; per-row ownership continues to depend on the reviewed observed-SYN parser.

## Required negative checks

The four Rust tests include altered aggregate schema, campaign ID, source and row count, checked counter overflow, known 30-trial statistics and separately counted 20-trial statistics.

A retained truncated PCAP fixture contains the first 32 bytes of shard 0's capture: the 24-byte file header followed by only eight bytes of the first 16-byte record header. The original capture is unchanged. The following actual invocation exited **101**, rejected with `UnexpectedEof: failed to fill whole buffer`, and produced no success JSON:

```sh
./independent-pcap-review /Users/casey.allard/spiralcore/local/large-wan-20261029/independent-negative-pcap
```

This is an expected negative result, not a failed measurement or helper setup problem.

## Capture limitation and descriptive subset

Shard 2 has 622,806 PCAP records and tcpdump-captured packets, but 622,809 received-by-filter packets. All three reported kernel-drop counters are zero. The three-packet difference remains unresolved; it is not interpreted as three proven lost transfer packets or harmless startup noise. The checker preserves `qualified_all_three_shards: false`, and omits combined thirty-pair IP distributions and reductions.

At the parent's explicit request, a separate `descriptive_qualified_ip_subset` contains all and only the two complete captures, shards 0 and 1, global trials 0–19. It retains the exact trial list and excluded-shard reason, and labels `registered_30_pair_claim: false`. Its twenty-pair distributions, paired ratios, reductions and seed-plus-update totals are descriptive supplementary evidence. The full application-body, CPU and elapsed analysis retains all thirty trials. No favorable timing or byte value was used to select the subset.

All IP evidence concerns only the runner-to-Cloudflare relay leg. It includes captured TCP/TLS/HTTP overhead, ACKs and retransmissions, and excludes the relay-to-receiver leg. It is not physical Ethernet wire accounting. Sender plus receiver protocol CPU is not energy; raw/zstd and sparse retain their documented optional-DAG-warming differences.
