# Native Firecracker guest-agent Vsock enforcement

On Linux x86_64 with Firecracker v1.17.0, the booted MVM guest agent directly
started the static workload through canonical ProcStart. In both runs the
workload reported seccomp mode 2, IPv4 socket creation refused with EPERM, and
successful AF_VSOCK socket creation. The workload installs no filter. The
initramfs /init matched the agent in the checked contributor runtime archive.
Each guest used a private signed Dev plan, one vCPU, 1 GiB RAM, and no guest NIC.

| Run | Boot | Through canonical stop | Guest process exit | Harness exit |
|---|---:|---:|---:|---:|
| Default warm-pool settings | 585.129 ms | 669.475 ms | 0 | 1 |
| Idle Firecracker pool disabled | 557.082 ms | 642.147 ms | 0 | 0 |

The default run stopped the original workload. Its outer zero-Firecracker
assertion then caught a separate idle pool process. Its saved PID, process
start time, API socket and scope matched the test's pool entry. The named
scope was stopped through the user service manager and read back inactive.
The exact detached reaper session was then stopped after the pool process was
gone. The cold repeat exited zero, with no Firecracker PID or listening
socket for the test VM and the original workload scope inactive. Historical
state files and socket paths remain for inspection.

The canonical host signer was initialized inside the isolated MVM home. An
earlier attempt progressed to the launcher and failed because the elevated
process could not find Firecracker on sudo's restricted path. A private
read-only mount exposed only the pinned binary to that launch. Its identity
and the inherited namespace were checked through sudo. The host installation
was unaffected.

This verifies enforcement by the ordinary checked contributor guest agent.
The installed mailbox lane still requires the original authenticated signed
bundle runtime and its own lifecycle witness. This is not a transfer benchmark,
production release validation, or installed mailbox acceptance.
[Runtime preflight and input pins](../2026-11-05-guest-agent-enforcement/README.md)
