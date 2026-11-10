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

## Windows wake correction, 2026-11-10

The first hosted matrix at source `2eb707ad` passed Linux x64/ARM64 and
macOS Intel/ARM64, but failed Windows Server 2022 x64, Server 2025 x64
and Windows 11 ARM64. In all three, the injected failed-wake test received
`TimedOut` instead of `BrokenPipe` after the owner closed the carrier.
[Original failed run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42170936885).
Cloned socket shutdown did not reliably interrupt the Windows readiness wait
when both the normal and shutdown wake posts failed.

The correction checks the original owner's stopped flag before and after
each poll, with each poll capped at the lesser of 50 ms and the remaining
original deadline. Healthy readiness notifications remain immediate. This
fallback introduces at most 20 requested idle poll intervals per second
per continuously parked reader; observed wake latency also depends on
scheduling. It changes no wire bytes or object identity.

A new deterministic regression leaves both sockets open, posts no waker,
sets only the owner fence after poll entry, and requires `BrokenPipe`
within 500 ms of a two-second original deadline. Its initial build failed
because the checked wait did not exist. With the fix, all **47 FlowMux
tests passed with zero skipped** on Wick/Linux x64, Rust 1.97.1, and
strict all-target Clippy passed. These are component correctness checks;
they do not measure WAN throughput, CPU savings or installed MVM capacity.
