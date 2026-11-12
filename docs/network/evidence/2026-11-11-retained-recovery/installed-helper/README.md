# Installed host-agent collection of a per-VM helper

On Linux x86-64, a real pipe-backed child reached `Serving` through signed `Acquire`, `DelegateConfig` and `Query` requests. Before the repair the host agent refused retirement because it had no receipt from the original worker provider. The fixture used a separate signed `Stop` to collect the child, then failed its coordinator assertion. After the repair the coordinator collected the original child with exit code 0, joined the original control loop and retained the exit receipt. The success path used no fixture stop.

| Single release-build invocation | Result | Elapsed |
| --- | --- | ---: |
| Original coordinator, before repair | Expected refusal | 8.75 ms |
| Original coordinator, repaired | Original child collected | 9.65 ms |
| Reconcile while another operation holds the original resource | Waited for release, then returned one permit | 24.81 ms |
| Journal failure after registration reply | Original signer collected, worker exited 1 | 31.67 ms |

Each elapsed time covers its entire test process. The repaired coordinator fixture recorded 7,888 KiB maximum RSS. GNU time reported 0.00 s user CPU and 0.00 s system CPU at 0.01 s resolution. These lifecycle fixtures sent no guest object and yield no transfer rate or compression comparison.

All 27 repaired checks passed on Rust 1.97.1. The run retained hashes for five executables and 4,498 source files. The test injects the provider into the installed coordinator. A remaining resource-lifetime question is whether a retained provider keeps reaped child handles and sockets after it returns worker credit. Production helper installation, Firecracker guest transfer and WAN acceptance remain open. [Recorded output, timings and identities](results.json).

To repeat the real-child case on Linux, check out [the exact MVM fork revision](https://github.com/Casey-allard/mvm/tree/4842bdf8db9fe30ba6f911b37bcc881bd94d1ec4) and run:

```sh
cargo test --release --locked -p mvm-hostd --bin mvm-host-agent \
  worker_injected_owner_tests::installed_retirement_collects_injected_original_mailbox_child \
  -- --exact --nocapture --test-threads=1
```

The JSON identifies each additional Cargo target and exact test name. Use `--lib` for the `mvm_hostd` cases and `--test host_agent_idle_collection` for the process cases.

## Guest compatibility with MVM 0.28.0-rc.1

The [combined source](https://github.com/Casey-allard/mvm/commit/f61a8bd67c61b3f213923cdd073e865094e5d804) incorporates upstream `3010cfdb`, including [Ari's change to hold an idle mailbox dial open](https://github.com/tinylabscom/mvm/pull/5343). On November 12, two guest-agent mailbox tests and 17 outbox tests passed on a Linux x86-64 host. Cases covered exact frame bytes, ordered spool delivery after a full-queue refusal, concurrent writers, and refusal of an ambiguous service grant. The outbox suite took 1.09 s wall time, 0.02 s user CPU and 0.02 s system CPU, with 14,072 KiB maximum RSS. The previous coordinator evidence remains pinned to 0.27. [Test names, executable hashes and measurements](linux-028-guest.json).

To reproduce, use Rust 1.97.1 and build the `mvm-agentd` binary and library test harnesses with `cargo test --locked -p mvm-agentd --target x86_64-unknown-linux-musl --bin mvm-guest-agent --lib --no-run`. We used the debug test profile with debug symbols and incremental compilation disabled, cross-linked with `rust-lld` on macOS, checked SHA-256 after copying both executables to Linux and ran them with one test thread. Select `mailbox::tests::` in the binary harness and `mailbox_outbox::tests::` in the library harness. Both ran as unprivileged host processes. The native Firecracker transfer, installed helper lifecycle and WAN tests require their own runs.
