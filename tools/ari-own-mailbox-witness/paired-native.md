# Native B-addressed journal qualification

The fixture uses a fresh, private MVM home on one Linux host. It creates two persistent machine definitions, binds each to a separately measured root image, and grants one vCPU and 256 MiB per guest. Each image runs its own single process worker. Admission for A depends on the original executing B, B's signed source-directed Send, B's boot-enrolled Receive, A's own signed source scope and the operator-authorized 30-second retention ceiling. Co-location does not supply a credential.

The host gives the 313-byte SpiralCore frame only to A. A checks the fixed frame digest, sends the authenticated direction probe to its local agent, reserves the original outbox operation, and returns only the staged message ID plus its counters. The host forwards the original sealed payload across the acquired A and B helpers. Its observation of the signed B V2 binds the address, execution epoch, time window, ciphertext commitment, delivery ID and original staged message ID. A returning `staged` does not acknowledge B. Positive transport and application results are recorded separately.

B's later SDK Call supplies only the correlated message ID. B uses its own reader, compares the re-encoded frame to its expected SHA-256, syncs the recovered body to a new guest file and reopens it, then explicitly acknowledges its original lease. To show byte-for-byte equality independently, B returns the readback on the SDK application control channel. The host compares it against the original fixture file. This additional output is qualification traffic; raw control-channel and physical-network totals remain unmeasured. The host distinguishes relay endpoint I/O, guest reader I/O and the optional post-mux per-helper socket counters.

Each acquired original helper stays in the orchestrator's custody until an exit is observed. Assisted termination, retry after uncertain rollback, loss of a positive application result and partial meter output remain explicit in the record. Recovery never adopts a process discovered from a pathname or standalone PID. Closing an application or SDK socket does not establish durable publication, end-application processing or the final original helper exit.

The retained Linux helper-only result used two real service executables against synthetic guest sockets. The single-guest result used a real Firecracker machine and its own mailbox. Neither result is a run of this cross-guest driver. No matched zstd timing, multi-host WAN, Windows, macOS or FIPS qualification follows from a same-host run.

## Reproducing the single Linux sample

Use an isolated Linux x86-64 KVM host and a new private MVM home with a workload kernel and the operator retention ceiling set to 30,000 ms. The native binaries use MVM 0.28.0-rc.1 with the [guest runtime revision](https://github.com/Casey-allard/mvm/tree/93b3b65ae72a4746a54a21c7c158d5453cceb8f5) and the [host SDK and helper revision](https://github.com/Casey-allard/mvm/tree/7a1aa5bc5f92e7530e3939d0b3917ab9491cd207). Use Rust 1.97.1, Zig 0.13.0 and cargo-zigbuild 0.23.0. The locked crates resolve sibling paths.

| Relative working directory | Contents |
|---|---|
| `spiralcore` | This repository |
| `source` | Guest MVM checkout |
| `paired-guest` | Copy of `spiralcore/tools/ari-own-mailbox-witness/paired-guest` |
| `host/source` | Host MVM checkout |
| `host/witness` | Copy of `spiralcore/tools/ari-own-mailbox-witness` |

From this layout build the guest binaries, host SDK witness and original mailbox executables.

```sh
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0
cargo +1.97.1 zigbuild --locked --manifest-path paired-guest/Cargo.toml \
  --release --target x86_64-unknown-linux-musl --bins
cargo +1.97.1 build --locked --manifest-path host/witness/Cargo.toml \
  --features paired-journal-qualification --bin paired-journal --bin image --bin prepare
cargo +1.97.1 build --locked --manifest-path host/source/Cargo.toml \
  -p mvm-hostd --features durable-guest-io-witness --bin mvm-mailbox \
  --bin mvm-host-agent --bin mvm-network-endpoint --bin mvm-signer-helper
cargo +1.97.1 build --locked --manifest-path host/source/Cargo.toml -p mvmctl --bin mvmctl
```

Build the canonical original guest overlay with `prepare "$(realpath source)"` under the fresh MVM home using `MVM_RUNTIME_OVERLAY_ACQUIRE_MODE=build` and `MVM_RUNTIME_OVERLAY_SOURCE_ROOT` set to the original guest source path. For each role, call `image ROLE EXECUTABLE EXECUTABLE_SHA256 RUNTIME_BIN_DIR OCI_ENTRYPOINT_SHA256 NEW_OUTPUT_DIR`. The roles are `journal-a` and `journal-b`. Use the separately built source and destination workload executables and measure the original OCI entrypoint from the prepared runtime archive. Confirm the worker extracted from the resulting ext4 image matches its measured executable.

Use that same home and original runtime for the launch. Set `MVM_FIRECRACKER_BIN`, `MVM_MAILBOX_PATH` and `MVM_SUBSTITUTION_ENDPOINT_PATH` to the respective measured binaries, put their matched siblings on PATH and set `MVM_FC_IDLE_POOL=0`. Invoke `paired-journal A_EXT4 A_EXT4_SHA256 B_EXT4 B_EXT4_SHA256 fixtures/workload-frame.bin` from the witness directory. Preserve the pre-stop result, per-original child outcomes and measured socket records even when the harness returns failure. [Observed native measurement](../../docs/network/evidence/2026-11-12-paired-mailbox/paired-durable-native/README.md).

## Qualifying a lost first receipt

Use the same isolated two-guest setup and 313-byte frame. Pin the host and its installed service binaries to MVM `27038e3f322e4336df180073e12cae59cc4dba54` and the guest runtime to `93b3b65ae72a4746a54a21c7c158d5453cceb8f5`. Set aside the original host service binaries for installation **before** applying the relay fixture. Build the orchestrator against a *separate private checkout* of that exact host source. Apply the following overlay only to the library linked into the test orchestrator. The workload feature adds read-only probes for A’s retired outbox and B’s queue after ACK.

```sh
git -C host/source apply --check ../witness/receipt-loss/driver-only.patch
git -C host/source apply ../witness/receipt-loss/driver-only.patch
cargo +1.97.1 build --locked --manifest-path host/witness/Cargo.toml \
  --features receipt-loss-qualification --bin paired-journal
cargo +1.97.1 test --locked --manifest-path host/witness/Cargo.toml \
  --features receipt-loss-qualification --bin paired-journal receipt_retry::tests::
cargo +1.97.1 zigbuild --locked --manifest-path paired-guest/Cargo.toml \
  --features receipt-loss-qualification --release --target x86_64-unknown-linux-musl --bins
```

Rebuild the two sealed images using those workload executables and the original guest runtime. Start the same `paired-journal` command with a fresh private home, original service binaries, distinct original VM and helper owners, and verified fixture. The driver fails if the first signed receipt reaches A, the second differs, the original envelope changes, A archives the operation, B returns a second lease, or the empty probe falls outside retention. Retain partial/failed results and require both original helper exits and complete meter footers for this sample. [Native recovery measurements](../../docs/network/evidence/2026-11-12-paired-mailbox/paired-durable-native/receipt-loss.md).
