# Original HVF worker and guest mailbox bridge

An Apple mailbox worker receives two private Unix stream descriptors from its original controller. They carry the two directions of the existing sealed mailbox protocol. The worker verifies the controller’s kernel-reported process identity and user, and keeps the full-duplex control connection open. It checks that both data streams are distinct and sets close-on-exec on the received descriptors. A length-prefixed configuration describes the two admitted original HVF instances. [Worker source](https://github.com/Casey-allard/mvm/commit/3fc7bf973e71d00a152299dd744b04a309a0775e)

For each instance, the worker arms its own process-exit observer before asking the original supervisor to answer the existing signed challenge. The answer binds that live process to its original VM identity and boot nonce. The worker opens the local role’s canonical port 5257 bridge and compares its kernel peer PID with the verified supervisor. Each worker observes both original processes. An observed exit or changed generation permanently retires the affected observer. The original SDK holds authority to stop either VM. [Original reader and identity-drive binding](hvf-original-reader.md)

| Host and source | Check | Result | Log SHA-256 |
| --- | --- | --- | --- |
| Apple Silicon, baseline test | Compile new lifecycle controls against predecessor | Missing original worker observation API | `5909b423f74900530558ce9ffe5eb7a8d39a67254786b74c929dea0722aa51bb` |
| Apple Silicon, `5f22eb9e7` | `mvm-vmm --lib hvf_stop::mailbox::` | 3/3 | `50c7473f2a87b3aa147296a8eed4d56db1bf70d91597a7d77487d53a31bae81c` |
| Linux x86-64, `c6023c901` | Same three observer and bridge regression controls | 3/3 | `2d363639d7167f5779c542ca93844cc17febf255f1c569e880c258599b113b7f` |
| Linux x86-64, `c6023c901` | Original object-worker controls | 8/8 | `0534f020eff6190f40e1636280f55d132a06e7bcdd78f1551e3c7f5db3cf43f1` |
| Linux x86-64, `c6023c901` | `cargo check --locked -p mvm-client --lib` | SDK compiled | `b1d6c98c628befdc08b8901e71e55ea5e761068d3be2ec244066ae43811ce57f` |

The locked debug builds used Rust 1.97.1. All 4,529 Linux source files matched Git tree `d1c5be106d9754cea09f42ea60a5635edc76ab0e`. The bridge fixture checked nine bytes, including two trailing zeroes, over a real Unix socket served by a synthetic supervisor. It also changed the instance nonce while retaining the same running PID and verified that restoring the record could not revive that observer. Two independent observers both saw the exit of the same real child process. These controls booted zero guests. Application transfer time and CPU, end-to-end vsock traffic and WAN throughput were unmeasured. [Installed object-pair tracker](https://github.com/R4-Laboratories/SpiralCore/issues/202)
