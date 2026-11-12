# Firecracker guest workload inputs

The sender and receiver root filesystems were prepared on Linux x86-64 for the addressed mailbox test. A contains the submit workload. B contains the recipient workload. Both use the Rust OCI entrypoint wrapper. We verified the hashes of all ten workload binaries, wrappers, entrypoint declarations and runtime configurations in the [input manifest](guest-inputs.sha256). Each directory occupied 1.4 MiB as reported by `du -sh`. The guest images contain synthetic test programs and no test payload.

All three executables were built for `x86_64-unknown-linux-musl` at [MVM source revision 60965c80](https://github.com/Casey-allard/mvm/commit/60965c80db91ca98f51eb647dc32e7dcf16495ee). All 4,548 tracked files and modes matched that revision before the build. ELF inspection confirmed static x86-64 executables with no program interpreter or dynamic-library dependencies. The Linux namespace measurements previously used dynamic executables. These static artifacts are the inputs to the Firecracker launch.

From the MVM checkout, build the workload inputs with the musl target installed. Then use the [SpiralCore root filesystem builder](../../../../../tools/installed-mailbox-witness/make-addressed-pair-rootfs.sh) on Linux. The builder takes a fresh directory followed by the wrapper, sender and receiver executables. Each side runs one serial warm process, with one call per worker in this fixture.

```sh
cargo zigbuild --locked --release --target x86_64-unknown-linux-musl \
  -p mvm-agentd --bin mvm-oci-entrypoint \
  --example original_submit_workload --example original_cast_workload
bin="${CARGO_TARGET_DIR:-target}/x86_64-unknown-linux-musl/release"
bash "$SPIRALCORE/tools/installed-mailbox-witness/make-addressed-pair-rootfs.sh" \
  "$PWD/guest-inputs" "$bin/mvm-oci-entrypoint" \
  "$bin/examples/original_submit_workload" "$bin/examples/original_cast_workload"
(cd guest-inputs && sha256sum -c guest-inputs.sha256)
```

The manifest records the actual prepared files. End-to-end qualification additionally requires a source submission, an authorized coordinator transfer and a successful independent read from the booted B workload.
