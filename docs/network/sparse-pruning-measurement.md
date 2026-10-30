# Sparse candidate pruning: local measurement contract

SC-OPT-007 compares the unchanged legacy sparse encoder at
`cc0199ffd255b9e2e002afe8c96f10462a3e474f` with the candidate public
`delta::encode`. The [proposal](sparse-candidate-pruning-proposal.md) supplies
the proof and acceptance gates. No transport, endpoint, cache or WAN operation
is timed. There is no CPU-to-energy inference.

The Rust example is
[`examples/sparse_pruning.rs`](../../crates/spiralcore-transfer/examples/sparse_pruning.rs).
It includes immutable byte-for-byte legacy encoder and protocol snapshots from
`tests/support`. The legacy decoder reconstructs every selected recipe after
timing. The implementation tests separately compare actual frame vectors; this
cross-process harness binds serialized frame order, lengths and operation count
in a domain-separated SHA-256 receipt and requires matching paired receipts.
`None` is recorded as a Raw decision using the original bytes, not a fictitious
decoded transfer.

## Fixtures and schedule fixed before timing

All targets contain exactly 16 MiB. Input allocation, generation and object-ID
hashing happen before timing with equal input ownership. The deterministic seed
and byte generators are part of the preserved example source.

| Class | Case | Exact workload |
|---|---|---|
| Primary | `prefix_insert` | 4 KiB of `0x5a` prepended to a pseudorandom 16 MiB minus 4 KiB base |
| Primary | `middle_insert` | Same size change, inserted at the base midpoint |
| Primary | `unrelated` | Independently generated full-size pseudorandom replacement |
| Control | `repeat` | Exact full-size pseudorandom repeat |
| Control | `localized` | 32 midpoint bytes XORed with `0xa5` |
| Control | `scattered` | 32-byte XOR changes beginning at byte 4,096 and repeated every 32,768 bytes, exercising the aligned recipe |
| Control | `periodic` | A repeated 4 KiB pseudorandom template; replace the first 4 KiB with `0x5a` |

Three separate legacy-only processes per case establish the pre-edit baseline:
21 observations. These are not the acceptance sample. The first observation is
retained like every other observation; there is no discarded warmup. The observed
baseline minimum CPU interval was 7,346 microseconds, above the predeclared
100-microsecond resolution floor. The [baseline receipt](evidence/2026-10-30-sparse-pruning/baseline-source.json)
binds the original runtime, new harness and executable separately.

The acceptance schedule is case-major, ten old/new process pairs per case,
alternating AB, BA, AB, BA and so on: exactly five of each order and 140 samples.
Each process performs **one timed encode call**. Any CPU duration below 100
microseconds fails resolution qualification; it is not silently rerun, batched,
discarded or replaced. Inputs vary with the registered pair number and match
between old/new arms. No builds, tests or profiles owned by this task run during
timing. This is a shared developer host, not a controlled thermal environment.

## Timing and verification

The primary clocks measure process CPU and wall time from encode entry through
return. Frame hashing and legacy-decoder byte comparison happen next, outside
that interval. Returned-recipe destruction receives a separate CPU/wall timer.
The example records the verification interval too.

`encode_plus_drop_*` is the sum of the two disjoint timed intervals; it is not
a continuous elapsed interval. `entry_to_release_including_verification_*` is
the actual continuous span and explicitly includes verification and the gap
between timers. Only encode CPU participates in the proposal's CPU gate.
No field is described as transfer-only time, WAN time or application goodput.

The acceptance CPU gate is a geometric mean of paired old/new encode CPU ratios
of at least 1.10 and at least seven candidate wins in ten for **each** primary.
Each control requires a paired geometric mean of new/old CPU at most 1.05.
Report wall distributions, drop time, every pair and all negative gates without
changing the thresholds after observation. Fixture and output mismatch is a
correctness failure, regardless of timing.

## Separate frame-buffer allocation diagnostic

The ordinary timed executable has no custom allocator or allocation tracer.
A separately built diagnostic uses the example-only compile configuration
`sparse_allocation_diagnostic`. Its allocation code is excluded at compile time
from the timed binary. The diagnostic's timings are never performance evidence.

