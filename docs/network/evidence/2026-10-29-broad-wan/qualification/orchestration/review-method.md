# Portable producer-pin integration checks

The native Rust tests extract the three marked blocks from the actual local run-shard.sh at runtime. They execute those exact shell blocks under /bin/bash3.2 with generated, owned temporary JSON fixtures and a mock gh executable that accepts only the two read-only producer-variable queries. They do not launch the full runner, access GitHub, create tokens or start network processes. The temporary fixture trees are removed by Rust Drop.

Behavioral RED was observed before adding checks: the original source accepted a wrong mocked producer pin and a failed reuse receipt, and lacked checkpoint copies. All three tests then passed after implementation. They cover29 shell executions:15 preflight cases, one exact checkpoint case and13 download-validation cases. Negative controls include noncanonical/zero/oversized IDs, missing or wrong remote pins, wrong source/qualification, invalid digest spelling, failed or mismatched reuse/proof/archive records, changed artifact identity/size and changed manifest bytes.

Run from /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore:

```sh
/Users/casey.allard/.cargo/bin/rustc --edition=2021 --test /Users/casey.allard/spiralcore/local/broad-wan-20261029/qualification/run-shard-integration/tests.rs -L dependency=crates/spiralcore-transfer/target/debug/deps --extern serde_json=crates/spiralcore-transfer/target/debug/deps/libserde_json-f11651c4b5834b4e.rlib -o /Users/casey.allard/spiralcore/local/broad-wan-20261029/qualification/run-shard-integration/tests
/Users/casey.allard/spiralcore/local/broad-wan-20261029/qualification/run-shard-integration/tests
/bin/bash -n /Users/casey.allard/spiralcore/local/broad-wan-20261029/run-shard.sh
```

Compiler: rustc1.97.1 (8bab26f4f68e0e26f0bb7960be334d5b520ea452), aarch64-apple-darwin, LLVM22.1.6. The serde_json rlib SHA256 is d1233394af34e44bbb105c560c78302ffa07d22cc5f35fd30b6413efde000eed. Script, test source and executable pins are in qualification.json. No prior historical artifact was modified. The rest of the runner was previously source-reviewed and is outside this implementation's three-block scope; this test is not an execution of an actual WAN campaign.
