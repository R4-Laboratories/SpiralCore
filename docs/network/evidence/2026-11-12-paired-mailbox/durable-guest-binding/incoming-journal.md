# Destination enrollment for an incoming production journal

A signed MVM plan can select one incoming sender by committing to a destination operator’s policy. The policy names A’s original mailbox and epoch, A’s signed local scope, B’s separately signed Send for A and B’s selected mailbox template. The host checks those two signatures and both mailbox authorities. B’s ordinary reader service creates and retains its own boot-bound Receive credential. A’s later enrollment must use that exact credential. [Contributor source](https://github.com/Casey-allard/mvm/commit/9d08b503c0d5a85c21e097edd05760024115d17d)

B derives a separate A→B payload key from its retained local key, the admitted policy digest and the original B boot. Its journal and receipt signer remain under B’s custody. The first admitted final A plan and boot claim the direction. The retained journal refuses a replacement generation on either side. Journal admission verifies the exact signed Send and reads the current revocation file. Ordinary own-mailbox profiles omit the new field from their signed representation.

## Executed socket and journal test

On Apple Silicon macOS, the test sealed 257 bytes ranging from 0x00 to 0xff followed by a trailing zero. It wrote the signed capability and encrypted envelope to a real Unix-domain socket, verified B’s signed V2 receipt, dropped the service, reopened the original retained journal and read all 257 bytes back through the host API with B’s original signed Receive. A subsequent turn using the revoked Send was refused.

| One debug-build observation | Measurement |
|---|---:|
| Application payload | 257 B |
| Completed writes to B’s socket | 5,503 B |
| Completed reads from B’s socket | 1,119 B |
| Start of sealing to verified signed receipt | 35.371 ms |
| Start of sealing through reopened journal and exact readback | 96.099 ms |
| Whole test process | 0.27 s wall, 0.03 s user CPU, 0.01 s system CPU |
| Maximum resident size | 24,150,016 B |

The CPU sample includes fixture creation, both endpoints, the positive turn, restart, negative controls and teardown. The byte counts cover only the positive socket exchange. Fresh signed credentials, ciphertext and their JSON encoding vary between runs. The sender and destination shared one process. This run measures canonical journal overhead for a small uncompressed body. A compression speedup, native two-guest exchange, guest application ACK and WAN time still need separate measurements. [Exact results and artifact hashes](incoming-journal.json)

Two incoming-policy tests passed, along with seven original-journal controls, one reader restart control, two durable journal controls and one ordinary production mailbox test. The patch applies to the contribution fork’s A-host integration base and reproduces the tested MVM tree. To run the incoming cases from that tree, use the command below. [Patch](incoming-journal.patch)

```sh
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo +1.97.1 test --locked --offline -p mvm-hostd --lib paired_incoming::tests -- --nocapture --test-threads=1
```
