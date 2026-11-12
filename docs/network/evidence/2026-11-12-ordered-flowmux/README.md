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
