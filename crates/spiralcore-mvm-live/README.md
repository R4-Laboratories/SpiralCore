# Native MVM/vsock integration witness

This Rust program runs SpiralCore's existing lossless object protocol inside real
ARM64 Linux guests using MVM's native macOS Hypervisor.framework backend. It is an
explicit development binding for the canonical MVM mailbox envelope, with one
persistent receiver per session. It does not activate MVM's production mailbox
service, replace FlowMux, or implement an Internet router.

The `one` stage sends from one guest to a host endpoint. The `two` stage boots two
guests in separate HVF worker processes and forwards encrypted envelopes through
a host relay. The receiver commits to a private ext4 disk. After it stops cleanly,
a new guest mounts that disk read-only and verifies exact bytes with no mailbox
credentials. That historical readback is distinct from authorization to resume a
new execution epoch.

The retained MVM patch is pinned to
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b`; see `mvm-patch/` for preparation and
provenance. Port **5257 is experimental**. The production supervisor continues to
leave this binding disabled. The existing policy-controlled FlowMux port 5253 is
unchanged. No guest networking interface or general TCP/IP bridge is installed.

## Build and run

Use Rust 1.97.1 and an ARM64 macOS host with `kern.hv_support=1`. Prepare the pinned
MVM patch in an isolated source tree, supplying its generated Cargo config on
every command. The crate lockfile belongs to this extracted witness; do not
replace the standalone adapter's compatibility lockfile.

Build the host in release mode, then sign this owned executable with
`scripts/hvf-entitlements.plist` using an ad-hoc signature:

```sh
cargo +1.97.1 build --release --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml --config /absolute/path/to/mvm-patch.toml
codesign --force --sign - --entitlements crates/spiralcore-mvm-live/scripts/hvf-entitlements.plist /absolute/path/to/target/release/spiralcore-mvm-live
```

Build the same crate for `aarch64-unknown-linux-musl` using a compatible native
cross linker (the retained run uses Zig 0.13.0 and cargo-zigbuild 0.23.0). Supply a
pinned ARM64 Linux kernel supporting initramfs, virtio-vsock, virtio-blk, ext4 and
devtmpfs. The retained campaign records exact kernel and tool hashes.

```sh
spiralcore-mvm-live session NEW_PRIVATE_RESULT_DIR KERNEL_IMAGE GUEST_ELF one warm 1048576 0
spiralcore-mvm-live session ANOTHER_NEW_DIR KERNEL_IMAGE GUEST_ELF two warm 1048576 0
```

The other arms are `whole` (prepared whole-object zstd) and `raw`. Every session
uses public deterministic fixtures: seed, repeat, localized edit, unrelated
object, another local edit, truncation and append. Warm also transfers a schema
artifact, a shared-runtime checkpoint, a generated trace and its residual edit.
All arms share an uncompressed initial seed; this is recorded in setup receipts.
Run directories must not exist. Failures preserve their public receipts and owned
cache/disk for diagnosis. Remove those data files only after successful exact
verification; retain the compact JSON results.

## Security and measurement boundaries

Bootstrap signing/encryption material is generated ephemerally and passed only
through a private pipe and in-memory initramfs. The relay receives only outer
route admission fields and never decrypts envelopes. Incoming endpoint directions
contain no remote signing private key. The witness uses one outstanding call,
bounded frames, real UTC and monotonic deadlines, and conservative five-second
clock uncertainty. It refuses mismatched identity, capability, epoch, route,
correlation, causation and replay through the canonical verifier and adapter.

`sender_operation` measures encode through authenticated publication reply.
`receiver_operation` measures open, reconstruction, publication and signed reply;
framed receive, read-buffer allocation/copy and idle waiting are excluded from
receiver processing CPU. Setup contains
fixture generation and prepared codec state. Fixture ownership copies and the
independent expected-digest calculation are excluded from operation timing. Full
VM lifecycle, worker/VMM CPU, orchestrator CPU and boot/readback remain separate
fields. Do not add guest CPU to the enclosing worker CPU: that counts the same
execution twice. Guest Linux CPU accounting differs materially from native macOS
process and vCPU-thread accounting; these are separate reported counters, not
validated physical-energy measurements. The derived operation rows label their
partial sum explicitly. `planned_application_batch_bytes` comes from the planned
BatchSender frames and can include frames skipped after an early Published reply;
use actual stream counters for transferred bytes.

Stream counters include canonical encrypted envelopes, four-byte frame prefixes
and readiness, in both directions. Two-guest physical stream bytes count both
vsock hops. They exclude virtio control/credit headers and boot assets. The
inherited MVM relay allowance is 4 MiB/s with an 8 MiB burst; it is not bypassed.
Local HVF timings are not WAN timings. The single-trial registered campaign is a
functional integration pilot, not a latency-distribution or statistical speed
qualification.

The supplied demonstration uses public/synthetic data and has no FIPS validation
or CUI accreditation claim. Full upstream MVM workspace qualification, production
mailbox lifecycle, remote orchestration, Iroh, new-epoch cache migration and
arbitrary host power-loss recovery remain separate work.
