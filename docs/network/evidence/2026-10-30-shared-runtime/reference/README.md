# Pinned Python mathematical reference

This packet records the behavior of the user's existing Python kernel for a
Rust shared-runtime port. Python is used only for reference extraction and
qualification. The production engine and transfer integration are Rust.

The source is the `spiralcore_kernel` literal extracted from the original local
wrapper, not a historical repository export:

| Artifact | SHA-256 |
| --- | --- |
| `/Users/casey.allard/spiralcore/spiralcore.py` | `77076e195d3f93d91b79b653e946af58b215c52166382269de9360da83226ad0` |
| Extracted `spiralcore_kernel.py` | `2e11825b16a825f066bc249d785d436c3345abe4fd2c0958fbdb156dea9eb269` |

`extraction.json` is an exact copy of the original extraction manifest.
`source-excerpts.json` preserves each selected source segment, original line
numbers, segment hashes and extraction edits. `generation.json` records the
generator hash, actual argument vector, interpreter/platform and unchanged-source
checks. `generation.exit` records the shell-observed exit code.

## Execution boundary

The generator verifies the whole kernel hash **before** parsing or compiling
definitions. It compiles only selected function ASTs with a restricted builtin
set plus `math` and `cmath`. It does not import the original module, instantiate
its runtime, load cards, execute saved source, create UI/audio, access the network
or read the wall clock. No third-party imports or installations are used.

The runtime reference executes the original `step` prefix through the six CA
updates and particle invariant, stopping before routing, audio, audit or memory
card calls. It binds the unchanged original `memory_field`, `think`, and the
subclass's fixed-time `sync_system_to_kuramoto` to a minimal state. The initial
time is explicitly zero and `fixed_dt` is always supplied; the unavailable
wall-clock branch is never executed. Thus this is a reference for the bounded
mathematical transition, not a qualification of the entire desktop program.

The only random operation is the explicitly requested deterministic standard-
library `random.Random(184)` phase initialization. The 16 raw phase values and
their IEEE-754 binary64 bits are recorded, followed by the original initial
`kuramoto_step(..., K=1.75, dt=0.05)` warmup.

## Coverage and compatibility boundaries

`vectors.json` contains:

- 63 CA cases across rules 184, 110 and 30: empty rings, all one/two-cell states,
  all eight neighborhoods, the original default 48-cell row, low-bit masking,
  and four 256-bit byte blocks. Each case records its initial state and every
  bounded subsequent step; bit ordering is MSB first.
- 11 standalone Kuramoto traces with explicit phases, frequencies, coupling and
  time step; four separate two-oscillator field-model examples; three historical
  unequal-length observations.
- 14 integer valuation examples and two fold-only sequences through 81 and 27
  steps, respectively.
- Seven coupled-runtime cases totaling 123 transitions: default 48 cells through
  81 steps; widths 1, 2 and 256; disabled memory coupling; and low/high time-step
  clamps. All six rings, phases, frequencies and root summaries are retained.

The source masks CA inputs to their low bit and allows an empty ring. The Rust
checkpoint profile deliberately requires a positive bounded width and binary
cells. The 57 valid nonempty binary CA cases are exercised through the Rust
state transition; the other six remain historical behavior observations.

`kuramoto_step` clamps its own `dt` to `[0.0001, 0.5]`; the runtime synchronization
method first clamps elapsed time to `[0.005, 0.25]`. Coupling is
`K * sin(mean_phase - phase)` when the mean-field magnitude exceeds `1e-12`,
without an additional magnitude multiplier. The Python function silently
truncates unequal phase/frequency vectors with `zip`, while dividing by the
original phase count. The Rust profile's fixed arrays exclude this malformed
input shape.

The observed fold is an unweighted average over current roots and a sum of their
memory counts and phase fields. At step 9, payloads 1 through 9 produce value
6.2, not the original-payload mean 5. This is a lossy mathematical summary, not a
lossless payload store, and it is not substituted for the immutable object DAG.

CA cells, counters and lengths require exact equality. Python/libm floating
outputs are numeric references, **not cross-language bit-identity guarantees**.
The Rust integration tests compare floating fields with absolute-plus-relative
tolerance `1e-10 * (1 + abs(expected))`. Separately, checkpoint encode/decode,
resume and replay tests require exact byte equality within the Rust profile.

## Reproduction

Run from the repository root with the preserved local source available:

