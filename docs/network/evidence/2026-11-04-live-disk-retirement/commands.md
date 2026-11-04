# Executed scoped Linux commands

The emitted source was compiled under the locked canonical Git graph with `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo +1.97.1 test --manifest-path crates/spiralcore-mvm-live/Cargo.toml --release --locked --features live,linux-host,canonical-host --lib installed_disk::retirement_tests -- --test-threads=1 --nocapture`. The first missing-API RED and successful successor outputs are retained here.

Before the independent execution below, the emitted `spiralcore_mvm_live-99b49e51f8c202a6` binary was copied to `/var/tmp/spiralcore-resident-parent-20261103/live-disk-protected/live-lib` and chmod500. Source5 and binary hashes were captured before/after; the three Rust files in this checkpoint match those hashes.

```sh
./live-lib installed_owner::endpoint_disks::tests:: --test-threads=1 --nocapture
./live-lib installed_disk::retirement_tests:: --test-threads=1 --nocapture
```

The first scope contains10 test entries, one a subprocess helper entry. The second contains1 meaningful scope-binding test. The five new live-cleanup tests are a subset of the10 entries. No new VM, WAN or throughput run occurred. The independent source review is recorded separately; it is not execution evidence.
