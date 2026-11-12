# Durable mailbox delivery and receipt recovery on Apple Silicon

Two separately admitted MVMs on one Apple Silicon host exchanged a 313-byte SpiralCore frame over their original vsock mailbox services. Only A received the frame from the test driver. B used its own boot-enrolled reader, synced and reopened the 257 payload bytes, including the trailing zero, and acknowledged the lease. The driver verified the returned bytes against the original and matched the independently recorded message IDs. MVM identified each running VM as HVF. Each used one vCPU, 256 MiB RAM and no guest NIC. [Machine-readable measurements, artifact identities and meter checksums](arm64-hvf.json)

| One native run per column | Clean delivery | First signed receipt withheld |
|---|---:|---:|
| SDK setup through admission of both guests | 4.822 s | 5.241 s |
| A’s first Call through host verification of B’s first signed durable receipt | 85.394 ms | 72.129 ms |
| Same start through verification of the replayed receipt | — | 100.209 ms |
| Same start through B’s ACK and host comparison of the reopened payload | 2.117140 s | 0.550778 s, including an extra empty-read check |
| Sealed frame and signed Send written toward B | 5,863 B | 5,854 B twice |
| Signed receipt read back from B | 1,208 B | 1,214 B twice |
| Signed receipt written back toward A | 1,208 B | 0 B first, 1,214 B on retry |
| A / B workload process CPU | 0.301 / 0.649 ms | 0.533 / 0.938 ms |
| Orchestrator wall / process CPU, including setup and retirement | 7.444 / 3.695 s | 6.307 / 4.036 s |
| Outer command wall / user CPU / system CPU | 8.67 / 4.12 / 0.55 s | 7.70 / 4.26 / 0.65 s |

The test-only relay read and verified B’s entire first receipt, then closed the connection before returning receipt bytes to A. The original guest agent retried the unchanged pending operation. The sealed envelope and serialized signed receipt matched across both turns. A observed its pending slot retire with neither a rejection nor an uncertainty record. After B acknowledged one lease, a subsequent authenticated read returned empty. Both original helpers exited with code 0, unassisted, on the first stop in each run. Both audit chains verified 23 entries, two sessions and two seals.

The relay counts cover the sealed envelope, Send credential and signed receipt. The port-5257 counts cover a different boundary, including guest-side reader and ACK traffic. A’s clean meter recorded 381 B read / 1,208 B accepted for writing, and B’s recorded 2,628 / 8,224 B. On retry these pairs were 762 / 1,233 B and 2,976 / 11,759 B respectively. All four meters closed. The JSON also records the SDK application-channel bytes. Summing these scopes would double-count traffic. Both runs expanded this small message. One observation per case cannot estimate a latency distribution or the cost of a retry. WAN, matched raw/zstd throughput, energy and sustained capacity require separate runs. [Clean A](arm64-hvf-clean-a.counts) · [Clean B](arm64-hvf-clean-b.counts) · [Retry A](arm64-hvf-retry-a.counts) · [Retry B](arm64-hvf-retry-b.counts)

## What changed on Darwin

Three earlier native runs reached both HVF machines and stopped them cleanly. The source submitted a frame, but the relay returned EINVAL before it could verify B’s receipt. The third run located the error at the read of B’s signed verdict. Darwin can reject a socket timeout change after the peer closes while response bytes remain buffered. The fix gives the bounded exchange exclusive use of a nonblocking stream, drains readable bytes through EOF, and waits for readiness within the original absolute deadline. It preserves original helper identity, signed receipt verification and application ACK semantics. The failed runs each recorded 762 B read / 19 B accepted for writing on A’s port-5257 helper and 0 / 0 on B’s guest-facing helper. [Original failures and native process outcomes](arm64-hvf-before.json) · [Fixed MVM source](https://github.com/Casey-allard/mvm/commit/8238000662f59893b6ec703ab4b37d7a86c08340)

## Reproduction inputs

Use the [paired qualification driver](../../../../../tools/ari-own-mailbox-witness/paired-native.md), Rust 1.97.1, MVM host revision `82380006` and guest revision `93b3b65a`. The test host ran macOS 27.0 build 26A5378n. The driver requires separate original HVF backend observations for A and B. The supervisor used MVM’s `assets/mvm-supervisor.entitlements` and passed local code-signature verification. The signed mailbox plans, keys and journals were created independently in each fresh local MVM home.

1. Build the guest runtime from the pinned guest revision with `mvmctl dev guest-bins --arch aarch64`. The canonical source cache on the Mac verified the archive. MVM’s `prepare` produced the same initramfs, overlay, verity and sidecar identities as the Linux assembly. The run used the manifest-checked default ARM64 tenant kernel from `image-set/v0.2.4`.
2. Build both role workloads with `cargo +1.97.1 zigbuild --locked --release --target aarch64-unknown-linux-musl --bins`. For the retry run enable `receipt-loss-qualification`. Seal two distinct root images per run with the canonical image builder. All four rootfs, verity, root-hash and metadata file hashes are in the results JSON.
3. Build the two driver variants as described in the linked instructions. Apply the receipt-loss overlay only to the retry driver. Both runs use the same production mailbox, HVF supervisor and service helpers. Point MVM’s helper paths to those original executables, prepare an isolated home with the original 30-second retention ceiling for each run, and verify the frame and image digests before boot.
4. Run `paired-journal --backend hvf A_ROOT A_SHA B_ROOT B_SHA fixtures/workload-frame.bin` for each variant. Retain the result, both closed meter files, external process timing and `mvmctl trust audit verify` output. The extra readback over the SDK channel is qualification traffic. A sealed cross-host carrier remains tracked separately. [WAN acceptance](https://github.com/R4-Laboratories/SpiralCore/issues/47)
