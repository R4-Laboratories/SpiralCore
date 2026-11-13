# Distinguishing sender reply deadline failures

The ordinary platform matrix at PR head `5c38bf3c` failed in three Windows roles and one separate Intel macOS duplex case. [Original observations and verified artifact hashes](https://github.com/R4-Laboratories/SpiralCore/issues/44#issuecomment-6867302904). The Windows `SenderReplyWait` observation conflated three distinct branches. This diagnostic assigns a bounded category at each existing branch; it preserves deadlines, errors, reply consumption and wire bytes.

| Existing branch | New observation |
| --- | --- |
| The absolute budget is exhausted before calling receive | `SenderReplyExpiredBeforeReceive` |
| `recv_timeout` returns Timeout | `SenderReplyReceiveTimeout` |
| A reply arrived but the subsequent absolute-budget check fails | `SenderReplyExpiredAfterReceive` |

Disconnected receive still returns `Closed`, which remains excluded from retained failure observations. The original first-failure record still wins. No clock, worker, queue or sampler was added. The macOS fixture reports the failing handle's existing observation at each initial write/read assertion before dropping the handle.

## Actual before/after control

The existing full-queue fixture fills four real deliveries, then withholds the fifth Commit acknowledgment. Its original 180 ms sender deadline, two-second upper bound, frame-kind/timeout checks, cancellation, both joins and fenced-delivery checks remain. The tests-first checkpoint is `3328b40c`.

The exact selector is `duplex::tests::full_delivery_queue_backpressures_then_times_out_without_extra_published_ack`, run through the standalone FlowMux crate's `cargo test --locked --lib` target on Linux x86-64, Rust 1.97.1, using the existing shared target.

- **Before:** compiled successfully, then 0 passed, 1 failed, 31 filtered in 2.11 s. The actual site was `SenderReplyWait`; the required site was `SenderReplyReceiveTimeout`. Original test executable SHA-256: `e51c68371dcf64432c7e8cc99225b723eb5f961426f8fe3e63c1489197021eee`. Raw log SHA-256: `897e77de54fc2071ec83975e7afc8066068be06490f01c1446fd530031758974`.
- **After:** the same selected fixture passed 1/1, 31 filtered, in 2.05 s. The complete default library then passed **32/32**, and the stall-feature library passed **33/33**, with zero failures or ignored tests. Strict `cargo clippy --locked --all-targets -- -D warnings` passed in both configurations. Independent code, Rust and security source review passed.

The observed runtime control covers the channel timeout branch. The before-receive and after-item deadline boundaries retain source-level attribution only; the current I/O fixture cannot force them independently without scheduling assumptions. The fixture's elapsed time includes deliberately withheld delivery and is not a transfer-speed benchmark.

Qualified candidate SHA-256:

- `src/failure.rs`: `5140fef12ad978ee9720299684e1583384aa5cfbd6cd6f617213c4ad3c4efa80`
- `src/duplex.rs`: `81b137eb6be573c065acea816ef0e68aa0b6e4ac821b737c198c564f2417a14e`
- `src/duplex/tests.rs`: `5eb44bbe74e4a77e68842c28a3228b36e2b4e2e52e1b595e543e5c6187c3d325`

The old public `SenderReplyWait` enum variant remains available. Adding variants to this exhaustive Rust enum can require downstream match updates. These are local diagnostic categories, not new protocol outcomes or signed meanings.

This change does not establish the cause or a fix for the intermittent Windows/macOS timeout. The original platform failures and PR #260 merge hold remain. The next evidence must come from the unchanged hosted workload with the more precise error observation.
