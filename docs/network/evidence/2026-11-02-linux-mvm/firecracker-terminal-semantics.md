# Firecracker 1.17 terminal semantics

Primary source review, 2026-11-02, before terminal acceptance implementation:

- [Version-pinned FAQ](https://github.com/firecracker-microvm/firecracker/blob/v1.17.0/FAQ.md):
  x86 has no emulated ACPI power-management shutdown. Linux poweroff halts the
  guest OS but leaves the VMM process alive. With `reboot=k`, a guest reboot
  reaches the emulated i8042 reset interface and terminates Firecracker.
- [Version-pinned VMM stop](https://github.com/firecracker-microvm/firecracker/blob/v1.17.0/src/vmm/src/lib.rs#L691):
  the stop log line is shared by normal and erroneous stop paths. Its presence
  alone cannot prove successful guest completion.
- [Version-pinned vCPU exits](https://github.com/firecracker-microvm/firecracker/blob/v1.17.0/src/vmm/src/vstate/vcpu.rs#L486):
  ARM system-event logging preserves the numeric reset/shutdown distinction;
  the successful i8042 reset path on x86 does not have that same marker.
- [Version-pinned i8042](https://github.com/firecracker-microvm/firecracker/blob/v1.17.0/src/vmm/src/devices/legacy/i8042.rs#L258):
  a successful reset signals an eventfd and increments a metric, without a
  distinct success-only log line suitable for a strict poweroff classifier.

Consequences: an x86 worker must not wait for natural process exit after the
existing `RB_POWER_OFF` guest, nor treat a generic VMM stop log as proof of clean
poweroff. The appropriate MVM-native completion boundary is the existing typed
workload-exit report (port 5251, four-byte little-endian exit code and host ACK),
combined with the successful application completion marker after guest flush and
unmount. The worker can then explicitly terminate its owned VMM, confirm teardown
and synchronize the backing disk. Reports must call this application completion
plus host termination, not a witnessed guest poweroff.

The canonical reporter and capture are already present at pinned MVM revision
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b` in
`crates/mvm-agentd/src/bin/mvm-exit-report.rs` and
`crates/mvm-core/src/exit_capture.rs`. These observations do not authorize adding
a new control protocol, mailbox envelope or forwarding path.
