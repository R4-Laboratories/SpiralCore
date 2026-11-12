# Reconciling a withheld final application ACK on the original guest

Two original x86_64 Firecracker guests completed a cold 4,096-byte object transfer and a warm lookup. On a third lookup, B reread the full 4,096 bytes, returned Published and acknowledged A’s original Call. A's original reader then processed A’s ACK of that Reply and returned its matching signed lease acknowledgment. The opt-in guest workload withheld that successful result from its application settlement logic exactly once. The resulting uncertainty left the original Reply lease in A’s pending slot. [Per-attempt measurements, source pins, binary identities and log hashes](ack-reconciliation.json)

The first ACK crossed A’s real guest application reader socket. The workload wrote 2,129 bytes and read 2,413 bytes before fault injection. A subsequent settlement request for the wrong sequence was refused. A fresh send was also refused while that lease was pending. The original worker then retried the original sequence through the canonical reader. It received confirmation for the same signed lease and cleared its pending slot. One more settlement attempt returned the empty-slot refusal. All five recorded outcomes had identical original boot, workload instance and signed-lease digest. [Guest return boundary](https://github.com/R4-Laboratories/SpiralCore/blob/a9b3beed2bc076006eddcc14e4ea9f7a29181712/tools/ari-own-mailbox-witness/paired-guest/src/object_ack_loss.rs)

| Native operation | Verified readback | Both guest port-5257 links | Unique sealed helper bytes | Elapsed |
|---|---:|---:|---:|---:|
| Cold Raw transfer, four Calls | 4,096 B | 487,101 B | 63,800 B | 3.892 s |
| Clean warm lookup, one Call | 4,096 B | 125,330 B | 14,266 B | 1.036 s |
| Warm lookup with final ACK result withheld | 4,096 B | 121,106 B | 14,197 B | 1.000 s in A |
| Original settlement retry | Same original Reply lease | 4,822 B | 0 B | 58.953 ms in A |

The successful retry wrote 2,129 bytes and read 2,413 at A’s guest application reader. At the host’s port-5257 boundary, A read 2,409 bytes and wrote 2,413. There was one A port exchange across the entire reconciliation sequence. B's guest stream and all four sealed observations recorded zero bytes and zero new turns during reconciliation. There was no second cross-guest submission or receipt. The original retry consumed 0.265 ms of A workload CPU. The faulted lookup plus all four subsequent controls took 1.428 s and 7.145 ms coordinator CPU. The JSON lists the original CPU and byte counters for each operation.

All six sessions closed, and both original helpers exited unassisted with code 0 on their first stop. The audit verified 17 entries, two sessions and two seals. The cold, warm and reconciliation intervals recorded zero I/O errors. A's lifetime guest meter recorded one write error outside those intervals. Setup to both verified starts took 5.341 s. The command took 12.15 s wall, 10.40 s user CPU and 0.25 s system CPU with peak RSS 84,016 KiB. A separate ARM guest-runtime build was active on the host during this single observation.

The fault acts after an actual matching reader response has crossed the guest application socket. These numbers cover recovery while the original A workload, both VMs and their authority remain live. They do not establish recovery after a process or VM restart or after lease expiry. The MVM services and their sealed carrier were unchanged.

## Reproduction

Compile the paired x86_64 musl release workloads with `large-object-fixture,object-reader-v1,object-ack-loss-qualification`. Seal the two role images and compile the host driver with `object-ack-loss-qualification`. The host driver and helpers used the dev profile with debugging information disabled. Use the two original image digests, a fresh selected installation and the same metered helpers and guest runtime as the [matched codec controls](matched-codecs.md).

```text
paired-journal installed-object-ack-loss A.ext4 A_SHA256 B.ext4 B_SHA256
```

The dedicated driver’s 20 component checks passed. The initial guest cross-build caught a private fixture entry-point error. The tested successor exposes that entry only inside the guest workload crate and only in the opt-in build. The original compiler log and final source and executable identities are recorded in the packet.
