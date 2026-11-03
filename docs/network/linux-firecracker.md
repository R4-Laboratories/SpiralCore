# Linux/Firecracker canonical mailbox witness

[Measured results, exact recovery and remaining qualification](linux-mvm-results-2026-11-02.md)

SpiralCore has an explicit Linux host entry point behind `linux-host`. The
default Linux executable remains the guest PID 1. Both use the existing transfer
engine and canonical MVM mailbox envelope; the host launches through MVM's
`FcDriver`, not a second packet plane or a new mailbox format.

This is a development integration. It does not activate the upstream production
mailbox service, add a guest NIC, replace FlowMux or fulfill the Iroh cross-host
transport issue. The runtime uses the same pinned MVM source and retained patch
as the HVF witness. Linux native execution is qualified on x86-64; ARM64 Linux
packing and architecture-aware boot arguments do not constitute an ARM64 KVM run.

## Build on an approved Linux environment

Current builds use the public pinned contribution fork and committed lockfile.
The [current patch bundle](../../crates/spiralcore-mvm-live/mvm-service-patch/README.md)
reproduces its source when a prepared tree is needed. Historical measurements
used the older bundle; retain that source identity when reproducing them.
Select a dedicated build directory as `CARGO_TARGET_DIR`.

```sh
test -n "$CARGO_TARGET_DIR"
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
cargo +1.97.1 build --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --features linux-host
cp "$CARGO_TARGET_DIR/release/spiralcore-mvm-live" "$CARGO_TARGET_DIR/spiralcore-linux-host"

RUSTFLAGS=-Ctarget-feature=+crt-static cargo +1.97.1 build --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --target x86_64-unknown-linux-gnu
cp "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/spiralcore-mvm-live" "$CARGO_TARGET_DIR/spiralcore-linux-guest"
readelf -l "$CARGO_TARGET_DIR/spiralcore-linux-guest"
```

The guest must have no `INTERP` program header: it runs directly as `/init`,
without a guest dynamic loader. The host and guest are different builds of the
same binary target; keep separate copies rather than accidentally running the
guest executable on the host.

Run contract tests with `--features linux-host --all-targets`; the dedicated
Linux CI job performs those tests and Clippy. CI compilation is separate from
native KVM execution.

## Host prerequisites and execution

Use an explicitly approved KVM host or the MVM project builder/test environment.
The host needs a functioning `/dev/kvm`, the MVM-required systemd user resource
scope, and its ordinary privileged Firecracker launch support. Do not disable
the scope requirement to work around a missing user-session bus.

The [pinned asset helper](../../tools/linux-mvm/prepare-assets.sh) downloads and
verifies the x86-64 test kernel and Firecracker into a new private directory. It
does not install globally, alter KVM permissions or launch a VM. The MVM driver
internally invokes `sudo`; the verified Firecracker executable must therefore be
available in that launch's standard executable path. A private client `PATH`
alone can be stripped by `sudo`. The Wick pilot used an explicitly recorded
temporary standard-path link, removed after testing, without changing sudoers,
session-manager configuration or other labs.

The host CLI retains the existing session arguments:

```text
spiralcore-linux-host session NEW_DIR KERNEL GUEST_ELF one|two warm|whole|raw|combined|combined-cpu|whole-mixed SIZE TRIAL [standard|experimental64]
```

All file paths must be explicit, and the result directory must be new. `one`
uses a guest sender and host receiver. `two` uses two guests and a ciphertext-only
relay, then boots a credential-free read-only verification guest on the receiver
disk. There are at most two 1-vCPU/1-GiB guests at once. Use the
[registered measurement contract](evidence/2026-11-02-linux-mvm/measurement-contract.md)
for the paired workload and accounting rules.

## Lifecycle and measurement boundaries

Each worker has its own private MVM home. The kernel stays held by descriptor;
the credential-bearing initramfs travels through a pipe and sealed anonymous
memfd rather than a public disk file. Only typed mailbox and workload-exit vsock
channels are present; FlowMux, telemetry and machine-control queues are not
repurposed.

On x86 Firecracker, guest poweroff need not exit the VMM. After application work,
flush/unmount and final console drain, the guest sends MVM's canonical exit code.
The host requires captured zero plus a successful completion record, terminates
the owned VMM through MVM's driver, observes process exit and synchronizes the
backing disk. The receipt explicitly records host termination rather than
claiming natural hypervisor poweroff. Parent error cleanup addresses only that
worker's fixed VM identity in its private home.

Linux currently has no equivalent of HVF's internal mailbox rate limiter. Its
effective rate and burst are null, with `mailbox_rate_enforced: false`; the
requested profile remains visible as intent. Compare Linux arms on that same
unshaped path. Do not pool their timings with rate-limited HVF results.

Envelope/framing bytes count both directions of the mailbox service. Two-guest
relay stream bytes count both mailbox hops. These counters exclude the separate
workload-exit service and virtio control/credit headers; they are not complete
device-wire counters. Endpoint CPU, worker CPU,
the partial observed Firecracker CPU sample, transfer elapsed time and full
boot/recovery lifecycle are distinct measurements. No energy or total-host-CPU
claim follows from the partial sample.

## Separate integration tracks

The next host-networking seam is the existing per-VM FlowMux endpoint after its
signed-plan destination admission, resource limits and audit decision. A
cooperating SpiralCore peer may encode authorized data there; opaque encrypted
traffic cannot be assumed compressible. This requires the separate adapter
decision under MVM issue #4607, with no hidden fallback or guest NIC.

Guest access is another typed service: the requested development-shell mode and
restricted sealed-workload management mode require separate capabilities and
audit. Vsock alone is neither SSH nor an encrypted remote-access protocol.
These modes are not implemented by this mailbox host-parity witness.
