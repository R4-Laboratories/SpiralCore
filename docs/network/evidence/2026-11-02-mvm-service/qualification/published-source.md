# Published-source canonical service checkpoint

Date: 2026-11-02. Tracker: [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43).
This checkpoint is component and loopback correctness evidence. It is not a
WAN benchmark, native guest witness, Windows execution or production acceptance.

The test host reports macOS 27.0 build `26A5378n`, Apple Silicon. Rust 1.97.1
resolved the public contribution fork `Casey-allard/mvm` at
`5766754fbaae59dbee548f968716a514d3a796c1`, based on upstream `6ae461a`.
No local Cargo patch configuration was supplied. Registry dependency versions
were retained from the prior committed locks where compatible; resolution
changed the canonical MVM source and its required dependency closure.

The implementation was still uncommitted at this checkpoint; the containing
checkout HEAD was `3e9226403f65652f7bf47e207a594b451344343b`. Exact reviewed source:

| File | SHA-256 |
|---|---|
| `src/service.rs` | `208a2382ad4d539d9f53f22aa3eaaf6ce3e005a069745abf6c162ead4adc3d90` |
| `src/receipt.rs` | `d5faf9945cb591218c098459ef812e1fc935ac03f976c84cc7cd8cbb8a112b9a` |
| Adapter `Cargo.lock` | `9f5f0796fa03e121c5fd01ffb57758df41db0cbd991e5d9d5fedf33010392f9e` |
| Live consumer `Cargo.lock` | `2c8d52cd04947d2616c283305f6c9f36df930b7190b977238dd5fe6af035ce6e` |

Commands used `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0` and the existing shared
target directory. The selected portable targets first passed **48 tests** with
`--features live`; the full command below then passed **76 tests**, zero failures,
ignored or filtered tests:

```sh
cargo +1.97.1 test --locked \
  --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml \
  --all-features --all-targets
```

| Target | Tests passed |
|---|---:|
| Library / FIFO ordering | 8 |
| Canonical envelope and authorization | 11 |
| Existing live socket lifecycle | 10 |
| Encrypted destination receipts | 15 |
| Bounded mailbox service | 16 |
| Independent destination epoch regressions | 3 |
| Shared object adapter | 4 |
| Sender helpers | 2 |
| New canonical service over TCP | 1 |
| Existing codec transfer | 3 |
| Object-v1 endpoint | 1 |
| Mailbox transfer example | 2 |

The TCP test transfers a 65,000-byte deterministic binary prefix plus an embedded
and trailing-zero suffix. It verifies the exact application Reply, separately
verified signed admission for both Call and Reply, and exactly two transmitted
and two received envelopes per endpoint. Envelope counters agree across peers;
there is no receipt-of-receipt traffic. This is actual localhost TCP, not WAN.

The full strict Clippy command initially rejected a test-only default-field
assignment in `tests/service.rs`. After the minimal struct-initializer correction,
the same full command with `clippy` in place of `test` and `-- -D warnings`
passed (1.50 s incremental check). The reviewed service and receipt hashes
above were unchanged by that test-only correction. This check duration is not a
transfer performance result.

A separate full-adapter Windows MSVC cross-check on Wick failed in the `ring`
C build because the Linux environment lacks MSVC `lib.exe`; its GNU compiler
cannot build that target. No source or cryptography substitution was made to
hide this environment failure. It neither overturns the earlier narrower
canonical MVM compilation result nor establishes a Windows adapter result.
The native Windows runner jobs are the next execution gate.
