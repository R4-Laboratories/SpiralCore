# Firecracker guest-agent enforcement preflight

The checked contributor runtime assembled successfully. The initramfs `/init`
and the verified runtime archive's guest-agent member have the same SHA-256.
The ordinary signed Dev launch passed runtime and image metadata selection, then
stopped before Firecracker started because the isolated host had no signer
public anchor. Guest seccomp behavior is **unverified** in this run.

The Rust fixture requests the Vsock tier, HostVsockProxy, no ingress, no
services, and no secret release. It is designed to start the static probe
directly with canonical guest ProcStart. The probe only inspects filter mode and
calls the socket API. Its expected output is seccomp mode 2, IPv4 socket EPERM,
and successful AF_VSOCK creation. On the host the same probe refused unexpected
arguments and ordinary unfiltered execution. The earlier recovery witness used
a development entrypoint to install the filter, which answers a different
question. [Same-VM mailbox recovery evidence](../2026-11-05-installed-mailbox-bounded-dial/README.md)

The starting rootfs is from the published x86_64 Firecracker image set. The
derived test rootfs contains the static probe, carries the unchanged
release-provided metadata, and has freshly generated workload verity and an
explicit digest in the private signed plan. The newer guest runtime comes from
a private, clean composed SDK checkout and was checked by MVM's contributor
runtime path. The checkout ID identifies the build input, and is a private
composition of the prior reviewed SDK changes. This evidence packet includes
the witness sources and pins. It does not contain a complete SDK source export,
a runtime archive, or any signing key.

Next initialize the isolated host observation identity through MVM's normal
signer path, verify its public anchor, and run this same direct guest workload.
Installed mailbox recovery additionally requires a verified original signed
bundle. Contributor runtime verification supplies separate evidence from that
installed-runtime binding. No WAN, transfer, FIPS, or upstream PR acceptance is
claimed by this preflight.
