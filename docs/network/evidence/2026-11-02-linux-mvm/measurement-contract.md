# Linux/Firecracker functional parity pilot

Registered before optimized guest execution on 2026-11-02. The user authorized
an isolated Wick KVM test, at most two 1-vCPU/1-GiB guests concurrently, with
synthetic fixtures, no guest NIC and no changes to DeepSeek or other labs.

Start with a one-guest and then a two-guest 1-MiB debug smoke. These are functional
tests, never optimized timing results. Failed runs remain recorded. Verify
guest/vsock operation, exact durable reconstruction, keyless reboot readback,
and no owned Firecracker process remaining after completion or cancellation.

After the smoke succeeds, build host and statically linked guest with release
optimization, Rust 1.97.1 and retained lockfiles. Use the pinned Firecracker,
kernel, MVM source plus retained patch and record source/binary hashes. Run
Combined, CPU-conscious Combined and prepared whole-object zstd on the same
eight mixed targets, with paid checkpoint/schema setup as already defined by
the existing native MVM campaign. Requested tiers are 1 MiB and 16 MiB; exact
trace lengths are 1,047,202 and 14,851,384 bytes. Trials are 0, 1 and 2; run one
guest/host and two-guest stages, for 36 paired sessions. Rotate arm order by trial
to reduce drift. Run sessions sequentially; limit build jobs to two.

Report both directions' canonical encrypted envelope and framing bytes, logical
delivery bytes versus two-hop relay bytes, sender/receiver process CPU,
per-operation and session elapsed time, boot/lifecycle and disk-sync time.
Include setup, exact byte/length verification and recovery. Never add nested
guest CPU to VMM CPU. The Linux worker observes Firecracker process CPU through
`/proc`; this can omit shutdown and is explicitly partial, not total host CPU.
CPU/elapsed ratios are descriptive three-trial pilot results, not tail-latency,
energy, WAN or 1,000-workload acceptance evidence.

The native Firecracker guest-dial bridge does not expose HVF's internal mailbox
rate limiter. Effective Linux rate/burst are null and rate_enforced is false;
the requested profile is retained only as intent. All Linux comparison arms use
the same unshaped path. Do not pool their timings with shaped HVF experiments or
claim rate-policy parity. Frame, capability, queue, lifetime and memory bounds
remain independently enforced. Production local mailbox, durable replay and
per-tenant scheduling remain upstream integration work, not acceptance claimed
by this pilot.

The user separately requested a host-side FlowMux/SpiralCore gateway and both
development-shell and restricted sealed-workload access modes. Those are
distinct follow-ups: preserve the signed FlowMux plan and single network path;
give interactive access its own admitted capabilities and audit. Mailbox
ciphertext relays remain keyless; no SSH service is inferred from vsock itself.
