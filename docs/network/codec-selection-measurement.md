# Local codec-selection measurement contract

This is the reproducible local measurement for SC-OPT-006. It is separate from
the frozen WAN campaigns. Its harness is
[`examples/codec_selection.rs`](../../crates/spiralcore-transfer/examples/codec_selection.rs).
No socket, HTTP connection, TLS session, relay, or WAN is used. Body counts are
the equivalent lengths of the existing HTTP protocol bodies, obtained from
actual serialized frames; elapsed and CPU measurements are local work only.

## Registered question and acceptance

Does the opt-in adaptive sender choose the smallest eligible representation
before Begin without changing exact output or durable publication behavior?
For every paired target, require its measured body-equivalent bytes to equal
the minimum observed across the four fixed arms. Require its predicted bytes
to equal its actual serialized bytes. Every seed and target must survive
closing and reopening the endpoint and an independent byte-for-byte comparison
against the original in-memory input, including exact length and object identity.

Bytes are the primary acceptance metric. Record the extra computation, including
discarded candidates, rather than imposing an unregistered CPU improvement gate.
The exhaustive selector is a byte-minimizing reference among these configured
encoders. It is not a claim of optimal compression, minimum CPU, lowest latency,
or superiority over a matched rsync/VCDIFF implementation. Any later pruning or
heuristic policy needs its own comparison and acceptance record.

## Workload and schedule

The acceptance run has 30 trials per size and case, at 1 MiB (1,048,576 bytes) and
16 MiB (16,777,216 bytes). Each trial runs raw, independent-chunk zstd, sparse,
whole-base zstd prefix and adaptive against identical inputs. The arm order
rotates by `(trial + case_index) % 5`; all five positions occur six times per arm
in 30 trials. This is deterministic cyclic balancing, not randomized order or
complete carryover balancing. No timing observation is discarded or replaced.

| Case | Base and target | Seed |
|---|---|---|
| `cold_random` | Deterministic pseudorandom target | None |
| `cold_structured` | Repeated seed-derived 4 KiB template, with the first eight bytes of each record replaced by its index modulo 16 in little-endian form | None |
| `localized_32` | Pseudorandom base; exactly 32 midpoint bytes XORed with `0xa5` | Separate raw transfer for every arm |
| `unrelated_structured` | Pseudorandom base; separately generated structured target | Separate raw transfer for every arm |

`fixture()` defines the seed, xorshift recurrence and case/size/trial binding in
the preserved Rust source. Fixture generation and input allocation occur before
operation timing, equally for all arms. The same owned base/target input vectors
are available to each arm; this run does not charge source file I/O or seed-time
retained-base copying. It reopens the receiver between seed and update, so it
does not inherit an in-memory verified base from the seed.

There are 1,200 target operations and 600 paid seeds: **1,800 exact operations**.
All are retained. The paired body acceptance covers 240 target pairs. The planned
byte-comparison total is 16,043,212,800 bytes, counting seeds and targets. A
three-trial qualification run is a separately labeled smoke test; it cannot
replace the registered 30-trial run.

Cold sparse uses raw and cold prefix uses ordinary chunked zstd, matching the
earlier fixed-arm policy. Warm sparse/prefix keep their existing raw fallback.
All arms disable optional DAG warming and use identical endpoint limits and
durability. They each receive a fresh cache directory and scope. No preexisting
target hit is expected. The adaptive function receives `warm_probe=false` for
this workload; identity-repeat probing is qualified by the integration tests.

## Counters and timing boundaries

For each exchange, encode the requests with `wan::encode_batch`, decode them,
call the endpoint, encode/decode the replies, and count the serialized lengths.
The reply count includes 16 extra bytes for the two `SCWREP01` timing fields
relative to the `SCWBAT01` serializer. Those metric fields are counted by their
defined size; they are not an actual HTTP response in this local adapter.

