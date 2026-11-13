# Native B-addressed journal qualification

## Original sender ordinal campaign

Build the host witness and both guest workloads with `sender-ordinal-v1`. Build the installed `mvm-mailbox` with `sender-ordinal-io-witness`. Use the signed ordinal boot projection and guest runtime from [the joined MVM revision](https://github.com/Casey-allard/mvm/commit/4dcf544bed90bfc2cb90c065998a025b859ddab4).

```sh
paired-journal --backend firecracker --sender-ordinals 65 \
  "$rootfs_a" "$sha256_a" "$rootfs_b" "$sha256_b" "$fixture_frame"
```

The same admitted pair serves the entire campaign. For each ordinal, A stages a distinct 313-byte frame on its guest-local reader. B first performs an authenticated empty Receive. A then requests the original B receipt while the orchestrator services the sealed relay. B reads all 257 payload bytes back from its synced guest file and acknowledges its signed lease. The orchestrator compares all bytes, checks that A recovered the same signed receipt and requests explicit settlement of that receipt digest. The next frame starts only after A confirms the expected contiguous retired prefix.

The private `sender-ordinal-series.jsonl` contains one synced row per completed ordinal. The row records workload CPU, SDK input/output sizes, authenticated reader bytes, sealed relay bytes and elapsed time. The two original helper descriptors supply full-session port-5257 read/write totals through retirement. SDK input/output counts exclude outer framing. The empty Receive precedes forwarding each frame, so its elapsed time measures a sequential control. The driver accepts counts from 1 to 2,049. Supported capacity requires a completed run at the claimed count.

## Single transfer

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
| `fixtures` | Link to `spiralcore/tools/ari-own-mailbox-witness/fixtures` for the compiled-in workload frames |
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

Build the canonical original guest overlay with `prepare "$(realpath source)"` under the fresh MVM home using `MVM_RUNTIME_OVERLAY_ACQUIRE_MODE=build` and `MVM_RUNTIME_OVERLAY_SOURCE_ROOT` set to the original guest source path. On a Linux builder preparing an Apple Silicon guest, pass `aarch64` after the source path to select the guest target explicitly. For each role, call `image ROLE EXECUTABLE EXECUTABLE_SHA256 RUNTIME_BIN_DIR OCI_ENTRYPOINT_SHA256 NEW_OUTPUT_DIR`. The roles are `journal-a` and `journal-b`. Use the separately built source and destination workload executables and measure the original OCI entrypoint from the prepared runtime archive. Confirm the worker extracted from the resulting ext4 image matches its measured executable.

Use that same home and original runtime for the launch. Set `MVM_FIRECRACKER_BIN`, `MVM_MAILBOX_PATH` and `MVM_SUBSTITUTION_ENDPOINT_PATH` to the respective measured binaries, put their matched siblings on PATH and set `MVM_FC_IDLE_POOL=0`. Invoke `paired-journal A_EXT4 A_EXT4_SHA256 B_EXT4 B_EXT4_SHA256 fixtures/workload-frame.bin` from the witness directory. Preserve the pre-stop result, per-original child outcomes and measured socket records even when the harness returns failure. [Observed native measurement](../../docs/network/evidence/2026-11-12-paired-mailbox/paired-durable-native/README.md).

The subsequent same-host run used the same original guest image digests and rebuilt the host executables at [27038e3f](https://github.com/Casey-allard/mvm/tree/27038e3f322e4336df180073e12cae59cc4dba54). Both original helpers exited with status 0 on the first attempt. [Measured rerun](../../docs/network/evidence/2026-11-12-paired-mailbox/paired-durable-native/graceful.md).

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

## Selecting the native host backend

The paired driver accepts `--backend firecracker` on Linux x86-64 or arm64 and `--backend hvf` on Apple Silicon macOS. Omitting the flag selects the native backend for those host targets. The SDK still enforces its own backend availability and version requirements. Both persistent role definitions and both original SDK starts use the same selection. After each start, the driver reads MVM’s per-VM backend marker through `AnyBackend::started_vm_kind`. A missing or mismatched marker stops the trial and enters the original-owner cleanup path. The final result records the requested backend and the separately observed source and destination markers, including null for a role whose start never returned.

For HVF, use the existing native MVM supervisor and its verified arm64 Linux kernel, guest agent and OCI wrapper. Build both distinct guest workloads for `aarch64-unknown-linux-musl` and rebuild their sealed role images with the same `image` program. Keep the signed 30-second mailbox ceiling, original service executables and private MVM home. Invoke `paired-journal --backend hvf A_EXT4 A_SHA256 B_EXT4 B_SHA256 fixtures/workload-frame.bin`. The Linux sample above used Firecracker. Each platform’s native delivery and lifecycle results are tracked separately in [#46](https://github.com/R4-Laboratories/SpiralCore/issues/46).

### 4 KiB fixture with interleaved control

Use the same paired build layout, host service binaries, 30-second retention ceiling and verified runtime. Build the host driver with features `paired-journal-qualification,large-object-fixture` and the two guest workloads with `large-object-fixture`. Prepare the authority and runtime in a fresh private MVM home. Seal each workload with its corresponding `series-a` or `series-b` role. This selects the known 4,152-byte frame at both workloads and in the source's signed outbound direction. Verify `fixtures/sha256.txt` before running.

```sh
paired-journal --backend firecracker --objects 3 \
  "$source_ext4" "$source_sha256" "$destination_ext4" "$destination_sha256" \
  fixtures/workload-4k-frame.bin
```

For the comparison, build both workloads and the driver with the large-object feature disabled, use a fresh private installation and the original `fixtures/workload-frame.bin`. Keep the original per-object reports, both port-5257 counters, teardown statuses and the audit verification. [Recorded size comparison](../../docs/network/evidence/2026-11-12-paired-mailbox/paired-durable-native/size-control/README.md)

### Application Call and Reply on the selected object pair

The `object-reader-v1` guest workload uses the canonical local reader and the existing Object-v1 sender. With `large-object-fixture` it accepts up to 4,096 input bytes. The source worker accepts a bounded `object_send` operation containing the 32-byte pair commitment, a caller-retained unique 32-byte transfer ID, and the original byte string. Encode these as lowercase hex in fields `pair`, `transfer`, and `payload_hex`. The caller must retain the same logical ID and input across an uncertain submission. Each Call sequence derives a stable message ID and a separate correlation ID in the selected pair.

The worker stages each Call, asks the original agent for its destination admission status, then reads B’s correlated application Reply. It validates the object response before issuing the original reader ACK. A lost ACK response retains one original lease in that live worker. An explicit `object_settle` operation takes `pair`, `transfer`, and integer `seq` and retries that exact ACK. Its success confirms only that lease's settlement. Worker process restart and full transfer resumption require separate recovery.

On B, send `{"object_receive":{"pair":"<64 hex>","transfer":"<64 hex>","seq":0}}` for the first Offer. For each subsequent frame send its original sequence. Each successful response includes the same `receiver_instance` (32 hex characters) and authenticated `receiver_boot` (64 hex characters) through the live worker. B rechecks the exact enrolled Call at cache operations, retains its computed Reply across transient backpressure, waits for the original V2 and then ACKs the Call. `readback_hex` contains the full verified object only on publication or an exact-object cache hit. A repeated worker request after a lost successful stdout confirms the historical ACK. To obtain protected readback again, use a new authorized transfer. [Destination worker and incomplete-return control](../../docs/network/evidence/2026-11-12-paired-mailbox/paired-durable-native/object-guest-reader/destination-worker.md).

Build the Linux object reader with the matched MVM contributor revision at `source` and a copy of this checkout’s `paired-guest` beside `source` and `spiralcore`. The recipient supplies its original boot, selected pair, signed lease, header and exact payload to the verifier. Each authorization check verifies the fresh challenge and current time against that same delivery. [Matched MVM source](https://github.com/Casey-allard/mvm/commit/efaffa969161d7101aef135e70046a7220eca970)

```sh
cargo +1.97.1 test --locked --manifest-path paired-guest/Cargo.toml \
  --features object-reader-v1,large-object-fixture --all-targets
cargo +1.97.1 build --locked --manifest-path paired-guest/Cargo.toml \
  --features object-reader-v1,large-object-fixture --bins
```

The predecessor Linux x86-64 sender component passed eight library checks and strict Clippy and built both worker binaries. Two tests exchange the real typed reader messages on AF_UNIX socket pairs with fixture peers. They exercise matching staging, pending and terminal responses, including wrong ID, V1, refusal and ambiguous settlement controls. These tests do not boot guests or establish WAN or compression speed. A direct macOS build of the Linux workload encountered Linux-only socket flags. Apple Silicon host acceptance uses a Linux guest built for arm64.

### Measuring an installed cold object and its exact repeat

With the original pair owner running, point `MVM_HOME` at its existing private installation and supply the lowercase SHA-256 commitment from its selected signed pair. Build the host witness and both guest workloads with matching fixture sizes. The default is 257 bytes. The `large-object-fixture` build uses 4,096 bytes. The installed owner must service both the admitted Call and Reply throughout both transfers. Start with a fresh receiver cache.

```sh
MVM_HOME="$qualification_home" paired-journal object-repeat "$pair_sha256"
```

The driver submits the fixture only to `sc-journal-a`. It asks `sc-journal-b` to consume each original Call by pair, transfer and sequence. B must ACK each Call and return the complete object on the terminal operation of both transfers. The driver compares all bytes, checks both original boot bindings between phases and requires the same B worker throughout. The cold phase must send the full literal. The repeat must finish in one Call with zero literal data and fewer application bytes in both directions combined. Fresh transfer IDs go to stderr before either submission. Each completed phase logs its measurements before the savings check.

The final JSON gives the Object-v1 request and reply bytes, per-role reader I/O and process CPU, serialized workload stdin and stdout including the readback, and elapsed time through both guest results. These stream counts exclude outer SDK framing. Coordinator CPU excludes its SDK subprocesses. The application total excludes envelopes and vsock framing. Collect the original port-5257 counters, sealed relay counters, and retirement outcomes from the running owner separately. The driver leaves host relay and network wire byte totals null. The qualification command requires a running installed pair. Its component checks alone establish no transfer-time or WAN result.

In an independently created, owner-only empty qualification home, `paired-journal object-enrollment-check` checks the four original Call, Reply and Receive credentials before launching any VM. It requires the 30-second operator retention ceiling and refuses an existing mailbox template. The installed orchestrator must use the in-process preparation once and retain the exact nonces, grants and image digests through signing and registration. A second preflight cannot recreate the original signed pair. [Linux role image and grant checks](../../docs/network/evidence/2026-11-12-paired-mailbox/paired-durable-native/object-role-images.md)

## Controlled helper restart before readback

The Linux controller can run one ordinal with `--restart-helpers --sender-ordinals 1`. Build the host witness against the MVM SDK that provides `OriginalJournalSourceHost::restart_helper`. The guest workload and sealed runtime retain their admitted ordinal profile.

```sh
paired-journal --backend firecracker --restart-helpers --sender-ordinals 1 \
  "$rootfs_a" "$sha256_a" "$rootfs_b" "$sha256_b" "$frame"
```

After A verifies B's original acceptance and before B reads or acknowledges the delivery, the controller interrupts and replaces A's helper, then B's helper. Each returned child status must identify SIGKILL. Both guest VMs keep running. A must recover the same accepted message ID and receipt digest through its restarted helper. B then reads the original 257 bytes from its restarted journal and acknowledges its original lease. A settles that original receipt. The normal ordinal result adds a `helper_recovery` object with restart intervals, original exit signals and the additional A status operation's byte and CPU counters. The ordinary campaign keeps its existing result fields.

This mode measures recovery after acceptance and before guest readback and settlement. The initial signed receipt has already reached A. The separate lost-receipt witness exercises loss before that confirmation. Controlled restart refuses on other host platforms before interrupting a helper.
