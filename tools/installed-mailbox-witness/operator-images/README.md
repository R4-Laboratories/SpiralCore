# Build sealed role images for the addressed mailbox witness

This Linux host-side fixture accepts one digest-pinned static Rust executable per role. The operator selects source A or destination B. MVM's OCI injector strips image-supplied controls, installs the verified runtime's OCI wrapper and selects the role executable. The builder then serializes the guest's typed `RuntimeConfig` for one serial worker. The native MVM ext4 writer builds the read-only filesystem and both dm-verity sidecars. The output passed to the SDK is the **file** `a/image/rootfs.ext4` or `b/image/rootfs.ext4`. A directory takes the SDK's unpack-and-inject path.

Use Linux x86_64, the matching Firecracker guest runtime, the two digest-verified static workload binaries and a fresh private output directory. This builder consumes no mailbox credentials and starts no guests. [Qualified MVM build API](https://github.com/Casey-allard/mvm/commit/5ae2cdc4977e12a574a5baa2e4f0522c3f7a1ae7) · [Guest workload inputs](../../../docs/network/evidence/2026-11-12-paired-mailbox/guest-images/README.md)

The following example uses a disposable checkout of the pinned MVM source and its existing Cargo lockfile. Keep this build checkout separate from the source used to produce the guest runtime archive.

```sh
# FIXTURE is the absolute path to this directory in SpiralCore.
# MVM_BUILD is an independent checkout of the MVM revision linked above.
install -m 0644 "$FIXTURE/prepare_addressed_rootfs.rs" \
  "$MVM_BUILD/crates/mvm-build/examples/spiralcore-addressed-rootfs.rs"
cargo build --locked --manifest-path "$MVM_BUILD/Cargo.toml" \
  -p mvm-build --example spiralcore-addressed-rootfs
builder="${CARGO_TARGET_DIR:-$MVM_BUILD/target}/debug/examples/spiralcore-addressed-rootfs"
```

Set `RUNTIME` to the verified output JSON from the guest runtime preparer, `INPUTS` to the directory containing `guest-inputs.sha256` and the two executables, and `OUT` to an existing private directory with no `a` or `b` subdirectory. Obtain the trusted input digest manifest independently. A digest copied from an untrusted executable is insufficient.

```sh
set -eu
umask 077
runtime_bin="$(jq -er .runtime_root "$RUNTIME")/x86_64/bin"
wrapper_sha="$(jq -er '.runtime_manifest.files["x86_64/bin/mvm-oci-entrypoint"]' "$RUNTIME")"
for role in a b; do
  case "$role" in
    a) program=spiralcore-mailbox-producer ;;
    b) program=spiralcore-cast-recipient ;;
  esac
  input="$role/usr/local/bin/$program"
  sha="$(awk -v name="$input" '$2 == name {print $1}' "$INPUTS/guest-inputs.sha256")"
  "$builder" "$role" "$INPUTS/$input" "$sha" "$runtime_bin" "$wrapper_sha" "$OUT/$role"
done
python3 "$FIXTURE/verify_operator_images.py" "$OUT" "$RUNTIME" "$INPUTS/guest-inputs.sha256"
```

The readback uses `debugfs` from e2fsprogs and Python’s standard library. It checks each executable, OCI marker and argv, warm-worker limits, grant requirement and the absence of the other role. It recomputes the dm-verity tree from the completed ext4 image and compares every byte with the published tree and root. Supplying Python's optimization option fails before validation. [Current artifact hashes and build costs](../../../docs/network/evidence/2026-11-12-paired-mailbox/operator-image-service-access/README.md)

## Check access under the guest service identity

For each role, use the executable and wrapper digests from the same trusted inputs as the build. The probe requires Linux with `sudo`, `unshare`, `mount`, `setpriv`, and Python 3. The image and mount point must be in a trusted operator-owned directory. The mount is private, read-only, and disables journal replay. Only the mount setup uses privilege. The readback drops to UID/GID 1000 with no groups or capabilities and checks every required inode before reading the files. It checks the kernel's effective execute permission for both ELF files.

```sh
mkdir -m 0700 "$OUT/mountpoint"
for role in a b; do
  case "$role" in
    a) program=spiralcore-mailbox-producer ;;
    b) program=spiralcore-cast-recipient ;;
  esac
  input="$role/usr/local/bin/$program"
  sha="$(awk -v name="$input" '$2 == name {print $1}' "$INPUTS/guest-inputs.sha256")"
  bash "$FIXTURE/check-ro-service.sh" "$OUT/$role/image/rootfs.ext4" \
    "$OUT/mountpoint" "$role" "$sha" "$wrapper_sha" "$FIXTURE/verify_service_access.py"
done
rmdir "$OUT/mountpoint"
```

If the local operator has a different UID, give the probe UID traversal to the dedicated mount point by using a separate trusted mount path, while keeping source images and operator evidence in their private directories. The observed witness operator and the service probe both had UID 1000 on the Linux test host.
