# Native MVM own-mailbox witness

These three Rust executables exercised a signed production mailbox and a normal warm-worker Call in an isolated Linux/x86-64 Firecracker guest. The checked-in 313-byte input was generated with the project's ordinary `Frame::encode` and independently decoded. The `fixtures` directory also contains its original 257-byte body, including the trailing zero. [Results and exact executable digests](../../docs/network/evidence/2026-11-12-paired-mailbox/own-mailbox-native/results.json)

Use a Linux KVM host with working user systemd, a verified Firecracker 1.17.0 executable, the upstream runtime build dependencies (including Rust's musl targets, cargo-zigbuild, Zig and ext4/verity tools), and the normal MVM guest-build toolchain. Run as the non-root VM owner with authorized KVM device access. Use a fresh mode-0700 work directory and private MVM home on every attempt. Keep compiled artifacts outside the repository.

Place this directory at `$work/prep` and check out the MVM source as the adjacent `$work/source`. The recorded third trial used the following pins.

```sh
umask 077
work=$(mktemp -d)
cp -R tools/ari-own-mailbox-witness "$work/prep"
git clone https://github.com/Casey-allard/mvm.git "$work/source"
git -C "$work/source" checkout --detach 1755a2dea880ae5b7fcdf0479ca7f9807face651
export MVM_HOME="$work/home"
mkdir -m 700 "$MVM_HOME" "$work/install" "$work/report"
export CARGO_TARGET_DIR="$work/host-build"
cargo build --locked --manifest-path "$work/prep/Cargo.toml" --bins
(cd "$work/source" && cargo build --locked -p mvm-hostd --bin mvm-mailbox --bin mvm-host-agent --bin mvm-signer-helper --bin mvm-network-endpoint)
(cd "$work/source" && cargo build --locked --bin mvmctl --features user)
for b in mvm-mailbox mvm-host-agent mvm-signer-helper mvm-network-endpoint mvmctl own-mailbox prepare image; do
  install -m 700 "$CARGO_TARGET_DIR/debug/$b" "$work/install/$b"
done
export PATH="$work/install:$PATH"
export MVM_RUNTIME_OVERLAY_ACQUIRE_MODE=build MVM_RUNTIME_OVERLAY_SOURCE_ROOT="$work/source"
mvmctl kernel build --which workload --source download
prepare "$work/source" > "$work/runtime-pieces.json"
```

Build or obtain the separately pinned static x86-64 musl `spiralcore-durable-own-mailbox` workload from SpiralCore revision `01c8ad87856b81440353fed0c07bc703b12dbebd`. See the [crate build and MVM dependency pins](../../crates/spiralcore-mvm-guest/README.md). The measured ELF digest is in the results. An independently rebuilt binary must have its own trusted digest and build provenance.

The image builder accepts the operator-selected executable and digest, plus the canonical runtime directory and its independently verified OCI entrypoint digest. It injects the official runtime and seals the ext4 filesystem. Verify the resulting image, verity file, roothash file and metadata against the operator manifest. Set `ROOT_DIGEST` to that manifest's recorded ext4 SHA-256 before launch. Keep the associated files together.

```sh
runtime_bin="$MVM_HOME/cache/guest-runtime/v1/objects/$(jq -r .guest_archive_sha256 "$work/runtime-pieces.json")/tree/x86_64/bin"
image own "$WORKLOAD_ELF" "$WORKLOAD_DIGEST" "$runtime_bin" "$OCI_WRAPPER_DIGEST" "$work/operator-image"
export MVM_FC_IDLE_POOL=0 MVM_MAILBOX_PATH="$work/install/mvm-mailbox"
export MVM_SUBSTITUTION_ENDPOINT_PATH="$work/install/mvm-network-endpoint"
export MVM_FIRECRACKER_BIN="$VERIFIED_FIRECRACKER"
(cd "$work/prep/fixtures" && sha256sum -c sha256.txt)
/usr/bin/time -f 'wall=%e user=%U sys=%S rss_kib=%M' -o "$work/report/time.txt" \
  own-mailbox "$work/operator-image/image/rootfs.ext4" "$work/prep/fixtures/workload-frame.bin" "$work/report" "$ROOT_DIGEST" \
  >"$work/report/outcome.json" 2>"$work/report/driver.log"
mvmctl trust audit verify
```

The dedicated host had no other Firecracker guest or live MVM scope at startup. Each launch explicitly selected one vCPU, 256 MiB, a signed mailbox, and one original worker. A 45-second worker timeout and a 55-second host observation limit bound the Call. The host driver attempts normal SDK stop and removal on Call refusal. Preserve the initial exit status, private failure output and journal evidence if the command fails. Check original service retirement before reusing a test host. A successful response must have the original length, digest, scoped message ID, counted reader requests and explicit ACK. Whole-host CPU, full vsock traffic and WAN timing require separate meters.

The reproduction package uses the final driver that accepts a separately preverified ext4 digest. The initial two runs used the same driver with the first image digest fixed in source. Their original executable identities are recorded separately. Host private keys, storage and raw VM logs stay on the test host.
