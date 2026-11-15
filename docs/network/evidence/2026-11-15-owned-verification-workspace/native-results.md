# Native component observations

Every row verifies 67,108,864 application bytes in each direction and actual TLS 5,395,050 sent / 5,395,015 received at endpoint A, with reciprocal peer counters. All 87 FlowMux and 20 ordered regressions pass under unchanged guards. These are native cooperating host-process fixtures, not native MVM-backend, WAN or supported-capacity qualification. CPU is whole-process lifecycle CPU, not separate sender/receiver or energy.

| Native row | Setup (ms) | Lifecycle wall (s) | Lifecycle CPU (s) |
| --- | ---: | ---: | ---: |
| windows11-arm64 | 132.928 | 129.509555 | 109.437500 |
| macos26-arm64 | 66.548 | 23.810605 | 23.708574 |
| ubuntu24-x64 | 6.856 | 4.083319 | 6.767320 |
| windows2022-x64 | 90.295 | 39.204089 | 67.187500 |
| windows2025-x64 | 64.827 | 35.283660 | 64.437500 |
| macos26-intel | 38.257 | 53.050308 | 38.035028 |
| ubuntu24-arm64 | 8.874 | 7.616249 | 11.611045 |

The filesystem-only verification observations are separate:

| Native row | Largest new request (bytes) | Cumulative requests (bytes) | Window wall (ms) | Window process CPU (ms) |
| --- | ---: | ---: | ---: | ---: |
| macos26-arm64 | 250 | 112532 | 174.715875 | 279.206000 |
| ubuntu24-x64 | 250 | 110370 | 27.348249 | 42.817288 |
| macos26-intel | 250 | 112532 | 125.279600 | 248.766000 |
| ubuntu24-arm64 | 250 | 109824 | 58.115695 | 83.887680 |

Each filesystem row verifies 8,394,880 preloaded record bytes, two original leases and 101 entries per store. New-request windows include retirement coordination/assertions/destruction and exclude retained directory buffers. Raw setup and individual retirement clocks are in each Unix row's `verification` object in [validated-rows.json](native/validated-rows.json) and its original ZIP's `owned-verification.json`; Windows has no filesystem-witness row. These are requested Rust payload observations, not a hard workspace/RSS bound, allocator overhead, deterministic simultaneous peak, network traffic or original aggregate permit.
