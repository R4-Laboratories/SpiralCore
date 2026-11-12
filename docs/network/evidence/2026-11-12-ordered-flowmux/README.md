# Ordered FlowMux streams on native operating systems

Two TLS peers exchanged 4,096 ordered objects of 16 KiB in each direction. Each reader compared every block with its original bytes and required EOF at the expected boundary. Counters on opposite ends agreed on both streams. Both peers ran in one native test process on one host for each role. This measures the Small profile, including endpoint compute and cache retirement, on the deterministic workload in the [Rust test](https://github.com/R4-Laboratories/SpiralCore/blob/3da7ee0ca77e9fe426eb79bdb099d7a2bce9cf29/crates/spiralcore-mvm-flowmux/tests/ordered_stream.rs).

The ordinary Windows Server 2022 and 2025 runs failed. On 2022 the 240-second limit expired after 3,598 exact blocks each way. On 2025 a reader timed out at about 2,980 blocks. The instrumented 2025 run passed on the same source. [Windows 2022 job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941/job/131283432869) · [Windows 2025 job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941/job/131283432861) · [Instrumented Windows run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161919/job/131282942384)

## Independent Apple Silicon run

macOS 27.0 build 26A5378n, arm64, Rust 1.97.1, source `3da7ee0ca77e9fe426eb79bdb099d7a2bce9cf29`. The release timing test completed on its first run. The library suite passed 29/29 in 2.32 s.

| Measurement | Result |
|---|---:|
| Exact application bytes, each way | 67,108,864 B |
| TLS bytes A → B / B → A | 5,395,050 / 5,395,015 B |
| Setup wall / process CPU | 42.245 / 8.278 ms |
| Full lifecycle wall / process CPU | 91.381160 / 43.604576 s |
| Retained publications / evictions per receiver | 64 / 4,032 |
| Final recorded cache bytes per receiver | 1,051,712 B |

The two TLS directions together carried 10,790,065 bytes for 134,217,728 application bytes. This is a byte count against the unframed application size. A matched raw or zstd timing comparison, NIC packet overhead, send-only time, and WAN latency require separate measurements. Process CPU covers both local peers. The fixture did not report peak RSS on this Mac. [Original measurement rows](local-results.txt)

Run from the repository root with the source revision above checked out.

```sh
cargo +1.97.1 test --locked --release --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --features witness-timing --test ordered_stream -- --show-output
```

## Hosted native FlowMux results

GitHub tested merge commit `9382e5654ba888211d19644f2c8aaadda9ae0018` (tree `371c13ee78d3f8a66f6722a1ca23af1d46810b9a`), comprising `99daea18` and `3da7ee0ca77e9fe426eb79bdb099d7a2bce9cf29`. The PR head has the identical tree. Each row used a pair of mutually authenticated TLS endpoints in one native process on the listed runner. Each successful row sent and recovered 4,096 distinct 16 KiB blocks per direction (64 MiB each). Each block contained its big-endian index and repeated `0x61` bytes. This input is highly compressible.

| Runner | Endpoint build | Setup (s) | Lifecycle wall (s) | Test process CPU (s) |
|---|---|---:|---:|---:|
| [macos26-arm64](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941/job/131283432858) | ordinary | 0.064 | 48.032 | 34.656 |
| [macos26-intel](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941/job/131283432790) | ordinary | 0.034 | 75.528 | 44.826 |
| [ubuntu24-arm64](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941/job/131283432698) | ordinary | 0.009 | 10.926 | 12.847 |
| [ubuntu24-x64](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941/job/131283432883) | ordinary | 0.027 | 8.320 | 10.645 |
| [windows11-arm64](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941/job/131283432757) | ordinary | 0.122 | 187.063 | 153.766 |
| [macos26-intel](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161919/job/131282942954) | timing enabled | 0.148 | 96.020 | 58.305 |
| [windows2025-x64](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161919/job/131282942384) | timing enabled | 0.053 | 162.230 | 128.250 |

For every successful row, A sent **5,395,050 TLS bytes** and received **5,395,015**. B independently recorded the inverse counters. Each receiver retained 64 verified publications (1,051,712 bytes), one tracked attempt and zero pinned entries after 4,032 evictions. Lifecycle wall and process CPU include setup, all byte comparisons and store cleanup. CPU is cumulative over both peers and their worker threads. Each row is a single run. Send-only time, physical energy and a raw or equally tuned zstd baseline were not measured.

The successful ordinary platform jobs also passed the 20 ordered-cache/Small/schema regressions, their endpoint tests, metadata guards and strict Clippy. Failed jobs stopped before their later checks. The portable storage workflow ran the three native Windows storage primitives on Server 2022 x64, Server 2025 x64 and Windows 11 ARM64. The Windows coverage figures describe instrumented correctness, not performance. [FlowMux matrix](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941) · [Portable storage and quota jobs](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161843) · [Timing jobs](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161919) · [Linux MVM host contracts](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247162336)

### Unfinished transfers on the same source

| Runner | Last acknowledged A/B blocks | Last read A/B blocks | Elapsed to snapshot | Process CPU to snapshot | TLS A sent / received | TLS B sent / received |
|---|---:|---:|---:|---:|---:|---:|
| [windows2022-x64](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941/job/131283432869) | 3,598 / 3,598 | 3,598 / 3,598 | 242.006 s | 107.922 s | 4,739,044 / 4,739,009 | 4,739,009 / 4,739,044 |
| [windows2025-x64](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43247161941/job/131283432861) | 2,981 / 2,980 | 2,980 / 2,981 | 229.700 s | 88.594 s | 3,926,702 / 3,926,630 | 3,926,836 / 3,926,702 |

The ordinary Server 2022 run exhausted the 240-second whole-transfer gate. Server 2025 reported a per-operation read timeout before the whole-transfer gate, while the separately instrumented Server 2025 run completed. Failure snapshots can include in-flight TLS bytes. Neither failed row completed the workload. The roots of these timing differences remain unresolved. The current candidate stays unqualified for the full Windows capacity gate.

- windows11-arm64: 216/228 production quota lines. NA: no production branch mappings.
- windows2022-x64: 216/228 production quota lines. NA: no production branch mappings.
- windows2025-x64: 216/228 production quota lines. NA: no production branch mappings.

All 16 hosted diagnostic ZIPs matched GitHub’s SHA-256 digest and every listed inner file (471 entries). Each ZIP recorded the actual merge SHA. The instrumented Windows gate previously timed out on the historical merge, then passed on an unchanged rerun. Both original outcomes remain available. [Historical failure and repeat](https://github.com/R4-Laboratories/SpiralCore/issues/44#issuecomment-6827611515)

| Hosted archive | SHA-256 | Verified entries |
|---|---|---:|
| `flowmux-component-macos26-arm64-1` | `2d197a14ed2db02cbe333f3c90a7af040ee980ca177b5f5a68efeb9420e8e481` | 13 |
| `flowmux-component-macos26-intel-1` | `7047484dc5c140b2caecd0008f066d8258a48f2c7bc74dbce4ad3b98d7db4e79` | 13 |
| `flowmux-component-ubuntu24-arm64-1` | `2fc88e282addb127af29743001513e6a5c083b646cb989ccd65ba2310c26b75f` | 13 |
| `flowmux-component-ubuntu24-x64-1` | `22aa1a24df1f94bc2a2b3984cad6e92941fab1a023f46d2d575a1ec52456b5b1` | 13 |
| `flowmux-component-windows11-arm64-1` | `d707d4bbdbdc862c058142541fa6ac82b337a7f65388ea5b1105b9bd60236af7` | 18 |
| `flowmux-component-windows2022-x64-1` | `d02f6960001dacdb3ee3ba33ef43a39e34e026696028f341b6964653ee885d38` | 13 |
| `flowmux-component-windows2025-x64-1` | `fe3cde20094330eefbcd24415debcab82238b313fd5dfbf1a998a418fa663f7a` | 13 |
| `flowmux-macos26-intel-timing-1` | `373955b490cad6a20eafb2fcf9a6dd2fdd1c7b29a48346581dc999be8bb3ded7` | 3 |
| `flowmux-windows-timing-1` | `ae7095ff84513a667df1c628746849171446c827db56b12ca6811b70aa257579` | 3 |
| `mvm-mailbox-portable-macos26-arm64-1` | `479eb8990796e28af940948946a6d48db55fc41d5733af8367d710902d773f7f` | 44 |
| `mvm-mailbox-portable-macos26-intel-1` | `5e9356b9d14a98e47f430fa8bb9f89bf7f52c43953e11024e9e126b593c65e1c` | 44 |
| `mvm-mailbox-portable-ubuntu24-arm64-1` | `07342c30ad3194782548fc6f7ae04726e1752bf3d8b3cec8f47b11b1b9f1b755` | 44 |
| `mvm-mailbox-portable-ubuntu24-x64-1` | `bc053cd240db6f2140dc4598054d0ea73980016611e7de51538030325ba457f6` | 44 |
| `mvm-mailbox-portable-windows11-arm64-1` | `81f3c0500dd84607e110d50d682515f87c5131a5c0358baf1ef17ec8f881fd45` | 65 |
| `mvm-mailbox-portable-windows2022-x64-1` | `72a101736dcfa3ec1183da811ad35d155a44ffadf8c019a1833e0a2c7ae466ee` | 64 |
| `mvm-mailbox-portable-windows2025-x64-1` | `f3a8c9d9f449ef70bb964b61c9d312bc7710b98c6c004acf33e5e16ce3eb6ccc` | 64 |
