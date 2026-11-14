# Current contributor-host Linux workspace qualification

MVM contributor `468661f47e01d0f59c6f1da4e35a968ceccb3ad2`, tree `ceea5a0d51e37394f256f365e7abb66ef66566cb`, completed the native Linux x86-64 workspace gate under recorded umask **022**. The host includes upstream `48fa5b488c4543f0f7988df722633554663a9764`. This is host qualification, separate from [current installed guest execution](native-rc3-fixture.md).

| Command, Rust 1.97.1 | Exit |
| --- | ---: |
| `cargo +1.97.1 test --locked --offline --workspace -- --test-threads=1` | 0 |
| `cargo +1.97.1 clippy --locked --offline --workspace --all-targets -- -D warnings` | 0 |
| `cargo +1.97.1 check --locked --offline --workspace --all-targets` | 0 |
| `cargo +1.97.1 check --locked --offline -p mvm-conformance --all-targets --features bdd` | 0 |
| Enclosing runner | 0 |

The raw test log contains 323 successful libtest summaries totaling 21,705 reported passes and 75 ignores. This is **not a distinct-test census**: nested subprocess output may repeat summaries. BDD was compiled, not executed. Native GNU all-targets compilation covers the recorded Linux target; it is not the literal Zig or `just check::gated` recipe. Clippy took 17.31 s, all-targets check 71 s and BDD compilation 43.66 s. Driver-presence expectations were explicit; no GPU workload ran.

Both source receipts matched 4,655 tracked entries / 64,659,163 bytes. Source manifest SHA-256: `48ff22ed15e5d3b89be61c8209f6260549450ffa91bf56d21798d2f66c9b1e86`. Raw workspace log: 1,962,138 bytes, SHA-256 `1c56e9fcca5b907d3021ae253fb0e38531d169979c3b6f7ab91108060b0c565e`. Retained private evidence directory: `mvm-468661-workspace-022.qjWlLBhG` on the Linux test host.

## Retained failures and scope

Earlier full-workspace execution under umask 077 exposed two artifact-construction defects: runtime-overlay VERSION file permissions and Stage 0 tar extraction executable permissions. The same executable failed under 077 and passed under 022. Neither expectation was weakened. Private home and temporary roots remain mode 0700; the successful suite does **not** establish global 077 support. [Upstream report #5480](https://github.com/tinylabscom/mvm/issues/5480)

The contributor also repairs the separate public mailbox boot-binding permission defect through the already-open descriptor; the root-only secret ACK key stays 0400. Its 077-specific controls remain distinct from the construction defects. [Permission controls](reader-boot-permissions.md)

The source-footprint gate initially failed at 1,040,715 SLOC / 1,311,801 physical lines across 2,953 files. The contributor explicitly raises its two ceilings to 1,040,800 / 1,311,900, leaving 85 / 99 lines of headroom. Categories, parser, tests and the 10,000-line downward ratchet remain intact. The original limit did not pass. The rebuilt gate and its three unit checks passed, and the category table was unchanged. This is a disclosed baseline change, not a source reduction.

The capture working-directory fix and its nested Cargo artifact controls passed before this gate. [Capture regression and correction](capture-project-directory.md) · [Contributor source](https://github.com/Casey-allard/mvm/tree/468661f47e01d0f59c6f1da4e35a968ceccb3ad2)

SpiralCore head `0941abea5e1122bd594477931288eda9d7c556c7` subsequently completed **38 successful hosted checks and four skipped checks**, including ordinary Windows Server 2022/2025 and Windows ARM64 portable component rows. Skipped jobs are not execution evidence. A successful current Windows row does not explain the historical 240-second failure at `c26b5d2c`; its attribution remains open. [Exact-head CI](https://github.com/R4-Laboratories/SpiralCore/pull/260/checks) · [Earlier Windows failure](reply-wait-attribution.md)

The complete MVM deliverable still requires the remaining native paired/sustained, platform, cross-host carrier, capacity and security acceptance. No WAN, customer-fleet or FIPS qualification follows from this workspace gate.
