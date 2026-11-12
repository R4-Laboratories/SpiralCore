# Installed mailbox components for two guests

A two-guest Cast needs an admitted sender, a host that owns both workers, and an admitted destination. Each measurement below exercises one of those interfaces. The installed same-host acceptance test still requires two booted MVMs exchanging one object, a verified read at the destination, and collection of both original workers. A two-host WAN test follows that gate.

| Interface | Executed result | Where the measurement begins and ends |
|---|---|---|
| Guest Submit on macOS | One 3-byte Cast was submitted six times. The host read 1,338 bytes and wrote 1,022 bytes in total. The six exchanges took 0.305 ms. The entire test process used 4.798 ms of CPU, including its harness. | Connected Unix socketpairs, exact guest request read on every attempt. The sixth attempt completed local publication after a matching duplicate verdict. [Bytes, time, process CPU and binary hash](guest-submit.json) |
| Destination Cast on macOS | The synthetic destination received the exact 5-byte body. The host wrote 276 stream bytes and read 98. Dispatch through the inbox acknowledgment took 0.389 ms. | One Unix socketpair exchange after canonical verification. Host thread CPU was 0.331 ms and guest thread CPU was 0.024 ms. Sealing, opening, journaling and guest boot are outside the timer. [Recorded sample](destination-cast.json) |
| Installed worker coordination on Linux x86-64 | Six focused host/SDK tests passed. Both production host executables built from pinned source. | Separate names and signed acquisitions share a 1 MiB protocol budget. The tests cover an unauthorized third name, crossing worker ownership, queue/replay accounting, and release of one worker while its sibling remains live. [Process timings and binary hashes](linux-workers.json) |

The Submit fixture drops a response, returns Full, changes the digest, changes the service generation, injects a local publication failure, then returns a matching duplicate. The result callback receives the original length and bytes. Separate cases exercise the binding, connection generation, result length, request ID and the distinct Call, Cast and Reply correlations. The first published result stays immutable after an unlink failure. On retry, the original encoded fields must all agree. The only allowed change is AcceptedEphemeral becoming DuplicateEphemeral. The regression rejects a changed message ID and a reversed disposition transition. The installed caller still has to persist that result before releasing the original claim. Ari's destination journal owns durable admission and the downstream receipt. [Accepted upstream division of responsibilities](https://github.com/tinylabscom/mvm/pull/5328#issuecomment-6815760571)

## Reproduce the component tests

The [guest Submit source](https://github.com/Casey-allard/mvm/commit/f4e34349e6bd44c3e4d3c4d381ceee8eab241cf2) includes the [destination Cast mapping](https://github.com/Casey-allard/mvm/commit/67822eba637bdc380f4d3af0b163a2763aae2e71). Run the following from that checkout on a Unix host with the pinned dependencies available.

```sh
cargo test --locked -p mvm-core --no-default-features --features test-support \
  --test mailbox_submit_exchange --test mailbox_local_frames --test mailbox_cast_endpoint \
  -- --nocapture
```

The [Linux coordinator source](https://github.com/Casey-allard/mvm/commit/4f8bb1bf7abd4eeaac8a836f8f301b6a3e150852) is a separate integration branch. The Linux results file names each test, binary and process CPU interval. Each named lane has an 8,192-byte frame body ceiling, four queued items and 64 lifetime admissions under the shared budget. For the legacy outbox, the body includes a 32-byte recipient, leaving at most 8,160 application bytes. The four-byte length prefix is additional. Installed reliable-role activation, persistent-session recovery, cross-host delivery and platform qualification remain open under [SpiralCore #43](https://github.com/R4-Laboratories/SpiralCore/issues/43).

## Replay-capacity release gate

Independent review found a fidelity defect in the six-test Linux revision. After
64 admissions, its worker reported replay exhaustion using a general mailbox
refusal. The legacy guest forwarder removes its original file for that refusal.
Draining the plaintext queue leaves the replay budget exhausted, so the next
item can be lost. This original revision remains unsuitable for deployment. The corrected revision and regression outcomes follow. The composed regression
must deliver the first 64 items and check that the 65th original remains
byte-for-byte available after the refused attempt. The timers above describe
the earlier component tests.

The legacy guest writer also permits a larger item than this opt-in host
profile admits. The 8,160/8,161-byte boundary requires an explicit test for
pre-publication refusal or retention of an already spooled original. Neither
component timing qualifies general guest traffic above the signed bound.

## Guest outbox custody after the fix

The production host service and guest forwarder were exercised together over Linux socketpairs. On the original source all three tests found a missing guest file. On the corrected source all three passed. Each retry compared the surviving file with its original bytes. [Failing revision](https://github.com/Casey-allard/mvm/commit/048bb293a8b26fd22dae29214f88e3af68306c8e) · [Corrected MVM revision](https://github.com/Casey-allard/mvm/commit/628b66d39e53ce1b4429a07197bc2d50e2b084ed).

| Test | Result on corrected source | Test wall time | Test process CPU |
|---|---|---:|---:|
| Lifetime replay budget | Delivered 64 items. The 65th remained byte-exact after two refused attempts. | 39.099 ms | 38.666 ms |
| Frame-body limit | Delivered 8,160 application bytes. The 8,161-byte original remained after two refused attempts. | 3.848 ms | 3.810 ms |
| Ambiguous reply or unlink failure | Retained the original after EOF, partial reply, timeout, write failure and oversized reply. A failed unlink after acceptance surfaced to the caller. | 2.416 ms | 2.349 ms |

The reply reader limits verdict data to 4,096 bytes plus a one-byte oversize check. An explicitly decoded terminal refusal retains its original semantics. Legacy replay after a lost acknowledgment can deliver a message more than once. All timings cover a complete Linux test process. Byte savings, guest execution time and WAN throughput require different trials. [Full recorded results](custody/results.json) · [Evidence checksums](custody/evidence.sha256).
