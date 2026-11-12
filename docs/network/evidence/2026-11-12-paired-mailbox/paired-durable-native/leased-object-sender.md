# Settling one object Reply before sending the next Call

A guest with one reader lease needs to validate the receiver's object response before it ACKs the lease. The shared Rust sender now issues one Object-v1 Raw Call at a time. A verifies a Ready response to its Offer, cumulative accepted length after each Data frame, and the final Published object digest. A valid refusal settles its own lease and ends the transfer with the remote error. A mismatched, malformed or oversized response remains unsettled. A failed or ambiguous ACK immediately stops the sender. The mailbox adapter owns authenticated correlation, original lease retention and recovery. [Sender source](https://github.com/R4-Laboratories/SpiralCore/blob/4599740c2c5f488a1744811ec37ce2d08c9cb8fa/crates/spiralcore-transfer/src/client.rs) · [Regression tests](https://github.com/R4-Laboratories/SpiralCore/blob/4599740c2c5f488a1744811ec37ce2d08c9cb8fa/crates/spiralcore-transfer/tests/batched.rs)

A single test process exchanged encoded frames with the real file-backed object receiver. It opened one receiver and submitted the same 4,096 bytes twice under independent transfer IDs. It decoded every returned frame, settled synthetic leases and compared both exact file readbacks with the source. The request/reply caps were 4,152 and 88 bytes. On each host the cold transfer needed four Calls and encoded 4,376 request bytes plus 282 reply bytes. The repeat needed one Offer of 102 bytes and one Published reply of 88 bytes, with zero Data bytes. These are serialized application bytes. The test used no socket, MVM envelope, V2 receipt or WAN link.

| Host and cache | Through final lease settlement | Through independent readback check | Sender + harness thread CPU through settlement | Receiver handler thread CPU |
|---|---:|---:|---:|---:|
| Linux x86-64, cold | 1,176 µs | 1,250 µs | 63 µs | 1,112 µs |
| Linux x86-64, warm | 130 µs | 200 µs | 54 µs | 76 µs |
| macOS arm64, cold | 15,351 µs | 15,737 µs | 281 µs | 5,290 µs |
| macOS arm64, warm | 668 µs | 1,047 µs | 262 µs | 406 µs |

Each row is one unoptimized Rust 1.97.1 test invocation's cold or warm sample. Receiver CPU covers request decoding, handling and reply encoding on the calling thread. Sender and harness CPU is the remainder of that thread's CPU through settlement. Both elapsed columns start before the sender hashes the input. Receiver setup and the post-settlement readback CPU are outside the CPU columns. These measurements establish neither a zstd comparison nor installed throughput.

Run `cargo test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --test batched -- --nocapture --test-threads=1`. Both hosts passed all 17 tests. macOS also passed the adaptive, prepared-client, combined-session and v1-contract suites (39 checks) and strict Clippy for the library and batched test target. The installed acceptance requires independent signed Call/Reply admission, the original guest reader, durable pending sends, and exact readback across the two guests. [Installed delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/202)
