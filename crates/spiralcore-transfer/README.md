# SpiralCore exact object transfer

This standalone Rust prototype transfers exact byte objects into a persistent endpoint cache. A cold
send uses raw bytes or bounded zstd chunks. A warm send uses a caller-selected base object's ordered
32-byte ternary DAG and transfers new records, or uses a versioned sparse COPY/LITERAL recipe.
Whole-object identity includes the exact byte length
and full SHA-256; the receiver verifies reconstruction before publishing an object.

The `spiralcore` CLI is a **loopback-only experiment for public or synthetic data**. Its fixed Context and local
Authority gate are explicit test fixtures. They do not authenticate another process or encrypt traffic.
Any local process able to connect can use the same demo policy. Internet deployment, MVM integration,
WAN performance, and FIPS validation are not qualified by this harness.

The separate `spiralcore-wan`, `spiralcore-bench`, and `spiralcore-capture` binaries support a bounded,
authenticated two-machine experiment over HTTPS. They measure synthetic transfers through a relay,
including endpoint CPU, time through durable commit, and captured IP bytes. See the
[WAN instructions](../../docs/network/wan.md) for the transport, scope, and reproducibility limits.
The WAN receiver accepts only scopes in its generated experiment plan; it is not a general public
file-upload service.

## Build and run the demonstration

Run from the repository root with Rust 1.89 or newer:

```sh
cargo build --release --locked --manifest-path crates/spiralcore-transfer/Cargo.toml
BIN="$PWD/crates/spiralcore-transfer/target/release/spiralcore"
"$BIN" demo --cache /tmp/spiralcore-first-demo/cache
```

Choose a cache path that does not exist yet. The endpoint creates and exclusively owns its dedicated
cache. The demo refuses a nonempty cache so a prior warm object cannot be reported as a cold transfer.
Keep the cache after the run to inspect the published objects, or choose another path for a new run.

`demo` runs a real TCP listener on `127.0.0.1` and three client connections. It sends a deterministic
1 MiB object cold, sends the identical object with its DAG base, then changes one 32-byte region and
sends the mutation with the original base. It closes the server, reopens the persistent cache, and
checks every resulting byte against the original inputs. It prints three JSON transfer records and a
final `restart_verified: true` record only after all comparisons pass. The repeat refers to an existing
object, so there are three transfers and two distinct published objects.

## Use separate sender and receiver processes

Prepare two public test files. In the receiver terminal:

```sh
"$BIN" serve --cache /tmp/spiralcore-session/cache --bind 127.0.0.1:7777 --sessions 3
```

After `READY addr=127.0.0.1:7777`, run in another terminal:

```sh
"$BIN" send --addr 127.0.0.1:7777 --file ./base.bin --mode raw
"$BIN" send --addr 127.0.0.1:7777 --file ./base.bin --base ./base.bin
"$BIN" send --addr 127.0.0.1:7777 --file ./changed.bin --base ./base.bin
```

The first transfer commits the base at the receiver. `--base` selects DAG mode and requires that the
receiver already has the matching base publication; it is not a claim that an arbitrary file exists
remotely. The endpoint may decline DAG caching when its node budget cannot admit an object. Cold raw
or zstd transfer remains separate from that admission. The current DAG node limit is 262,144, so a
64 MiB protocol ceiling does not imply every 64 MiB object can use a DAG base.

Use `--mode zstd` instead of `raw` for a cold compressed send. Each chunk falls back to raw when
compression does not make that chunk smaller. Raw/zstd modes cannot be combined with `--base`. The
default cold mode is raw. Files are read once into a buffer with a 64 MiB ceiling and a bounded extra
byte check; the bytes used for identity are the bytes transferred.

To preserve reuse across insertions, deletions and changed lengths, explicitly select sparse mode:

```sh
"$BIN" send --addr 127.0.0.1:7777 --file ./changed.bin --base ./base.bin --mode sparse
```

Sparse mode requires a verified whole-object base at the receiver and currently limits both local
inputs to 16 MiB. It compares aligned runs and common prefix/suffix regions, sends bounded COPY and
LITERAL operations, and checks final length/SHA-256 before durable publication. Poor recipes select
raw before Begin; a missing remote base is an explicit error requiring a new transfer. Sparse
publications skip optional DAG warming, so a subsequent sparse transfer can use their ObjectId
without building a DAG. The CLI exchanges frame receipts sequentially; the WAN adapter batches them.
See the [binary contract](../../docs/network/sparse-v1.md).

