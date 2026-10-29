# SpiralCore exact object transfer

This standalone Rust prototype transfers exact byte objects into a persistent endpoint cache. A cold
send uses raw bytes or bounded zstd chunks. A warm send uses a caller-selected base object's ordered
32-byte ternary DAG and transfers new records. Whole-object identity includes the exact byte length
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
compression does not make that chunk smaller. `--mode` and `--base` are mutually exclusive. The
default cold mode is raw. Files are read once into a buffer with a 64 MiB ceiling and a bounded extra
byte check; the bytes used for identity are the bytes transferred.

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
