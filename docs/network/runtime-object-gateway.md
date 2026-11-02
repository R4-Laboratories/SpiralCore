# Runtime and warm-cache object gateway

`spiralcore-gateway-object` connects cooperating applications through the same
Rust gateway transport implementation using a reliable object profile. It invokes
SpiralCore's existing packed ternary/Kuramoto/CA replay or rolling warm-cache
codec, verifies reconstruction, and commits bytes to a scoped receiver store.
It also exposes Raw and prepared whole-object zstd for matched comparisons.

This is an active object interface, not a transparent transformation of every TUN
packet. The existing `spiralcore-gateway run` command still serves independent IP
packets. The MVM adapter can use the same ObjectSender facade; the canonical
mailbox remains its policy boundary.

## Run

Build from the repository root:

```sh
CARGO_TARGET_DIR="$PWD/target" cargo +1.97.1 build --locked --release \
  --manifest-path crates/spiralcore-gateway/Cargo.toml \
  --bin spiralcore-gateway-object
./target/release/spiralcore-gateway-object run receiver.json
# On the cooperating sender:
./target/release/spiralcore-gateway-object run sender.json
```

Both configurations contain `role`, `bind`, `peer`, `server_name`, `certificate`,
`private_key`, `trust_anchor`, `peer_certificate`, `scope`, `policy_epoch`,
`store_dir`, `stats_path`, `ready_path`, `duration_secs`, `groups`, `verify_files`.
Identity files use the existing [gateway DER certificate/key formats](../../crates/spiralcore-gateway/README.md).
Report and readiness paths must be fresh files in existing writable directories,
outside the store and input paths. The service reserves them before writing and
refuses existing files or aliases; choose new report paths when restarting.
The peer must have a reachable UDP address and matching certificate trust/name/pin.
The object ALPN is `spiralcore-object/1`; packet peers cannot be substituted.

A receiver uses `role:"listen"`, an owned `store_dir`, `groups:[]`, and optionally
`verify_files` naming local expected files for independent exact readback. An
empty verification list still requires every publication digest/length to verify.
A sender uses `role:"connect"`, `verify_files:[]`, and one or more groups. Both
ends must agree on scope and policy epoch; namespace binding is checked before
object traffic. No received field grants authorization. Scope/policy changes
require restart and invalidate sender readiness.

Example sender group (paths are local files, not transmitted code):

```json
{
  "label": "simulation-output",
  "mode": "runtime",
  "checkpoint_path": "/input/checkpoint.bin",
  "recipe_path": "/input/recipe.bin",
  "operations": [
    {"label":"checkpoint","action":"checkpoint","warm_probe":false},
    {"label":"output","action":"send","path":"/input/output.bin","warm_probe":true},
    {"label":"changed","action":"send","path":"/input/changed.bin","warm_probe":true}
  ]
}
```

Recipes/checkpoints use `shared_runtime::Recipe::encode` and `State::encode`.
Runtime construction pays for `PreparedPrediction` once per group. The receiver
replays verified checkpoint state and applies exact residuals. The existing
byte-first runtime policy may choose a conventional codec for unrelated data;
`selected_mode` records the actual offer. A verified cache hit does not execute
the offered decoder; `publication_path` distinguishes it from a full transfer.

A `warm` group needs `schema_path`, a `schema` setup operation, and a `seed`
operation for its first acknowledged object. Later `send` operations compare the
rolling base, schema and conventional candidates. `raw` and `whole-zstd` groups
need only `send` operations. There is one prepared sender at a time; group changes
retain the receiver store and authenticated connection. Runtime and Warm are
explicit policies, not a claim of one universal minimum across both.

## Exactness, limits and measurements

One bidirectional QUIC stream carries bounded existing object batches. A failed
or interrupted stream is not resumed mid-frame. Checkpoint/base absence and
revocation are errors, never permission to reconstruct unchecked bytes. The
receiver publishes only after validation and reports a final reopened-store
readback. Canonical data checkpoints are never executed as source code.

Limits include 16 MiB input objects, existing replay/work bounds, one prepared
group at a time, eight groups,32 operations/group,20 MiB encoded batches,512
frames/batch,1 MiB QUIC receive windows,4 MiB send window and the existing shared
Endpoint quotas. The process is intended for an explicitly paired cooperative
peer, not an unbounded public upload service.

Results separate group preparation, schema/checkpoint/base seeds, each operation,
receiver codec/commit work, final readback, QUIC setup and process lifetime. Sender
operation time includes file loading, hashing, selection and publication reply.
Receiver batch timing covers Endpoint decode/validation/commit, excluding stream
wait and batch framing decode; those remain in whole-process totals. CPU is nested
inside elapsed time, not added to it. Counter snapshots distinguish object frames,
batch envelopes, QUIC UDP payload and independently captured IP bytes.
Transfer IDs link receiver batch costs to each sender operation without estimating
which decoder ran from the offered mode alone.

`examples/object_fixtures.rs` generates deterministic trace, changed trace, exact
repeat, opaque, changed opaque and repeat fixtures. It delegates all CA evolution
to the existing runtime. Sizes are1,047,202 and14,851,384 bytes; neither is
mislabeled as an exact16 MiB trace. Runtime preparation and all seed traffic are
paid in cumulative results. Raw/zstd get the same target-cache probe opportunity.
Formal results and failures are retained under
`docs/network/evidence/2026-11-02-runtime-gateway/`.