Rust callers can retain a `prepared::PreparedObject` across same-length changes and call
`prepare_delta` followed by `client::transfer_prepared_batched`. `client::transfer_sparse_batched`
accepts local base/target slices. Prepared state is immutable; promotion creates a bounded flat
successor index. Whole-object comparison/hash and closure traversal remain linear work. Sender
preparation and its temporary allocations belong to the caller's timing and memory budget.

The opt-in Rust API `client::transfer_adaptive_batched(target, base, transfer_id,
warm_probe, exchange)` chooses among raw, chunked zstd, sparse and zstd prefix
before Begin. `base` is `Option<&[u8]>`; `exchange` accepts the existing batch of
frames and returns its replies. For the HTTPS experiment adapter, pass
`|frames| http_client.exchange(frames)`. An already authorized receiver must have
the same base for a selected sparse/prefix offer. Missing-base and authorization
errors are returned without a hidden retry.

The returned `AdaptiveStats` contains `transfer` (actual protocol-frame statistics)
and `selection` (candidate costs and selected mode). Selection minimizes projected
successful-miss HTTP body bytes among the configured encodings, including replies;
actual HTTP body counts come from `HttpClient::metrics()`. It evaluates losing
encodings too, so its CPU cost can exceed a well-chosen explicit codec. Both inputs
remain bounded to 16 MiB. An exact local repeat with `warm_probe=true` takes a
separate raw identity-probe shortcut; an evicted receiver then gets raw bytes.
See the [selection decision](../../docs/network/codec-selection-decision.md) and
[local measurement method](../../docs/network/codec-selection-measurement.md).
The existing CLI and historical WAN plans retain their explicit codec behavior.

## Fitted schemas and trained zstd dictionaries

The opt-in Rust prototype exposes `schema::fit(&training_samples)` and
`zstd_dictionary::train(&training_samples, dictionary_size)`, returning immutable
serialized artifacts. Fit only on a separate training split. Send the artifact
once as an ordinary Raw object in the recipient's authorized namespace, then use
`client::transfer_schema_batched(artifact, target, transfer_id, warm_probe, exchange)`
or the identically shaped `client::transfer_zstd_dictionary_batched` API.

The schema family retains literal JSON layout and carries exact raw primitive
tokens. It preserves whitespace, order, escapes, numeric spelling and line
endings; unfamiliar or nested records remain literal. It does not normalize JSON
or execute supplied code. The dictionary family loads a genuinely trained zstd
dictionary, distinct from the existing whole-base raw-prefix codec.

Artifacts are at most 64 KiB and targets at most 16 MiB. An artifact ID identifies
exact bytes and a decoder version; it is not authorization. Both senders include
candidate encoding and hashing in elapsed time, and choose Raw before Begin on
ties or expansion. Their explicit profiles do not automatically compare against
all other codecs. A missing artifact fails without a hidden retry. The receiver
reserves input/output/decoder resources, verifies length and the whole-object
digest, and publishes durably through the existing path. No new CLI mode is
implied; these are library APIs and the `schema_fitting` comparison example.
See the [format and bounds](../../docs/specs/2026-10-30-schema-fitting-design.md)
and [registered measurement contract](../../docs/network/evidence/2026-10-30-schema-fitting/measurement-contract.md).

Reproduce the fixed, ten-trial local comparison with fresh output paths:

```sh
cargo build --release --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --example schema_fitting
SCHEMA=crates/spiralcore-transfer/target/release/examples/schema_fitting
"$SCHEMA" plan /tmp/spiralcore-schema-plan.json
"$SCHEMA" run /tmp/spiralcore-schema-plan.json "$(git rev-parse HEAD)" \
  /tmp/spiralcore-schema-results.jsonl /tmp/spiralcore-schema-cache /tmp/spiralcore-schema-artifacts
"$SCHEMA" summarize /tmp/spiralcore-schema-results.jsonl /tmp/spiralcore-schema-summary.json
```

