# Retiring an original Firecracker process after failed activation

Two earlier installed two-guest runs booted their VMMs and then refused activation. Each original systemd scope was checked against its recorded invocation, cgroup and original process before an operator stopped it. All four scopes became inactive with no remaining process. The SDK lacked backend exit evidence in those runs. [Observed guest failures](guest-activation.md)

The change covers an installed Linux Firecracker launch between successful backend boot and broker registration. The host obtains a process lifetime descriptor directly from the kernel for the connected Firecracker API socket and checks the process identity returned by the original driver on both sides of that operation. The host keeps the descriptor, VM handle, endpoint, console, canonical kernel and VM name lease through a failed launch. An authorized retry signals that descriptor and waits for kernel exit readiness. Release verifies that the recorded kernel generation belongs to the same process. The mailbox worker has its own retirement requirement. [Contribution source](https://github.com/Casey-allard/mvm/commit/f8feff807e46b877e4785ebc89714398de0fe6ac)

The installed role requires the Linux socket option `SO_PEERPIDFD`, introduced in the upstream 6.5 kernel, and permission to signal the original VMM. A socketpair probe checks for the kernel option before starting the VMM. If the actual Firecracker peer cannot be bound after boot, or exit or resource release cannot be verified, the SDK retains the failure and the original reservation. Discarding an unresolved error retains its launch guards for the rest of the provider process lifetime. Recovering after provider death requires a separate durable authority. [Linux 6.5 socket UAPI](https://github.com/torvalds/linux/blob/v6.5/include/uapi/asm-generic/socket.h)

## Linux process controls

| Case | Observation |
|---|---|
| API socket path replaced after original binding | The originally bound subprocess exited. A separately spawned replacement remained live. A second stop observed the same original exit. |
| Process generation changed during binding, or socket was absent | Binding failed. The original subprocess remained live. |

These controls used ordinary Linux subprocesses on a native x86-64 host. The test binary's ignored child entrypoint supplies the live listeners. No workload data crossed a mailbox in these tests. The installed KVM rollback test still needs a deliberately refused launch whose original VMM and worker both retire through SDK custody. [Commands, results and identities](original-firecracker-rollback.json) · [Source patch](original-firecracker-rollback.patch)