The Python extraction step is **not self-contained in a fresh clone**: it needs
the pinned local wrapper and extracted kernel at the paths below (or equivalent
copies with identical hashes). Those originals remain preserved locally because
they may contain newer user work outside this import's publication scope. The
bundled source excerpts and vectors are inspectable, and the Rust reference and
canonical-vector tests run from the repository's retained fixtures without
executing or requiring the original Python program.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 docs/network/evidence/2026-10-30-shared-runtime/reference/test_generator.py
PYTHONDONTWRITEBYTECODE=1 python3 docs/network/evidence/2026-10-30-shared-runtime/reference/generate_reference.py \
  --source /Users/casey.allard/spiralcore/local/shared-state-20261030/reference/spiralcore_kernel.py \
  --wrapper /Users/casey.allard/spiralcore/spiralcore.py \
  --extraction /Users/casey.allard/spiralcore/local/shared-state-20261030/reference/extraction.json \
  --out docs/network/evidence/2026-10-30-shared-runtime/reference
CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target \
  /Users/casey.allard/.cargo/bin/cargo +1.97.1 test \
  --manifest-path crates/spiralcore-transfer/Cargo.toml --release \
  --target aarch64-apple-darwin --features hardware-sha256 --test shared_runtime
```

The retained RED receipts show the absent reference generator and then the
absent full-transition reference helper. The first extractor attempt failed
because its restricted builtin set omitted `round`; adding that required
builtin resolved it without changing source mathematics. Eight extractor tests
then passed. `rust-engine-red.txt` records the Rust test failing because the
production `shared_runtime` module did not yet exist. Subsequent Rust receipts
are separate from these reference-tooling results.

The first actual Rust run passed 14 of 15 tests, including all Python transition
comparisons. Its failure was a test assumption: a 4096-cell, 4096-step trace can
fit below 16 MiB because CA rows are bit-packed. The limit regression was changed
to supply an explicitly insufficient output budget and separately check a caller
budget above the profile maximum. The original failure is retained in
`rust-engine-green-attempt2.txt`; it is not a production fidelity failure.

`rust-engine-green-attempt4.txt` records 20 passing integration tests after
adding explicit split-replay checks, root-retention coverage and native golden
vectors. `native-vectors.json` anchors four native outputs (48 cells, fixed
logical time step 0.05, payload 1): checkpoints at N=0, N=9 and N=81, plus a
nine-step trace. It records their exact byte lengths, SHA-256 and object IDs.
These are **Rust-produced compatibility vectors**, not Python byte comparisons
or completed cross-platform qualification. They were captured with Rust 1.97.1,
`aarch64-apple-darwin`, release optimization, `hardware-sha256`, and pinned
`libm=0.2.16` with `force-soft-floats`.

The initial native-vector capture deliberately compared against an empty golden
fixture, retained its actual output, then populated the fixture with:

```sh
jq -R 'select(startswith("NATIVE_CANONICAL_VECTORS=")) | ltrimstr("NATIVE_CANONICAL_VECTORS=") | fromjson' \
  docs/network/evidence/2026-10-30-shared-runtime/reference/native-vector-capture.txt \
  > docs/network/evidence/2026-10-30-shared-runtime/reference/native-vectors.json
```

The producing engine SHA-256 was
`1f56f592810d9b47494d5a3e0f036e27d9522e50b4f9dcde8f292b846a48ecff`;
the capture test executable SHA-256 was
`9b58e0ba633ea4e6b3016c8b02441a5f087a9f6d0652ff15d9283145380ce23c`.
Later preflight/allocation changes must keep these golden outputs unchanged.

`preflight-red.txt` records the separate allocation regression: the nine-step
trace had length 4,678 but retained `Vec` capacity 16,777,216. This is an observed
allocation property on the qualified platform, not an RSS measurement. The
follow-up exact-size preflight also tests exact output budgets versus budgets
one byte too small; it does not change the mathematical transition or wire bytes.

Final focused qualification is recorded in `engine-transfer-post-review-tests.txt`:
22 engine tests and seven transfer tests passed. The preflight capacity regression
now passes without changing the four native golden outputs. Actual all-target
`cargo check` and `cargo clippy -- -D warnings` also passed; see
`review-all-target-check.txt` and `review-all-target-clippy.txt` with their zero
exit files. Earlier tool-path, exhaustive-match and Clippy failures remain
retained. `engine-review.json` scopes the independent production-engine review
and records the final source hashes; it does not independently approve its own
reference-generator/test authoring. Test coverage percentages, cross-platform
qualification, WAN measurements and energy claims are not established here.

The later all-target run exposed one historical unsupported-mode test using byte
5, which is now the assigned `RuntimeV1` mode. The test alone now uses unassigned
byte 255; the parser is unchanged. `legacy-prefix-regression-green.txt` records
all eight prefix tests passing after that one-line correction. The original
failure remains in `../qualification/all-targets-final.txt`.