Adjust `SCHEMA` if using `CARGO_TARGET_DIR` or an explicit Cargo target triple.
The plan embeds source and fixture hashes; the run checks it before fitting.
This runs actual serialized client/receiver operations and durable commits,
then closes/reopens each store for exact byte comparison. It reports both
directions' protocol-body-equivalent bytes and process CPU, including fit and
artifact seeding separately. It uses no network sockets and measures no WAN
send time. Successful temporary stores are removed; failures retain their
observations and store. Artifacts and compact measurements remain reproducible.

## Shared ternary, Kuramoto and cellular-automaton replay

`shared_runtime` ports the coupled finite core of the retained Python runtime:
the observed ternary folding order, phase-sum memory feedback, 16 Kuramoto
oscillators, and six periodic CA rows for rules 184, 110 and 30. A canonical
checkpoint carries the state required to continue it. A bounded recipe specifies
the step count, payload and either a final checkpoint or a full state trace.
Logical time advances from the checkpoint; synchronized physical clocks are not
required for reconstruction. The wire decoder accepts data and fixed operations,
never Python or saved-card source.

`client::transfer_runtime_batched(checkpoint, target, recipe, transfer_id,
warm_probe, exchange)` adds this replay mechanism to the byte-first selector.
The receiver first needs the checkpoint as an ordinary authorized, verified
object. When replay wins, the sender sends the checkpoint reference, a 24-byte
recipe, a prediction digest and exact COPY/LITERAL corrections. Protocol framing
and acknowledgments add to that recipe size. The receiver regenerates the
prediction, applies the residual, verifies the target's exact length and digest,
and publishes through the same durable commit path. The returned selection
includes the ordinary raw/zstd/sparse/prefix alternatives. Evaluating a losing
runtime candidate still costs CPU and is included in transfer timing.

This profile reproduces the specified Rust state bytes. Python floating-point
comparisons use a documented tolerance; original Python cards, wall-clock
timestamps and UI state are outside this checkpoint format. Reference and
canonical-byte tests keep those claims separate. See the
[runtime/wire contract](../../docs/specs/2026-10-30-shared-runtime-design.md).

The Rust example runs reproducible comparisons through serialized protocol
batches, then reopens the cache and compares every byte. From the repository root:

```sh
cargo build --release --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --example shared_runtime
RUNTIME=crates/spiralcore-transfer/target/release/examples/shared_runtime
"$RUNTIME" plan --source "$(git rev-parse HEAD)" --run-id local-runtime-demo \
  --widths 48 --steps 243 --cases generated,edited,unrelated --trials 1 \
  --transport local-mechanism --out /tmp/spiralcore-runtime-plan.json
"$RUNTIME" local --plan /tmp/spiralcore-runtime-plan.json \
  --cache /tmp/spiralcore-runtime-cache --out /tmp/spiralcore-runtime-results.jsonl
```

All three output paths must be new. If using `CARGO_TARGET_DIR`, adjust `RUNTIME`
to that target directory. This local comparison reports protocol-body-equivalent
bytes and combined sender/receiver CPU; it is not a WAN measurement. The same
example supports `remote` through the allowlisted HTTPS experiment adapter and
`verify` against a stopped receiver. The
[measurement contract](../../docs/network/evidence/2026-10-30-shared-runtime/measurement-contract.md)
defines scope, controls, seeding and interpretation.

For repeated related targets, construct
`shared_transfer::PreparedPrediction::new(&checkpoint, &recipe)` once and call
`client::transfer_prepared_runtime_batched(&prepared, &target, transfer_id,
warm_probe, exchange)` for each target. This reuses the sender's immutable
prediction and setup digest. Charge `prepared.retained_bytes()` to the caller's
memory budget and measure construction separately from repeated sends; it can
retain a full 16 MiB prediction. The receiver still loads the authorized
checkpoint, replays, verifies, and durably commits each target. This API does not
create a receiver cache or bypass namespace/epoch checks.

Replay now evolves the six CA rings in packed words and emits their canonical
bytes directly. The public scalar `State::step` and wire format remain compatible.

