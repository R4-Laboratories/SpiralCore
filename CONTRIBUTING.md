# Contributing to SpiralCore

Start with the [README](README.md), [delivery status](docs/network/delivery.md)
and [optimization ledger](docs/network/optimization-ledger.md). Open an issue or
pull request at [R4-Laboratories/SpiralCore](https://github.com/R4-Laboratories/SpiralCore).
Identify the track: object transfer, IP packets, MVM, or research. Evidence on
one track does not automatically qualify another.

## Development

Active implementations and measurement helpers use Rust. Standalone crates have
their own lockfiles. Recorded builds use Rust 1.97.1; use `--locked` and reuse
`CARGO_TARGET_DIR` when disk space is constrained. Preserve supplied Python
prototypes and historical artifacts as references.

Run focused tests for the affected crate, for example:

```sh
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-gateway-core/Cargo.toml --all-targets
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-gateway/Cargo.toml --all-targets
```

TUN execution requires Linux and documented namespace privileges. MVM live tests
require the patched dependency configuration; follow the [MVM instructions](crates/spiralcore-mvm-live/README.md).
A successful cross-build does not establish actual TUN or guest execution.

## Correctness and measurement

- Preserve exact bytes and lengths, including zeros, insertion, deletion, append
  and truncation. Interrupted/corrupt input must not produce verified publication.
- Preserve adapter authorization, cache scope and policy/epoch checks. A content
  hash is not a capability.
- Pin source, binaries and fixtures before benchmarking. Match transport,
  durability and cache state across comparisons.
- Record both directions' bytes at an explicit layer, CPU, elapsed, setup and
  recovery. CPU overlaps elapsed; do not add it twice or call the difference
  pure send time.
- Retain observations, failures and negative results. Distinguish local, shaped
  and real Internet WAN measurements; include conventional controls.
- Keep compact evidence in Git and larger reviewed bundles in [public releases](docs/network/public-artifacts.md).
  Do not publish credentials, private keys, unrelated traffic, stores or build caches.

## Scope and licensing

Keep changes focused. Explain what changed, why, and what was actually tested.
Original contributions use MIT; preserve imported notices and declare borrowed
material. See [LICENSING.md](LICENSING.md).
