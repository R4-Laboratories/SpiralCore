# Two installed Firecracker guests on MVM host source incorporating #5384

Four fresh Linux x86-64 guest pairs ran the same synthetic 4,096-byte object cold and then repeated it against the original receiver. The cold request offered raw bytes or whole-object zstd under the same Object-v1 protocol. Both modes used the verified receiver cache on the repeat. All eight readbacks matched the original. The experiment changed the offered codec and used the order raw, zstd, zstd, raw to expose order effects. Each new pair began with fresh keys, queues and workload state. The 5 ms guest-reader cadence and signed images were common to every pair.

## Measurements

Application traffic includes Object-v1 requests and replies. The sealed column counts each exchanged byte once across the two mirrored helper channels. The vsock column counts reads and writes on both guest port-5257 links. These are different observation points and must not be summed. CPU columns are sender workload, receiver workload and host coordinator process time. Elapsed time runs through both guest results. The two application workloads and the coordinator do not account for the entire host or VM.

| Run and phase | Calls | Application B | Sealed B | vsock B | Sender CPU ms | Receiver CPU ms | Coordinator CPU ms | Elapsed s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Raw 1 cold | 4 | 4,658 | 63,970 | 503,458 | 5.675 | 29.343 | 14.474 | 4.043012 |
| Raw 1 repeat | 1 | 190 | 14,226 | 129,218 | 1.263 | 8.964 | 4.384 | 1.188749 |
| Zstd 1 cold | 4 | 4,674 | 63,955 | 893,450 | 11.339 | 31.843 | 31.149 | 11.254917 |
| Zstd 1 repeat | 1 | 190 | 14,317 | 182,313 | 3.233 | 9.034 | 9.107 | 2.841312 |
| Zstd 2 cold | 4 | 4,674 | 63,987 | 801,152 | 11.046 | 29.848 | 27.681 | 9.453171 |
| Zstd 2 repeat | 1 | 190 | 14,294 | 165,941 | 3.133 | 7.588 | 5.412 | 2.440655 |
| Raw 2 cold | 4 | 4,658 | 63,871 | 620,789 | 8.540 | 28.881 | 18.942 | 6.147339 |
| Raw 2 repeat | 1 | 190 | 14,248 | 145,701 | 2.403 | 7.684 | 5.148 | 1.637743 |

Both cold zstd operations sent 4,110 compressed data bytes for this 4,096-byte fixture. The additional codec framing brought the application cost to 16 bytes above cold raw. Each exact repeat saved four kilobytes of payload and used one original mailbox Call. The 190 application bytes still required roughly 14 KiB on the sealed channel and 126–178 KiB across the two guest links. The raw pairs finished sooner in both orders. Two samples per codec, substantial variation in guest polling and a shared host leave the cause and population speedup unresolved. A standalone zstd sender without receiver-cache negotiation, send-only timing, direct/relay WAN and FIPS deployment qualification are outside these measurements.

The original harness recorded one guest A write error in every final closed-session meter. Every cold and repeat phase meter recorded zero errors. Both helpers exited zero without assistance on their first stop in each run, with zero remaining Firecracker processes and active MVM scopes. All four audit chains verified 17 entries, two sessions and two seals. The contended launch observer saw the original driver holding a descriptor on the locked inode over two observations in every run.

| Run | Original counters, receipts and timing | Launch overlap, resource use and retirement |
|---|---|---|
| Raw 1 | [JSON](raw-1.json) | [Observed outcome](raw-1.txt) |
| Zstd 1 | [JSON](zstd-1.json) | [Observed outcome](zstd-1.txt) |
| Zstd 2 | [JSON](zstd-2.json) | [Observed outcome](zstd-2.txt) |
| Raw 2 | [JSON](raw-2.json) | [Observed outcome](raw-2.txt) |

## Source and reproduction

The contributor host revision includes Ari's merge of the Hickory dependency update at `550f1638`. The original source archive contained 4,532 verified host blobs. The guest source remained at `c9d9fe87`, with 4,515 verified blobs. Both inventories matched Git content and executable/symlink modes. The driver and workloads came from SpiralCore `615d8ff9`. Use the preceding launch recipe with the four codec arguments in the recorded order, a fresh home for every pair, and one cold/identical object sequence per receiver. The workload selected the same object ID in all four runs. [Host source](https://github.com/Casey-allard/mvm/commit/6af1b9b18e6a80f6798c75198d44581368bf0eb3) · [Upstream dependency change](https://github.com/tinylabscom/mvm/pull/5384) · [Driver source](https://github.com/R4-Laboratories/SpiralCore/tree/615d8ff98429b2339a069dcd0735c3df1a40af25/tools/ari-own-mailbox-witness) · [Original launch recipe](../README.md#reproduction) · [Five installed executable hashes](host.sha256).

Two integration test problems surfaced on the initial merged source. An assertion expected mode 0644 under umask 077, where the kernel created mode 0600. Using the native open options as the comparison passed all 19 private filesystem checks at each of 077 and 022. The guest test binary also referenced an undeclared hex crate. Restoring the original guest test rebuilt that binary. Two focused guest tests passed and its private mount namespace test was skipped in this run. All 66 selected Linux volume tests passed. The host build retained three warnings. These mailbox runs did not exercise the optional custom DNS resolver or the full workspace release checks. [Qualification output fingerprints](qualification.sha256).
