# Independent review of Linux capture qualification attempt3

Reviewed actual run40943666213, sourcebc8c5722d634a34dddc643c19301ef00064eea84. This is an offline check of retained artifacts; it sends no traffic, modifies no original artifacts and does not repeat qualification.

The native Rust checker independently reads every retained PCAP record, validates Ethernet/IPv4/TCP header boundaries and fixed fixture endpoints, sums original IP lengths rather than96-byte snapshots, and reconstructs SYN ownership for the single measured row. It charges late packets to their connection and marks the ongoing out-of-row connection unattributed. It compares campaign, row and unattributed counts to all three reports; verifies the exact published65536-byte body lower bound in both directions; matches final counter text, PCAP/lifecycle hashes, case copies and producer identity receipts; and checks recorded offload state, server exit/reap, empty namespace process lists, owned deletions and final topology absence.

The checker is intentionally limited to this frozen little-endian nanosecond Ethernet/IPv4 fixture. Assertions reject any unexpected input rather than generalizing it. Each file is bounded to16MiB before and during reading. Arithmetic involving JSON-supplied values is checked where combined. The raw packet accumulator is additionally bounded by the16MiB file limit.

Reproduction from the worktree at /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore:

```sh
/Users/casey.allard/.cargo/bin/rustc --edition=2021 -C opt-level=2 /Users/casey.allard/spiralcore/local/broad-wan-20261029/qualification/attempt3-review/review.rs -L dependency=crates/spiralcore-transfer/target/debug/deps --extern serde_json=crates/spiralcore-transfer/target/debug/deps/libserde_json-f11651c4b5834b4e.rlib --extern sha2=crates/spiralcore-transfer/target/debug/deps/libsha2-11f7c61dcc994809.rlib -o /Users/casey.allard/spiralcore/local/broad-wan-20261029/qualification/attempt3-review/review
/Users/casey.allard/spiralcore/local/broad-wan-20261029/qualification/attempt3-review/review /Users/casey.allard/spiralcore/local/broad-wan-20261029/qualification/linux-attempt-3 /tmp/spiralcore-attempt3-independent-NEW.json
```

The output path must be absent. Actual compile and check both exited0. Compiler: rustc1.97.1, commit8bab26f4f68e0e26f0bb7960be334d5b520ea452, aarch64-apple-darwin, LLVM22.1.6. These are local reviewer tools, not the Linux measured executable.

Pins:

- Reviewer source54b0647ace140404593a668148edab7b303ed941e349b6296e52c974f270022b.
- Reviewer executable707aa89d199469727366b4ae1d9dd2296874d2bbd64593504d8b6988a0459dae.
- Independent JSON1fc9eb11569c245c4c4a98a8833141adb17566b3cf99d49371c485b290465824.
- serde_json rlibd1233394af34e44bbb105c560c78302ffa07d22cc5f35fd30b6413efde000eed.
- sha2 rlibf330242bbe618610a480be8d6467841cc2c6acc06ff71d93c7d278cf702c0f02.
- Producer qualification JSON31372d47cf561d49e3cddc4f5067f26a418963f05e8aef9d998e873ddf68adb7.
- Linux controller identity807e808eb10c7b9b462960abc34aa10444253a9507467d98ecb346e61a20478f matches binary-sha256.txt. The remote executable itself is not included in this artifact, so this is receipt binding, not a fresh binary hash computation.

A separate read-only git-show hash check confirmed that the pinned source's fixture/controller/parser files exactly match the final independent source-review hashes e5540d09..., fd77d75f... and de0b0c1a.... The normal case's full27-command transcript was inspected: no NAT, bridge, forwarding, sysctl or default-route mutation appears. The helper structurally checks all three transcripts and successful cleanup.

The full stderr prefix is not retained, so its full-stream digest cannot be independently reconstructed. Final counter suffixes and lifecycle/PCAP hashes are verified. This proves only the bounded synthetic Ethernet/plaintext qualification outcomes, not WAN throughput, TLS behavior, energy savings or historical-capture completeness. A later workflow source needs its own source-pinned producer qualification.
