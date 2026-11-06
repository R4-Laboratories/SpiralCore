# FlowMux reader cleanup port

Source `334d1432e3e4b399b64d941cdf6def0c2289630e` ports the original reader
cleanup API and both public-path tests from [`6c885fe5`](https://github.com/R4-Laboratories/SpiralCore/commit/6c885fe5a1a03d65a6f9093edc407c98feff31be)
onto SpiralCore main at `f531b5776c157eace3bdfbb16c3e8ee594f4606d`. The scope
is the original reader handle, absolute cleanup deadline and result retention.
The tests also cover cancellation through a mapped alias. Terminal frame
handling and ordered retention were outside this port.

On Linux x86_64 with Rust 1.97.1, the two public-path tests first failed to
compile against main because the cleanup APIs were absent. After the port,
both tests and the other 34 locked FlowMux tests passed with zero failures
or skips. Strict all-target Clippy and the locked release build passed.
Independent Rust/code and security reviews checked the cleanup deadline,
handle ownership, alias behavior and retained worker result.

| Port input | SHA-256 |
| --- | --- |
| `crates/spiralcore-mvm-flowmux/src/duplex.rs` | `1c9efd6db92ae6d5ebce341bc115da9acea327b88db33f98cbdc692933ea67db` |
| `crates/spiralcore-mvm-flowmux/src/stream.rs` | `d625173e9e6fd923f2101fb5f9cd3c80c68d757c96d3dee6b771f53d44d7bd6e` |
| `crates/spiralcore-mvm-flowmux/tests/reader_completion.rs` | `d155c0f01fda67d7b31e4f82ca974dbdf332e7040498d6d83d56204a5722111b` |

This is component evidence. Installed service integration and WAN acceptance
are tracked in [#44](https://github.com/R4-Laboratories/SpiralCore/issues/44).
