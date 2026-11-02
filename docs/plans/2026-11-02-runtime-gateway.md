# Implement runtime/warm gateway channel

Contract: ../specs/2026-11-02-runtime-gateway.md. User approved the integration;
execute without new permission checkpoint. Base420e511, existing isolatedworktree.

1. Core owner: RED tests in gateway-core/tests/object_runtime.rs; add runtime
facade and real selection receipts in src/object.rs. Preserve existing APIs.
Risk:namespace/readiness; test missing checkpoint and changed context failclosed.
2. Transport owner: RED tests gateway/tests/object_channel.rs; implement separate
object profile in transport.rs plus bounded object_channel.rs. Risk:auth/partial
I/O; test wrong pin/profile, truncation, bounds, cancellation/deadline/finish.
3. Root: add directtransfer/tokio-util dependencies, object_service.rs/config and
binary; tests first for strictscope/config and real paired durable transfers.
Use blocking workers for synchronous existing codecs with TokioHandle bridge,
never block the network executor. Risk:measurement/authority; actual Endpoint
must verify before commit, and durable reopened readback must match.
4. Measurement owner: fixture example, registered local/shaped campaign andCI;
use exact same files and persistent receiver across tiers. Existing math and
selection remain unchanged; prove actual Mode::RuntimeV1/Warm reuse in receipts.
5. Integrate tests, strictClippy, cross-author code/security/Rust/infra review.
Freeze source beforeCI, retain full outcome, verify/archive and recompute results.
Publish implementation and results separately through reviewed PRs; syncmain.
No transparency claim for oldTUNdatagrams; reliableobjectchannel is explicit.
