# Focused Rust and security review

Reviewed the Linux host module, Linux feature/CLI dispatch, shared worker
supervision, patch-preparation dependency routing and pinned asset downloader.
The logical roster validated as `d1927d9e7ce51354`; both `rust-reviewer` and
`security-reviewer` were active. This was an independent source review, without
another build, performance campaign or expansion into unrelated code.

## Source findings

The Linux path reuses MVM's `FcDriver`/`VmmSpec` and canonical typed mailbox
channel. The declared `GuestDials` services are mailbox and MVM's existing
workload-exit reporter; there is no guest NIC, alternate FlowMux path, or `mvmd`
data-plane dependency. The existing signed/encrypted
envelope codec and capability checks remain in the shared adapter.

Credential-bearing initramfs bytes travel through the owned worker pipe and a
sealed anonymous memfd. The memfd path refers to the held producer descriptor
during synchronous Firecracker boot configuration. Kernel hashing and the boot
path likewise share a held descriptor. Public console reads are bounded and
refuse symlinks; malformed/missing/duplicate completion evidence fails closed.
The per-worker MVM home is created privately by the shared orchestrator and
validated by the Linux worker. Cleanup uses only the fixed witness VM name in
that private home, rather than a host-wide process search or kill command.

The Linux rate receipt explicitly distinguishes the requested HVF profile from
the effective unshaped Firecracker path. Null effective rate/burst and
`mailbox_rate_enforced: false` avoid attributing a larger transport allowance to
a codec. Worker CPU and the last observed Firecracker process CPU sample are
separate; the sample is explicitly partial and must not be called total host
CPU or added to nested guest CPU. Framing-byte counts retain their stated
exclusion of virtio control/credit headers.

The asset script downloads only fixed HTTPS URLs, checks complete pinned hashes
before extracting two named executable members, uses a fresh private directory,
and makes no global installation, KVM-access change or VM launch. Patch
preparation keeps all nine direct MVM dependencies in the same prepared source
tree rather than mixing canonical contract types from different sources.

## Items requiring resolution or explicit scope

1. **Linux architecture scope — source correction reviewed:** the launch now
   uses `FcDriver::workload_base_bootargs(false)` rather than a hardcoded x86
   serial console. This resolves the source mismatch. An x86-64 build/run still
   does not qualify ARM64 Firecracker support.
2. **Lifecycle evidence:** the parent now performs an owned cleanup subprocess
   after worker completion/error and in the worker guard's drop path. A real
   forced-worker/cancellation run must verify that the corresponding Firecracker
   process is gone. Source inspection alone does not establish no-orphan parity.
3. **Terminal completion — refined contract reviewed:** the guest now emits
   the canonical MVM workload-exit report only after campaign work, attempted
   sync/unmount, the final `guest_done` record and console drain. Success reports
   zero; cleanup/work failures report nonzero. `report_exit_to` uses MVM's exact
   little-endian i32 and requires the one-byte acknowledgment `[1]`; the focused
   interoperability test uses upstream `exit_capture::capture_stream`, and a
   wrong acknowledgment is rejected. Linux enables this reporter in private
   bootstrap; the host always declares its typed service, including on keyless
   verification boots. The host accepts only captured zero plus successful
   bounded console evidence, then performs owned VMM termination, observes the
   pidfd exit and synchronizes the disk. Its receipt explicitly states
   `natural_poweroff_observed: false` and names that terminal contract. This is a
   coherent completion contract without pretending to independently classify
   natural poweroff. The capture file can precede acknowledgment delivery by a
   small race, so no claim that the guest completed its shutdown syscall follows
   from host acceptance; the receipt appropriately makes no such claim.
4. **Production boundary:** the experimental port binding does not complete
   upstream mailbox service issue #4629, Iroh issue #4633, full rate-limit parity,
   the 1,000-workload matrix, or the separate FlowMux/network-adapter decision.

No additional credential-disclosure or canonical-envelope boundary defect was
identified in this bounded review. The points above gate the corresponding
parity/security claims, and require execution evidence or explicit qualification
in the delivery report. This review does not independently assert that the
native Wick runs, cancellation tests or final published source have passed.

## Follow-up scope

The guest-exit/config/host changes above received a second source-only pass.
Their ordering preserves zero-fidelity/durable-reopen checks rather than treating
an arbitrary VMM exit as transfer success. No new blocker was identified in that
follow-up. Actual Wick execution and forced-cancellation cleanup remain parent
validation obligations. The reported macOS test counts are execution evidence
from the parent, not tests independently repeated by this reviewer.
