# SC-OPT-007 sparse candidate pruning evidence

Status: **completed experiment; FAIL_CPU_GATES; candidate not adopted**. Exact
output and diagnostic-memory gates passed, while middle insertion and two
control CPU gates failed. The active encoder is restored to the published
baseline. See the [full result](../../sparse-pruning-results-2026-10-30.md),
[frozen registration](registration.json), and
[independent analysis](independent-review/actual-custody.json).

Measured candidate: `317c7f75b8b8355fbb2c74b4bad1944e4f0cb1e8`. Registration:
`5bd7c51832710176cca30bfd17ce071cfc80b6af`. `measurements.jsonl` and
`allocations.jsonl` each contain 140 observations. All 70 old/new pairs match;
each campaign verifies 120 decoded recipes plus 20 Raw decisions. No observations
were discarded or replaced. This is not a new WAN measurement.

The candidate-specific source/test snapshots are in `candidate-source/`. Their
hashes and the 67-entry source manifest identify the measured candidate, not the
post-rollback delivery tree. Reproduction requires checking out the measured
commit; the retained harness's `new` arm imports whichever encoder is active.
The [recovered qualification custody](recovered-qualification-custody.json)
clarifies the exact baseline test/build order from original session records.

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
