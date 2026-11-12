# Two original mailbox helpers and a lost source reply

Two Linux `mvm-mailbox` processes exchanged a B-addressed message through the local relay. B durably admitted the 257-byte payload and signed its V2 receipt. On the first submission the fixture closed A’s guest connection before reading the response. Two retries through the same helpers returned B’s original signed receipt. B’s fixture reader then recovered the original 257 bytes, including embedded and trailing zeros, and checked the signed application ACK echoed by B. Both original helper processes exited successfully. [Recorded measurements](results.json)

The test supplied fixture admission and boot materials for A and B and spoke the Firecracker guest handshakes over Unix sockets. The original helper executables performed sealing, forwarding, journal admission, receipt verification and reader service. The relay authenticated each original helper socket against its live child. The destination Send, source scope and B Receive came from independent authorization checks. The source and destination checked revocation in their original installation. Durable admission and the application ACK occurred on separate protocol exchanges. [Ari’s journal draft](https://github.com/tinylabscom/mvm/pull/5377) · [Original B enrollment](../durable-guest-binding/incoming-journal.md)

## One Linux debug sample

| Submission | Fixture writes to A | Sealed A → B | V2 B → A | Fixture reads from A | Submit through receipt verification |
|---|---:|---:|---:|---:|---:|
| First, fixture disconnects | 325 B | 5,539 B | 1,145 B | 0 B | unavailable |
| Retry 1 | 325 B | 5,539 B | 1,145 B | 1,145 B | 19.024 ms |
| Retry 2 | 325 B | 5,539 B | 1,145 B | 1,145 B | 20.276 ms |

Relay counts come from completed reads and writes across two Unix-socket legs. Each row counts the sealed record and return verdict once per direction. The first fixture completed its writes and disconnected. Its corresponding helper-to-fixture write count is unknown. Signed JSON lengths may vary with the fixture’s generated credentials.

B’s separate Receive exchange used **342 B toward its helper and 5,700 B back**. The signed ACK exchange used **2,217 B toward the helper and 2,221 B back**. From the first fixture connection through verified B ACK the elapsed time was **59.147 ms**, including all three submissions. The isolated test command took **0.66 s wall, 0.03 s user CPU and 0.01 s system CPU**, including setup, both helpers and cleanup. GNU time reported **35,960 KiB maximum RSS**. The record contains one sample with shared command-level CPU accounting. Per-endpoint CPU, send-only time and physical-network bytes remain unmeasured. The input was raw synthetic application bytes. This test supplies no comparison against zstd or evidence of WAN compression savings.

## Reproduction

Start on Linux from the [public contributor baseline](https://github.com/Casey-allard/mvm/commit/9d08b503c0d5a85c21e097edd05760024115d17d). Apply the [Rust patch](two-helper.patch) with `git apply --index`. Verify that `git write-tree` produces `cbff01b12fbb50963ef83bc579e1a66f37983cca`. Use Rust 1.97.1, two build jobs, disabled incremental compilation, and debug/test profiles without debug symbols. The offline commands assume the locked dependencies are already cached.

```sh
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cargo +1.97.1 build --locked --offline -p mvm-hostd --bin mvm-mailbox
cargo +1.97.1 test --locked --offline -p mvm-hostd --lib journal -- --test-threads=1
cargo +1.97.1 test --locked --offline -p mvm-hostd --lib original_a_and_b_helpers_relay_same_sealed_intent_after_lost_guest_reply -- --ignored --nocapture --test-threads=1
```

The journal selector passed 31 tests. Run the explicit ignored test separately after building the actual helper binary. It launches one isolated fixture subprocess, so nested harness output represents one measured trial. An earlier fixture kept its synthetic CONNECT listeners open during teardown and missed the original child exit deadline. A subsequent fixture revision failed to compile its ACK-key return tuple. The final test closed both fixture listener sockets, withdrew the fixture state and collected both original children. [Sanitized measured output](measurement.txt) · [Evidence hashes and earlier failures](results.json)

Two admitted VMs using their real SDK, guest readers and vsock carrier remain the installed delivery gate. Cross-host execution and platform acceptance are tracked separately. [Paired delivery issue](https://github.com/R4-Laboratories/SpiralCore/issues/168)
