# Native MVM/vsock integration witness

The opt-in [Linux/Firecracker host path](../../docs/network/linux-firecracker.md)
uses the same canonical mailbox and transfer core. Build it with `linux-host`;
the default Linux executable is the guest. The following original instructions
describe the ARM64 macOS/HVF witness and its historical rate profiles.

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

The current dependency is the explicit contribution-preparation fork commit
`5766754fbaae59dbee548f968716a514d3a796c1`, based on upstream `6ae461a`.
See `mvm-service-patch/` for the reproducible patch bundle and provenance.
The historical `mvm-patch/` bundle and prior results remain pinned to
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b`; they do not qualify the new service.
Port **5257 is experimental**. The production supervisor continues to
leave this binding disabled. The existing policy-controlled FlowMux port 5253 is
unchanged. No guest networking interface or general TCP/IP bridge is installed.

## Build and run

Use Rust 1.97.1 and an ARM64 macOS host with `kern.hv_support=1`. The default
manifest and committed lockfile resolve the pinned public contribution fork;
no local patch configuration is required. The optional `mvm-service-patch/`
preparation reproduces that source in an isolated tree. The crate lockfile
belongs to this extracted witness; keep the standalone adapter's lockfile too.

Build the host in release mode, then sign this owned executable with
`scripts/hvf-entitlements.plist` using an ad-hoc signature:

```sh
cargo +1.97.1 build --release --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml
codesign --force --sign - --entitlements crates/spiralcore-mvm-live/scripts/hvf-entitlements.plist /absolute/path/to/target/release/spiralcore-mvm-live
```

Build the same crate for `aarch64-unknown-linux-musl` using a compatible native
cross linker (the retained run uses Zig 0.13.0 and cargo-zigbuild 0.23.0). Supply a
pinned ARM64 Linux kernel supporting initramfs, virtio-vsock, virtio-blk, ext4 and
devtmpfs. The retained campaign records exact kernel and tool hashes.

```sh
spiralcore-mvm-live session NEW_PRIVATE_RESULT_DIR KERNEL_IMAGE GUEST_ELF one warm 1048576 0
spiralcore-mvm-live session ANOTHER_NEW_DIR KERNEL_IMAGE GUEST_ELF two warm 1048576 0
spiralcore-mvm-live session EXPERIMENT_DIR KERNEL_IMAGE GUEST_ELF two warm 1048576 0 experimental64
```

The optional final profile is `standard` (the default, 4 MiB/s) or
`experimental64` (64 MiB/s). Both retain the 8 MiB burst, stream and idle limits.
The host applies this profile only to the experimental mailbox service and records
the selected rate in the plan, session and worker receipts. Compare every arm at
the same profile; raising a transport allowance is separate from a codec gain.
Unknown profiles are rejected before boot. Other MVM service budgets are unchanged.

The other arms are `whole` (prepared whole-object zstd) and `raw`. Every session
uses public deterministic fixtures: seed, repeat, localized edit, unrelated
object, another local edit, truncation and append. Warm also transfers a schema
artifact, a shared-runtime checkpoint, a generated trace and its residual edit.
All arms share an uncompressed initial seed; this is recorded in setup receipts.
Run directories must not exist. Failures preserve their public receipts and owned
cache/disk for diagnosis. Remove those data files only after successful exact
verification; retain the compact JSON results.

## Combined runtime and warm-cache campaign

The opt-in `combined`, `combined-cpu` and `whole-mixed` arms run the current shared
Rust object gateway core through the same canonical mailbox boundary. `combined`
uses exhaustive selection; `combined-cpu` uses the explicit CPU-conscious schema
eligibility policy. `whole-mixed` supplies the matched prepared whole-object zstd
baseline. The legacy `warm`, `whole` and `raw` campaign remains available.

For these mixed arms, request size `1048576` or `16777216` and trial `0`, `1` or
`2`. Their generated trace sizes are respectively **1,047,202** and **14,851,384**
bytes. Each arm sends the same generated trace, residual edit, exact repeat,
unrelated object, opaque edit, its repeat, schema-fitting object and schema-drift
object. Both Combined arms first publish their checkpoint and schema. These
publications and codec preparation count toward paid session costs. One receiver
and its verified store persist throughout each session.

```sh
spiralcore-mvm-live session NEW_PRIVATE_RESULT_DIR KERNEL_IMAGE GUEST_ELF one combined 1048576 0 experimental64
spiralcore-mvm-live session ANOTHER_NEW_DIR KERNEL_IMAGE GUEST_ELF two combined-cpu 16777216 0 experimental64
spiralcore-mvm-live session BASELINE_NEW_DIR KERNEL_IMAGE GUEST_ELF two whole-mixed 16777216 0 experimental64
```

Always compare arms at the same bandwidth profile. The explicit `experimental64`
profile changes the mailbox allowance, not the compression mechanism. Selection
receipts distinguish policy-skipped candidates from proven pruning; candidate
costs use the selector's existing model, while mailbox results use actual encrypted
stream counters. Preparation, sender operation, receiver processing, and complete
VM lifecycle have separate timing scopes. See the
[SC-MVM-004 design](../../docs/specs/2026-11-02-mvm-combined-design.md).

## Security and measurement boundaries

Current provisioning selects `canonical-ephemeral-v1`: each application Call
has its encrypted signed destination-admission receipt, an application Reply,
and the Reply's encrypted signed admission receipt. These four envelopes are
counted separately. Mailbox acceptance is ephemeral; the object commit and
keyless restart readback establish the separate object durability result.
Old two-envelope measurements must not be pooled with this service profile.

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
default inherited MVM relay allowance is 4 MiB/s with an 8 MiB burst. The explicit
experimental profile raises only its sustained rate to 64 MiB/s; neither profile
bypasses the token budget.
Local HVF timings are not WAN timings. The single-trial registered campaign is a
functional integration pilot, not a latency-distribution or statistical speed
qualification.

The supplied demonstration uses public/synthetic data and has no FIPS validation
or CUI accreditation claim. Full upstream MVM workspace qualification, production
mailbox lifecycle, remote orchestration, Iroh, new-epoch cache migration and
arbitrary host power-loss recovery remain separate work.
