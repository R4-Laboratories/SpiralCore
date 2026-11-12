# Bounding addressed guest calls

On the installed Linux pair, both SDK launches reached admission, but neither workload produced a verified result. Both warm-worker consoles recorded an OCI entrypoint error. During the stall, two witness threads had established Unix streams to the two original Firecracker vsock multiplexers. Two other threads waited for workload-exit events on port 5251. The recorded call finally returned an incomplete control-frame error after host recovery. Those observations leave the number of bytes accepted by either workload unknown. [Native precursor and cleanup](https://github.com/R4-Laboratories/SpiralCore/pull/164)

The guest's 15-second workload timeout starts inside the guest. The original caller could wait longer for a multiplexer acknowledgment, authenticated negotiation or stream output while the VMM remained alive. An outer Rust thread join also delayed reporting the signed route result. The addressed driver now passes a single absolute host deadline to both guest calls and the signed route. It logs each role’s input length and transport outcome, without logging the challenge, message contents or credentials.

The opted-in SDK API selects the original guest control stream under that deadline. The existing canonical transport watchdog shuts down its own connected socket when time expires. The ordinary RunEntrypoint authorization, capture, guest timeout and terminal event handling remain in the call path. Original VM shutdown and successful collection are separate lifecycle checks. [MVM implementation](https://github.com/Casey-allard/mvm/commit/8b9f5573e5067a7da174e630d5be11aafe613892)

| Linux x86-64 component suite | Passed | Harness elapsed |
|---|---:|---:|
| MVM SDK entrypoint dispatch | 17/17 | 0.22 s |
| Addressed witness library | 11/11 | 1.55 s |

The added test uses an isolated control listener. It accepts the canonical negotiation bytes, stays silent and observes EOF when the 200 ms host deadline closes the original socket. Its guest timeout is 30 seconds. The host deadline covers socket selection and exchange. Filesystem I/O, caller-provided observers and process scheduling can add elapsed time. These component tests did not start a VM or measure a guest transfer. [Measurement and reproduction record](one-shot-call-deadline.json)
