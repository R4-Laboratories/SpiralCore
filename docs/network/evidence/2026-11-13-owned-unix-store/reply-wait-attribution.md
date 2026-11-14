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

The original platform failures remain recorded. The following hosted results qualify the diagnostic on the unchanged workload.

## Hosted successor

PR head `c26b5d2c2c4935a50b1b06f6d181fea9442bfae3` ran as merge checkout `e14c1f57be42f569e1a58defb7c4e325681486d3`. The ordinary Windows Server 2025 x64 test stopped at the **240-second workload deadline**. Both writers reached the deadline check before starting block 3944. Each direction had acknowledged and verified 3,944 of 4,096 blocks of 16,384 bytes (64,618,496 bytes each way). Both retention snapshots showed 64 publications and 3,880 evictions, and both scope checks passed. The failure snapshot recorded 242.012476 s wall and 115.687500 s process CPU. Neither writer reported a sender-reply failure. The readers subsequently reported `CallerRead/Timeout` and `WorkerExit/Io(UnexpectedEof)`. The ordinary job recorded the Cargo executable name `ordered_stream-c43c3372c7fdd5c2.exe`; its binary SHA-256 was unavailable. [Windows Server 2025 job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43530853202/job/132248013601)

The ordinary Intel macOS job passed the previously failing simultaneous-duplex test and verified all 4,096 blocks each way (95.927427 s snapshot wall, 57.421218 s process CPU). Two separately compiled Server 2025 variants also finished all 4,096 blocks each way: `witness-timing` in 215.982573 s wall / 122.703125 s CPU and `witness-stall` in 181.513092 s / 83.312500 s. Their executable hashes matched before and after each run. These measurements leave the source of the ordinary job’s elapsed time unresolved. [Intel macOS job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43530853202/job/132248013800) · [Timing job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43530853121/job/132247882507) · [Stall job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43530853121/job/132247882935)

Each downloaded ZIP matched its GitHub artifact digest. All members listed in each `SHA256SUMS` matched their recorded hashes.

| Artifact | ZIP SHA-256 | `tests.txt` SHA-256 |
| --- | --- | --- |
| [Windows Server 2025 default](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43530853202/artifacts/14087633789) | `cf6aa8e9b1f36764d6bd08582c276b481406a1b48ed8aa1e264e7c78f8bde1d4` | `e8cdb8e31f31a9c4f9a8cd70048aac8b61931b96770deba9f621c107fb966c6c` |
| [Intel macOS default](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43530853202/artifacts/14088098454) | `af85cd70e886820577c21dd9884550099c0e44ffbf047f87dbda0d2ebc5f9288` | `bd0d2e1cdeff8a40cfb17d89a0ff8dba6d84c121d6887c027276261049afd735` |
| [Windows Server 2025 timing](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43530853121/artifacts/14087537520) | `cfd0f86d4c6e91c1602cfab89ecba1d2fbf978b311afefc175e146348396bff0` | `92f7a7b9733391ad04cf135c1f56c22c6370ae5b7fa1ba255afbb13103b5b38d` |
| [Windows Server 2025 stall](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43530853121/artifacts/14087467439) | `4277cbfb4283cafce76ac63789ddbce020a14217e72485d4564e942f0a2a26ec` | `6636ae26c52e458411b2f436c0110ebc0a899ce25f44d8782bb03fdc3fb397be` |
