# One installed mailbox on Linux/Firecracker

This witness tests whether one Linux guest can deliver a deterministic 1,024-byte message through MVM's canonical outbox. A pass requires the installed host service to return every byte exactly. The run also exercises signed Stop, collection of the original helper process, and retirement of the host coordinator. The guest requests one vCPU, 256 MiB RAM, and a 180-second TTL. The result applies to one host and one guest. [Recorded attempts and results](../../docs/network/evidence/2026-11-11-retained-recovery/installed-helper/README.md)

## Prepare the sources and assets

Use an x86-64 Linux host with usable KVM, Firecracker, a user systemd session, the MVM kernel, and the build tools required by the pinned MVM revision (including Cargo, Zig and cargo-zigbuild). Run as the test account with access to KVM. Use a fresh private directory with a short absolute path for Unix sockets. The runtime build needs several GiB of scratch space. The three crates share one sibling MVM source checkout. Use the standalone preparer in the commands below.

From the SpiralCore repository root, in Bash:

```bash
set -eu
umask 077
scratch=$(mktemp -d /var/tmp/sc.XXXXXXXX)
cp -R tools/installed-mailbox-witness/. "$scratch/"
cd "$scratch"
sha256sum --check inputs.sha256
revision=a23e0b54d7c1bd2ea7976a57a77c6b5b60205fb2
git init -q source
git -C source fetch --depth=1 https://github.com/Casey-allard/mvm.git "$revision"
git -C source checkout -q --detach FETCH_HEAD
test "$(git -C source rev-parse HEAD)" = "$revision"
export MVM_HOME="$scratch/home" CARGO_TARGET_DIR="$scratch/target"
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2
mkdir -m 700 install results
test ! -e "$MVM_HOME"  # the preparer provisions a new private home
```

Keep the Git metadata. MVM uses the checkout revision when building its guest runtime. Supply the absolute paths to the trusted Firecracker binary and kernel through `MVM_FIRECRACKER_BIN` and `WORKLOAD_KERNEL`. The [pinned asset downloader](../linux-mvm/prepare-assets.sh) supplies the kernel and Firecracker. This run used the MVM image-set v0.2.4 workload kernel (SHA-256 `1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6`) and the Firecracker v1.17.0 executable (SHA-256 `99ad0f5cd0514a88aad0e9ae8cfdb3cc3b4ab9d190e1194602406c786b5de7a5`).

The [workload fixture](make-guest-rootfs.sh) creates the guest directory structure and four public configuration files, then installs the newly built producer and the OCI entrypoint from the prepared MVM runtime.

## Build and run

```bash
cargo build --release --locked --manifest-path preparer/Cargo.toml
"$CARGO_TARGET_DIR/release/spiralcore-prepare-installed-guest" \
    "$scratch/source" "$MVM_HOME" "$scratch/results/runtime.json"
cargo build --release --locked --manifest-path installed-witness/Cargo.toml \
    --bin spiralcore-mvm-canonical-witness
cargo build --release --locked --manifest-path source/Cargo.toml -p mvm-hostd \
    --bin mvm-host-agent --bin mvm-mailbox --bin mvm-signer-helper
for service in mvm-host-agent mvm-mailbox mvm-signer-helper; do
    install -m 500 "$CARGO_TARGET_DIR/release/$service" "install/$service"
done

# Reuse the canonical guest runtime's target directory to avoid a duplicate build.
guest_targets=()
for dir in "$MVM_HOME"/cache/guest-agent-build/targets/*; do
    if [[ ${dir##*/} =~ ^[0-9a-f]{16}$ && -d "$dir/x86_64-unknown-linux-musl/release" ]]; then
        guest_targets+=("$dir")
    fi
done
test "${#guest_targets[@]}" -eq 1 || exit 1
CARGO_TARGET_DIR="${guest_targets[0]}" cargo zigbuild --release --locked \
    --manifest-path guest-producer/Cargo.toml --target x86_64-unknown-linux-musl
runtime_tree=$(jq -er '.runtime_root' results/runtime.json)
bash make-guest-rootfs.sh "$scratch/rootfs" "$runtime_tree/x86_64/bin/mvm-oci-entrypoint" \
    "${guest_targets[0]}/x86_64-unknown-linux-musl/release/spiralcore-mailbox-producer"
kernel_dir="$MVM_HOME/cache/kernels/x86_64/workload"
mkdir -p "$kernel_dir"
install -m 600 "$WORKLOAD_KERNEL" "$kernel_dir/vmlinux"
sha256sum "$kernel_dir/vmlinux" | cut -d ' ' -f 1 > "$kernel_dir/vmlinux.sha256"

export MVM_MAILBOX_PATH="$scratch/install/mvm-mailbox"
export MVM_RUNTIME_OVERLAY_ACQUIRE_MODE=build MVM_RUNTIME_OVERLAY_SOURCE_ROOT="$scratch/source"
: "${MVM_FIRECRACKER_BIN:?set the absolute path to the verified Firecracker binary}"
export MVM_FIRECRACKER_BIN
"$CARGO_TARGET_DIR/release/spiralcore-mvm-canonical-witness" \
    "$scratch/rootfs" spiralcore-installed-028 \
    0101010101010101010101010101010101010101010101010101010101010101 \
    "$scratch/results/native.json"
jq -e '.status == "passed" and .exact_readback == true and
       .received_bytes == 1024 and .expected_sha256 == .received_sha256' results/native.json
```

The mailbox ID above is a synthetic fixture. Each invocation needs a fresh VM namespace and an unused report path outside the private MVM home. On failure, preserve the report and inspect the original process and service state before disposing of the test directory. In this revision a failure attaching the SDK provider after daemon startup, or a timeout waiting for idle retirement, can leave the daemon alive. Only count retirement when the runner records it.

## Read the measurements

`runtime.json` measures runtime preparation. `native.json` distinguishes setup, time spent receiving (including polling), full lifecycle wall time, and orchestrator process CPU. The payload length and digest refer to application bytes. Guest CPU and transport wire bytes have their own availability fields. This command provides a functional witness for the installed mailbox. Compression gains, sender/receiver CPU comparisons and WAN performance require a paired transfer benchmark.
