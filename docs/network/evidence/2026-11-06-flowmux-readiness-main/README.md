# FlowMux TLS readiness and terminal EOF

This port starts at SpiralCore main `0198349f` and takes the readiness
implementation from the installed development tree, retaining the current
main scope bytes, object framing, and reader cleanup API. A reader parks
on socket readiness under its original deadline. A shared wake notifies
the parked reader when another owner changes the TLS session. The worker
distinguishes physical EOF from authenticated application completion.

The combined installed candidate first reproduced a clean-close failure.
Both directions had sent application FIN, but Rustls returned
`UnexpectedEof` when its peer closed the transport. The reader fenced a
verified buffered suffix and the exact readback failed. An instrumented
repeat observed the same Rustls error. The polling version on main passed
one run of the same fixture.

The fix sends this EOF through the existing terminal check only after
the authenticated remote FIN. The original transmit lock synchronizes
publication of the local FIN, and the pending reply must be empty. The
completion check preserves a fatal delivery fence. Two real TLS controls
verify that missing remote FIN and withheld `Published` remain failures.

## Locked component check

On Wick with Rust 1.97.1, all 46 FlowMux tests passed with zero skipped.
The same source passed strict all-target Clippy.

```sh
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml \
  --all-targets -- --test-threads=1
cargo +1.97.1 clippy --locked --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml \
  --all-targets -- -D warnings
```

The component tests verify actual paired TCP and TLS behavior. Installed
MVM resource, CPU, platform, and real-WAN measurements remain separate
qualification work under [issue #44](https://github.com/R4-Laboratories/SpiralCore/issues/44).