During the exclusive encoder invocation, the tracer identifies the nine-byte,
alignment-one allocation from the unchanged sparse `header()` constructor.
For the pinned encoder sources this is the sole allocation of that shape in the
encoder; operation/frame metadata vectors and caller-owned inputs are excluded.
This attribution is source-bound and must be re-reviewed if those constructors
change. It is not a general allocation profiler that identifies arbitrary objects
from size alone.

The diagnostic tracks those pointers in a fixed-capacity thread-local registry
and follows their growth and deallocation. Tracked reallocation deliberately
uses allocate-copy-free, so old and new capacities actually overlap under this
documented diagnostic allocator policy. Both arms use the same policy. It records
actual requested allocation sizes, cumulative allocated capacity, peak live
capacity, reallocations and frees. Registry exhaustion is a failed diagnostic,
never a successful partial measurement.

The diagnostic process is single-threaded; its process-wide enable/failure flags
are not a concurrent-session interface. Run diagnostic unit tests with
`--test-threads=1`. Qualification covers retained-buffer restart refusal,
registry exhaustion and duplicate entries, missing entries during growth,
reentrant registry callbacks, and injected allocation failure during tracked
growth. The failure injection exists only in the test executable and requires
the original pointer, bytes and registry counters to survive a failed growth.
Successful growth and final drop must balance actual tracked allocations/frees.

Stop discovering new frame buffers at encoder return, retaining the existing
registry through returned-recipe destruction. Reconstructed-output and receipt
allocations during verification are therefore excluded. Record the retained
capacity at return, require it to equal the returned frame Vec capacities, then
require zero tracked live bytes/buffers after drop.

The proposal's memory gate is at least 50% lower peak frame-buffer capacity in
every one of the ten pairs in each primary **under this diagnostic allocator
policy**. This stronger pair-by-pair requirement was fixed before acceptance
collection. It does not establish
production system-allocator overlap, RSS, total allocator reservation or host
memory savings. Under the unchanged `pack` implementation, every materialized
candidate must allocate its initial frame header; zero such allocations for a
rejected candidate supports the no-materialization claim. The control-flow proof
and focused allocation regression tests remain necessary alongside the counters.

## Reproduction and evidence

Build normal release code with Rust 1.97.1, the pinned lockfile and the recorded
target/features. Freeze source/executable identities before collection. Example:

```sh
CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target \
  /Users/casey.allard/.cargo/bin/cargo +1.97.1 build --locked --release \
  --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --target aarch64-apple-darwin --features hardware-sha256 \
  --example sparse_pruning

/Users/casey.allard/spiralcore/target/aarch64-apple-darwin/release/examples/sparse_pruning \
  campaign --source FROZEN_40_HEX_REVISION --out NEW_JSONL_PATH
```

Build and qualify the diagnostic separately, then run its untimed allocation
campaign against the same frozen source. Never use that executable for CPU
acceptance:

```sh
RUSTFLAGS='--cfg sparse_allocation_diagnostic' \
  CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/local/sparse-pruning-20261030/diagnostic-target \
  /Users/casey.allard/.cargo/bin/cargo +1.97.1 test --locked --release \
  --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --target aarch64-apple-darwin --features hardware-sha256 \
  --example sparse_pruning -- --test-threads=1

RUSTFLAGS='--cfg sparse_allocation_diagnostic' \
  CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/local/sparse-pruning-20261030/diagnostic-target \
  /Users/casey.allard/.cargo/bin/cargo +1.97.1 build --locked --release \
  --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --target aarch64-apple-darwin --features hardware-sha256 \
  --example sparse_pruning

/Users/casey.allard/spiralcore/local/sparse-pruning-20261030/diagnostic-target/aarch64-apple-darwin/release/examples/sparse_pruning \
  allocations --source FROZEN_40_HEX_REVISION --out NEW_ALLOCATION_JSONL_PATH
```

The driver launches a new copy of the same executable for every sample, records
the child's executable hash and PID, and synchronizes every JSONL row. Existing
output is refused with `create_new`. Nonzero child exit, spawn failure and invalid
child JSON preserve failure details and earlier observations. Raw child output
is retained on failure. Any incomplete file remains incomplete evidence.

The source revision argument is caller supplied; external custody must verify
the actual core/harness/source hashes and build settings. Keep baseline, normal
paired campaign and allocation diagnostics separate. Independently recalculate
summary statistics and acceptance gates from the raw records before publication.
No warm cache, MVM integration, 64 MiB client, WAN or energy qualification follows
from this local encoder experiment.
