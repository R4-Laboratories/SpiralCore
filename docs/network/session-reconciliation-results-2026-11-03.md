# Explicit reconciliation of incomplete canonical mailbox exchanges

SpiralCore now exposes `campaign::sender_with_broker_and_reconciliation` for an embedding application that needs to recover a missing destination receipt or application Reply. Each invocation supplies one opaque, capacity-one signal. An application may explicitly request another send of the **same prepared Call**, under its original authorization, deadline and tracker identity. Idle alone sends no retry. Existing guest entry points remain strict.

[Source PR #66](https://github.com/R4-Laboratories/SpiralCore/pull/66) merged at `d7a01d7e9c8b59b13a861420a85bcaa0c99d8d97`; its tree matches the reviewed and hosted-tested tree.

[API contract](../api/session-reconciliation-v1.md) · [Fixed write-deadline contract](../api/stream-send-deadline-v1.md) · [Implementation plan](../plans/2026-11-03-session-reconciliation.md) · [Compact evidence](evidence/2026-11-03-session-reconciliation/README.md).

## What the endpoint does

The caller retains its exact encoded request, original handles and receipt/Reply/ACK facts. Its opt-in receive loop returns local Idle at a 250 ms granularity, checks the original lifetime, and checks ordinary FIFO completion before consuming a command. When a predecessor blocks delivery despite all current completion facts, the queued command stays queued and the current Call is not retransmitted. The service owns the Reply until ordinary one-time extraction.

A receiver can authenticate an older Call without repeating object work. If it no longer owns a cached Reply, it sends only its actual Duplicate admission receipt; absence of a cache does not prove application completion. While it still owns the exact current Reply, it replays that Reply **before** the closing Duplicate Call receipt. This order permits the caller to ACK the replay before finishing on its missing receipt. Both writes use the original lifetime, with fresh owner/binding/cache/tracker checks after the potentially blocking replay write.

The original plus seven retries exhaust eight Call-send invocations. Complete incoming frames count toward one sixteen-event ceiling across Idle and replay. A sixteenth fresh receiver Call refuses before new application admission. Failed or uncertain sends retain identity and fence reentry; there is no reconnect or replacement request. All eight exchange-owned Call, Reply and receipt-write sites now carry a fixed original deadline through connection and partial writes, rather than receiving a new parent timeout.

This is one canonical MVM protocol and the existing SpiralCore object/codec core. It adds no envelope, port, authority domain, guest NIC, automatic timer or fallback route. The public live crate is a development witness; public source reachability does not imply that native guest entry points choose the new API.

## Executed correctness and failure cases

| Qualification | Observed result |
| --- | --- |
| Missing Call receipt after receiver completion | Exact original retry obtains actual Duplicate receipt only; original Reply returned once; object executes once |
| Missing Reply while receiver waits for ACK | Exact cached Reply replay, canonical ACK, released cache and one object execution |
| Missing initial receipt and first Reply ACK | Reply-first reconciliation allows both peers to complete and release retained Reply state |
| Direct and production relay | All three scenarios pass directly and through the actual independent-direction relay: six tests |
| Real public API | Three TCP campaigns pass healthy/missing Reply/missing receipt cases, exact durable publication and independent reopen; signals expire between invocations |
| FIFO, event and authority bounds | Ready-but-Pending preserves queued command with zero retries; fifteen old controls plus fresh sixteenth Call gains no application state; changed generation/expired original lifetime refuse before connection |
| Socket writer lifetime | Late Call retry, Reply ACK, initial Reply and cached Reply partial writes stop under the original approximately 900 ms signed cap despite a two-second parent; actual partial counters and retained state recorded |
| Explicit attempts and silence | Eight byte-identical Call frames; ninth command refuses before transmission. Multiple Idle outcomes without commands send one original Call and zero retries |

The deadline matrix uses bounded scripted `SocketIo` implementations honoring installed timeouts. It qualifies the actual canonical writer loop, not OS TCP backpressure or comparative performance. Fault gates withhold registered complete ciphertext frames after the existing handshake, count actual successful reads/writes on each leg, and retain the withheld frame length/hash. They model lost protocol events, not TCP packet-loss behavior.

The retained RED history includes missing APIs, the original receiver's refusal of an historical Call, missing processing counters, the combined receipt/ACK race and the old raw-retry write exceeding its original deadline. The late-write old run uses an explicitly documented mixed source snapshot: old caller file plus current test-module declaration and other current files. It is not a benchmark of the full historical tree.

Three fixture corrections remain visible: old receiver scripts expected receipt-before-Reply; new writer fixtures independently resealed requests despite canonical fresh nonces; and the ACK assertion expected direct ServiceError rather than ReceiptSendError::Service. Assertions retain exact bytes, typed outcomes, original lifetimes and accounting. The initial hosted Windows Server 2022 row also exposed a fixture-phase error: software/debug canonical authorization consumed a 100 ms I/O deadline before assumed prefix/body progress. The portable transport check now authorizes the genuine frame first and starts its 100 ms view immediately before connection/writing; exact counters, typed refusal, fencing and deadline assertions remain. Full service-path signed-lifetime tests retain authorization within their original deadline. The failed original ZIP/API digest and 17 inner checksums remain in `qualification/initial-failure`; the final Windows row passes. One redundant-clone Clippy correction is test-only. The packet preserves failed runs rather than presenting these as additional product defects.

## Recorded traffic, CPU and transfer time

The strict transfer comparison uses the unchanged Rust `broker_objects` engine, a persistent TCP-loopback session, three fixed paired trials per arm, eight common application targets and a fresh process per cell. Combined also pays for its checkpoint and schema publications. Twelve before/current cells pass **108 exact publications and 108 independent object readbacks**. The actual common target is **1,047,202 bytes**; the requested fixture tier is 1 MiB. Current peak process RSS is at most 36,560,896 bytes.

The current Combined path retains substantial session byte savings against whole-object zstd:

| Current mixed session, median of three trials | Whole-object zstd | Combined | Median paired change |
| --- | ---: | ---: | ---: |
| Both-direction canonical bytes | 7,351,454 B | 4,930,219 B | -32.935% |
| Aggregate process CPU | 591.385 ms | 498.841 ms | -15.649% |
| Sender thread CPU | 324.952 ms | 276.180 ms | -15.009% |
| Receiver thread CPU | 265.464 ms | 221.713 ms | -15.106% |
| Verified transfer time, including compute | 595.057 ms | 518.749 ms | -13.192% |
| Full lifecycle time | 608.634 ms | 532.919 ms | -12.839% |

The final column is the median of paired relative changes; it differs from a ratio of the displayed column medians. The strict current-versus-prior paired medians are +0.765% aggregate CPU / -0.124% verified wall for whole zstd, and +0.156% / +0.026% for Combined. These exploratory observations do not establish statistical noninferiority or a causal improvement from reconciliation.

Individual workloads still lose: Combined schema-fit is +6.909% CPU / +6.612% wall at the paired median, unrelated content +1.150% / +0.821%, and schema-drift +0.590% / +0.724%. Combined's median paired RSS is 18.366% higher. Every trial and auxiliary-domain loss remains in the [measurement review](evidence/2026-11-03-session-reconciliation/measurement-review.md) and [machine-readable summary](evidence/2026-11-03-session-reconciliation/summary.json).

Sender and receiver thread CPU are reported separately from one aggregate process CPU measurement. Verified transfer time includes compute, durable publication and receiver readback; provisioning, independent offline readback, cleanup and full lifecycle retain their separate fields. Overlapping process-clock operation intervals must not be summed as endpoint CPU.

Bytes count canonical encrypted envelopes, framing, initialization, schema/checkpoint setup, requests and receipts in both logical directions. They exclude TCP/IP headers and retransmission. Random canonical encryption/key JSON representation makes outer lengths vary slightly across processes. Common target identities/lengths match; logical codec payloads and actual stream counters remain independently recorded.

These measurements exercise the normal strict path: receiver preamble, bounded writes and reporting changed there. They do **not** measure opt-in channel/Idle overhead or recovery speed. Functional recovery rows contain actual traffic and CPU/wall observations, with post-frame processing distinguished from lifecycle, but are not a matched zstd performance experiment. Three exploratory pairs do not establish WAN acceptance, tail latency or statistical noninferiority. No send-only or energy inference is made.

## Qualification and source custody

Final source is `b27fbba9bdc636575266d6da04ec8f1ad5c7c2ea`. On Wick, locked Rust 1.97.1 release passes **111 live checks**, **130 adapter checks** and **8 object-v1 checks**, zero failures/ignored. These scopes overlap. Two live and two adapter zero-test scopes are retained explicitly. Strict live Clippy and the default Linux guest-entry build check pass. The 249-entry source manifest verifies before and after. Independent scoped source Rust and security reviews pass; [hosted Linux](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41677859354) and [portable-client](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41677859307) workflows pass at that exact head. Seven native client rows pass **95 checks each (665)**, with **eight additional Unix storage checks**. Linux passes 111 live plus 138 shared checks (130 adapter + 8 object-v1), strict Clippy, default guest entry and analyzers. Counts overlap; all zero-test scopes remain explicit. The tested merge `7c10e0f063d1b513881b3ee3d4c9975504d90e11` and reviewed head have identical tree `2dc76d48038762670314ac929c4dda171f042a80`. All seven original ZIP/API SHA-256 digests and 236 inner checksums verify. Actual source merge `d7a01d7e9c8b59b13a861420a85bcaa0c99d8d97` has the same tree; [publication receipt](evidence/2026-11-03-session-reconciliation/publication.json) records all three identities.

Selective line coverage passes **239 tests in 29 fresh protected objects**, zero failed/ignored/filtered. Changed mapped executable production lines are **350/377 (92.84%)**; the five entire mapped production files are **2,155/2,308 (93.37%)**. All uncovered lines remain recorded. Test suffixes and the test-only orchestrator wrapper are explicitly outside that production metric. Branch mapping is **NA**, not 100%; no branch, MC/DC or whole-project claim. Every instrumented executable was copied and protected before execution; profiles and exports were isolated per binary. Source checks pass before/after, compiler warnings and test/export stderr are zero, and the normal measurement binary remains unchanged. [Final-source coverage procedure and receipts](evidence/2026-11-03-session-reconciliation/coverage-next/README.md). The earlier `coverage/` directory preserves its separate pre-correction source, with the same scoped line counts; it is not relabelled as final-source execution.

The normal comparison binary `fe7937f2fb348ff0a3c517cf856f2ad132883346a8aa05020e6dbd0e1d62b6d7` was built before later fixture-only corrections; its production files are identical. Its baseline is the protected strict engine from source `960e73c5`, binary `04a57c32154acad6dbc3754775039aad80cab632e77f9d50dce9965d52ad8d35`. Instrumented coverage timings are never used for performance. Compact records are local; raw profiles, executable copies and build exports remain on Wick. No new VM disk or local Cargo target was created. Successful owned payload stores are removed after flushed verification records; historical failures remain.

## Remaining delivery gates

This delivers an explicit incomplete-session consumer, not full C8 or production MVM delivery. General loss of the caller's last ACK **after the caller completes**, retained completed-tail ownership, closed-stream resume and native guest opt-in remain. The 2,048-Call guard, pipeline one and existing host budgets are unchanged. C9/C10 production retention and longer same-owner sessions, installed host ownership, shared plaintext/cache partitions, fleet fairness/capacity, canonical WAN and final upstream contribution remain open under [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42), [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43) and [#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).

The full live consumer is not qualified on Windows: its current forced hardware-SHA dependency profile and Unix-private object directory need explicit portability work. Historical seven native adapter-client rows qualify their own scope, not this new live consumer, durable Windows storage or a Windows MVM backend. Current-source seven-row results remain adapter/client qualification with those same limits. [#46](https://github.com/R4-Laboratories/SpiralCore/issues/46) and [#47](https://github.com/R4-Laboratories/SpiralCore/issues/47) retain native-role and real-WAN requirements. No FIPS/CUI certification or scale claim follows.
