# SC-OPT-007 sparse candidate pruning evidence

Status: work in progress. No performance acceptance or WAN result is claimed.

Baseline: `cc0199ffd255b9e2e002afe8c96f10462a3e474f`, freshly fetched from
`origin/main` before work. This milestone operates in the existing isolated
SpiralCore worktree on `codex/spiralcore-sparse-pruning`.

The unchanged baseline passed 185 release all-target tests, including the two
normally ignored large HTTP tests (`baseline-tests.txt`). Command:

```sh
CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target \
  /Users/casey.allard/.cargo/bin/cargo +1.97.1 test --locked --release \
  --all-targets --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --target aarch64-apple-darwin --features hardware-sha256 -- --include-ignored
```

The meaningful allocation regression was observed before production edits:
`tdd-red.txt` records the encoder returning Raw fallback while allocating eight
sparse header buffers, against the new expectation of zero. This marker counts
initial nine-byte allocations during encoding; it is not a peak-memory measure.

The immutable reference copies under `crates/spiralcore-transfer/tests/support/`
match the original baseline blobs:

| File | SHA-256 |
|---|---|
| `delta_legacy_cc0199f.rs` | `af31026c806c9046d62735b3ebdd03472e667103011a06b6f49ff76edb19e18e` |
| `protocol_legacy_cc0199f.rs` | `65fa50275a1487b937040c0f03de3cdeba8e49ce563e78d0bcb3763006804e6a` |

Setup failures remain separate from the expected optimization RED: the initial
oracle wrapper omitted an import, and an initial shell preflight used zsh special
variable names. Neither changed the production encoder. Their diagnostic records
are retained for review.

Measurement scope is local Rust encoder computation and a separate allocation
diagnostic. It does not include a network, durable receiver publication, TLS,
energy, or MVM integration. Previous WAN evidence remains bound to its own source.
Formal raw results and qualification records are intentionally retained at the
user's request; only disposable, unneeded diagnostics may be removed after their
disposition is recorded.