The main wall/CPU timer starts immediately before the selected sender call and
ends when it returns from verified durable publication. It includes candidate
encoding, rejected candidates, object hashing, frame serialization, decoding,
endpoint work and durable publication. Selection-to-JSON conversion, endpoint
open/close, fixture generation, reopened-byte verification, output-file writes,
summary calculation and synthetic cache cleanup are outside this timer. Those
exclusions are recorded in the registration row. They differ from some earlier
WAN timing boundaries, so do not compare local elapsed times directly to WAN.

- `operation_cpu_micros`: `ProcessTime`, encompassing both sender and receiver
  in one serial process. It is not an endpoint-summed measurement from two hosts.
- `receiver_*`: nested request decoding, endpoint handling and reply encoding.
- `exchange_*`: nested serialization/decoding/handler work for the whole local
  exchange. This contains the receiver interval.
- `operation_wall_micros`: elapsed operation time including local computation
  and publication waits. It is not transfer-only or network-only time.

Never add receiver or exchange CPU to operation CPU: they are already included.
Never subtract CPU from wall time to invent a send time. No peak-memory or energy
measurement is supplied by this harness. Normal clock/counter overhead remains
in the measured work. No builds, tests, profiles or other benchmark processes
may run concurrently with the registered timing campaign.

## Reproduction and custody

Build with a pinned toolchain and lockfile, then record the checked-out source,
working-tree status, build command and executable hash before starting. The
`--source` argument is a caller-supplied revision; the harness explicitly labels
its verification external. It hashes its own executable and embedded harness
source and records platform and hardware-SHA feature selection. Parent evidence
must bind the remaining source files, dependencies and build settings.

```sh
CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target \
  /Users/casey.allard/.cargo/bin/cargo +1.97.1 build --release --locked \
  --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --target aarch64-apple-darwin --features hardware-sha256 \
  --example codec_selection

/Users/casey.allard/spiralcore/target/aarch64-apple-darwin/release/examples/codec_selection \
  --trials 30 --sizes 1048576,16777216 \
  --out NEW_ROWS_JSONL --cache NEW_CACHE_ROOT --source FROZEN_40_HEX_REVISION
```

Both output and cache root must be new. The output uses `create_new`; the cache
root uses `create_dir`, so an existing artifact is not overwritten. Every JSONL
row is flushed and synchronized. An operation failure appends a failure event,
returns an error and preserves that operation's cache. Earlier successful rows
remain intact. A partially executed file is incomplete evidence.

Successful synthetic cache children are removed only after all phase reopening
checks, followed by a cleanup row. The run root and ownership marker remain.
The JSONL ends with paired acceptance, 40 cell summaries, and completion only
after validating the full counts, matching fixture/seed identities and minimum
byte decisions. Summaries include actual selected-mode counts, min/median/p95/max,
seeds, seed-plus-transfer cost, and paired raw/candidate ratios. Medians use the
midpoint of the two central observations; p95 is nearest rank (29th of 30).
Retain raw observations and failures, not just the summaries. An independent
review must recompute the checks and reported statistics before making a claim.

## Qualification scope

The example's tests check fixture repeatability and edit extent, workload/order
cardinality, configuration bounds and ambiguity, distribution definitions, and
a real 30-operation endpoint/reopen run at 64 KiB. They also require matching
adaptive predicted/actual body counts and preservation of an existing output.
These tests are correctness qualification, never a performance sample.

Development qualification on 2026-10-29 used Rust 1.97.1,
`aarch64-apple-darwin`, and `hardware-sha256`. The test-first sequence retained
three observed compile failures: missing `fixture` (three E0425 diagnostics),
missing harness/configuration/schedule functions (nine E0422/E0425/E0433
diagnostics), and missing `distribution` (three E0425 diagnostics). Each exited
101 before its implementation. The completed example suite then passed five
tests. Clippy initially rejected one manual range check; replacing only that
expression with `RangeInclusive::contains` produced a clean `-D warnings` run.
The final example test run and standalone rustfmt check both exited zero.
This is a development record, not a substitute for the final source-bound
qualification and timed-campaign receipts.

The implementation milestone does not update the historical WAN report. New WAN
results require a separately bound sender/receiver execution and capture run.
The later MVM integration still needs the canonical encrypted mailbox/vsock
adapter and the small-MVM/orchestrator resource and lifecycle contract.