The Rust API also exposes `client::transfer_zstd_prefix_batched` as a conventional delta control.
It encodes one bounded zstd frame against the authorized exact base, and selects raw before Begin
when the complete object-protocol encoding would not be smaller. Both inputs remain capped at
16 MiB. This mode is available through the API and the broad16 benchmark; the loopback CLI has no
prefix-mode flag. See the [versioned prefix contract](../../docs/specs/zstd-prefix-v1.md).

`Endpoint::open_with_policy(..., ColdGraphPolicy::Disabled)` disables optional Raw/Zstd DAG warming
for a minimal byte-publication comparison. Existing `Endpoint::open` preserves eager bounded
warming. The receiver chooses this immutable policy; a remote sender cannot change it. Hash,
authorization, quota and durable-publication checks apply under either policy. New broad16 plans
bind the disabled policy equally across raw, ordinary zstd, zstd prefix and sparse methods.

The receiver retains at most 64 MiB/32 verified graphs inside its existing resident-memory budget.
Active attempts pin their bases; unpinned graphs are evictable, and restart rebuilds them from verified
durable bytes. Every base use still checks namespace authorization and on-disk integrity.

`--sessions N` stops the serial server after N accepted connections, including rejected connections.
Omit it to keep serving until interrupted. Each connection may carry multiple request/response frames.
The cache has one exclusive owner, so stop the server before using `get`:

```sh
# Copy the 64-character object value from the changed-file transfer's JSON result.
"$BIN" get --cache /tmp/spiralcore-session/cache --object OBJECT_HEX --out ./recovered.bin
cmp ./changed.bin ./recovered.bin
```

`get` verifies the cached object through the endpoint and creates a new output file. It refuses to
overwrite an existing output path. Do not edit cache files or remove its ownership metadata manually.

## Performance objective

The primary objective is fewer bytes transmitted with exact reconstruction. Additional encode/decode computation at either endpoint is acceptable. CPU, memory and elapsed time are reported as tradeoffs; a loopback latency increase does not invalidate a measured byte saving. Compare complete traffic against the same transport and security baseline before making broader claims.

## What the measurements mean

Every successful `send` and each demo transfer prints:

| Field | Meaning |
|---|---|
| `object`, `object_bytes`, `mode` | Exact object identity, original byte count, and selected representation |
| `request_bytes`, `response_bytes` | Complete encoded application frames in each direction, including protocol headers |
| `socket_request_bytes`, `socket_response_bytes` | Bytes written/read by this TCP wrapper, including each four-byte length prefix |
| `socket_frame_bytes` | Sum of both socket-frame directions |
| `data_bytes`, `records_sent` | Data-body accounting and DAG records sent; neither is total network traffic |
| `requests` | Number of paired application requests/responses |
| `elapsed_micros` | Client transfer duration, including offer, encoding, exchanges and receiver commit response |

The socket counts exclude TCP/IP headers, ACKs, retransmissions and operating-system traffic. Timing
excludes input-file reads, prebuilding the selected base graph, and connecting the socket. Cache
seeding and base preparation are real costs and must be accounted for in a broader benchmark. These
single-host measurements demonstrate exact cold/warm behavior; they do not establish WAN speedups or
compare against conventional CAS, rsync/VCDIFF, or a secured transport.

Transport framing is `u32` little-endian application-frame length followed by at most 65,536 bytes.
Lengths are checked before allocating the body. Partial lengths/bodies are errors; EOF is clean only
between frames. Connect, complete frame reads, and complete frame writes have 30-second deadlines.
Only literal loopback IPv4/IPv6 addresses are accepted for bind and dial; wildcard/LAN addresses and
DNS names are rejected. Error replies use kind 127 with a two-byte code and echo the request's transfer
identity and sequence. Malformed outer frames are closed without a fabricated response identity.

## Verify locally

```sh
cargo test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets
cargo clippy --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets -- -D warnings
```

CLI integration tests use real loopback sockets and separate processes for raw/zstd sends, repeated
and changed objects, exclusive cache ownership, exact reads after restart, malformed/truncated frames,
oversized inputs, address restrictions, and the 1 MiB demonstration. The protocol and endpoint have
their own adversarial tests. The legacy UOROBJ01 implementation remains in the separate
`uor-r4-object-store` crate; it has a different compatibility wire format and 12-byte node identifiers.
