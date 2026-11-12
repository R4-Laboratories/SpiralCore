# One signed control request for an addressed Cast

The development SDK now asks the installed MVM host agent to perform one A→B Cast. While it holds both original admitted clients, the launcher prepares a single-use handle bound to the direction. After the two helpers enter Serving, the SDK signs the request and sends it to the original coordinator’s authenticated Unix socket. [SDK and control request](https://github.com/Casey-allard/mvm/blob/b71beee44ed68348ce3488db920369e28f9da802/crates/mvm-hostd/src/mailbox_process/owned_client.rs)

The control request carries the two owners, fresh nonce, active resource epoch, deadline and a public projection of the signed plans, grants and fixed policy. The coordinator checks both plans with its own host signer key, recomputes their content addresses, and compares them with the original worker admissions. Each original helper must have announced its own role and the same direction on its private lifetime channel. The coordinator pulls from A and forwards to B through the existing bounded data path. The inner message ID and plaintext remain at the endpoints. A successful control response reports B’s transient ingress verdict. Durable admission and workload readback have separate evidence requirements. [Verifier and dispatch](https://github.com/Casey-allard/mvm/blob/b71beee44ed68348ce3488db920369e28f9da802/crates/mvm-hostd/src/mailbox_launch/cast_pair/development/control.rs) · [Data path](../source-pull/README.md)

**One attempt per original source generation.** Once the coordinator claims the turn, it rejects further requests for that original, including a replay with a different nonce. A lost terminal reply leaves the caller uncertain about delivery. The lower-level helper can retain its original ciphertext, but recovery of a signed coordinator turn after an unknown result remains open. The release covered here admits only the expressly enrolled compact Firecracker development profile. Policy changes require retiring the old generation.

## Verification

Fourteen checks passed on Apple Silicon macOS. Eleven exercise the signed worker-control contract. One verifies the original subprocesses and workspace ownership after a lost reply. One starts the retirement collector, confirms it waits on an active turn and verifies collection after release. One checks the public projection against the two original owners and exercises the source and destination data handlers with exact readback of `00 41 ff 42 00`. A formatting revision failed compilation when its module export was dropped. The final revision restores that export and passed the same portable and host checks. [Test output](macos-arm64.txt) · [Source, executable hashes, measurements and failed revision](results.json)

| Independent component observation | Actual traffic | Elapsed | Thread CPU |
|---|---|---:|---|
| Original subprocess handoff ending at EOF | 4-byte prefix + 257 B body, 0 B reply | 1.376416 ms | Sender 0.091792 ms, receiver 0.057417 ms |
| A’s helper, first attempt with lost reply | Helper → relay 4,974 B, return 0 B. Helper → guest role 52 B, return 255 B | 5.761000 ms | Helper 3.251709 ms, guest role 0.128666 ms, relay 0.138959 ms |
| A’s helper, retry of the exact sealed frame | Helper → relay 4,974 B, return 11 B. Helper → guest role 194 B, return 255 B | 4.733375 ms | Helper 3.168625 ms, guest role 0.043584 ms, relay 0.178334 ms |
| B’s helper, opening that frame and confirming five bytes | Peer → helper 4,974 B, return 11 B. Helper → guest role 283 B, return 98 B | 10.115875 ms | Peer 0.062458 ms, helper 6.090708 ms, guest role 0.043500 ms |

Each row has its own timer and socket boundary. These are single unoptimized observations. The packet has no end-to-end timing for the installed control driving two booted guests, no WAN measurement and no zstd comparison.

## Reproduction

The [contribution revision](https://github.com/Casey-allard/mvm/commit/b71beee44ed68348ce3488db920369e28f9da802) contains the 4,555 tracked files used for the macOS checks. The [compressed patch](source.patch.gz) applies to `36981259b447de8207c0949d102b1bb850d54bd9` and reproduces its tested successor tree `ef9923bfc19ca27ad5ba22fa1392235798c883da`.

```sh
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2
cargo test --locked -p mvm-core --test mailbox_worker_control -- --test-threads=1
cargo test --locked -p mvm-hostd --lib lost_peer_reply_keeps_cast_workspace_until_the_original_child_exits -- --test-threads=1 --nocapture
cargo test --locked -p mvm-hostd --lib provider_fence_waits_original_activation_beyond_grace_without_renewing_it
cargo test --locked -p mvm-hostd --lib independent_original_development_keys_carry_exact_bytes_and_refuse_swaps -- --test-threads=1 --nocapture
```
