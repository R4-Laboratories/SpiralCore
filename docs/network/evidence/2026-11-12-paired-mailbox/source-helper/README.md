# Source mailbox helper retries the exact encrypted submission

A receives a canonical guest Submit over its private guest role, seals the submission, and writes the signed credential and ciphertext to the coordinator. It retains that frame when the coordinator connection closes without a verdict. On the next authenticated pull it can return the exact same ciphertext and message identity. A positive peer response can produce the guest’s correlated ephemeral SubmitResult. Durable journal acceptance still has its own receipt. [Source helper implementation](https://github.com/Casey-allard/mvm/blob/ba2cc8db503881cf29027ba8fd68a7f425a0cc22/crates/mvm-hostd/src/mailbox_process/source_ingress.rs)

On macOS arm64, the source exchange sent the five-byte fixture twice on fresh Unix socketpairs. The first peer closed after reading the frame. The second peer received the identical frame and returned a verdict. In a separate exchange B authenticated the retried frame and recovered all five bytes. The component test exercised the actual source and destination exchange functions. It supplied their sockets and reservations.

| Exchange | Peer frame / return | Helper to guest / guest to helper | Elapsed | Helper thread CPU |
|---|---:|---:|---:|---:|
| A, lost verdict | 4,954 / 0 B | 52 / 255 B | 7.339 ms | 2.973 ms |
| A, accepted retry | 4,954 / 11 B | 194 / 255 B | 4.121 ms | 2.786 ms |
| B, independent opening and readback | 4,954 / 11 B | 283 / 98 B | 10.056 ms | 3.704 ms |

Each row is one observation. The source timers include test-thread setup. Guest boot, the coordinator’s one-byte trigger, and WAN transit are outside these measurements. The three rows cannot be summed to estimate end-to-end time. The focused suite passed 22 tests against the recorded executables. [Measurements, CPU scopes and hashes](results.json)

The [cumulative contribution patch](source.patch.gz) applies to MVM contribution revision `43e99217dc7ff94d4f4f24aa7596dba18281bd7c` and reconstructs the tested revision `ba2cc8db503881cf29027ba8fd68a7f425a0cc22`. For the installed two-MVM test, the [prepared Firecracker workloads](../guest-images/README.md) give A and B different executables.
