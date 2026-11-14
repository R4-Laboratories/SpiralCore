# Independent reply progress: retained failing control

Development source: `7cc132d9e2b7381cb5ba5a1e92be1942148e7308`.
This is a deterministic trusted-connected TCP unit control on Linux, not an
authenticated MVM, Windows or WAN result. The production repair is unfinished.

The test pauses the original receiver's inbound authorization at an armed gate.
While it remains paused, that endpoint sends its own independent 1 KiB object.
The peer completes an additional reply record, but the local sender times out at
its unchanged 500 ms operation deadline. The gate is released and both original
worker handles are physically collected before assertions. Collection is not a
successful protocol completion claim for this failed transfer.

Command: `cargo test --release --locked --offline --manifest-path
crates/spiralcore-mvm-flowmux/Cargo.toml --lib
independent_reply_progresses_while_original_inbound_authority_is_held --
--nocapture --test-threads=1`.

Observed: 0 passed, 1 failed, 32 filtered out, 0.52 s; Cargo exit 101. Peer TX
increases from 222 to 408 wire bytes, 102 to 168 payload bytes and 1 to 2 complete
records before gate release. This establishes a synchronous reply-routing
dependency. It does not establish the cause of the historical Windows timeout.

Executable: 6,123,936 bytes, SHA-256
`8e23e367a41978e4f8b15c91abf512375a1ccbae89612e121174e2d21ce6c6e6`.
Retained [raw failing output](red.log), SHA-256
`73c3d1f5793b2e78c93443a5d8c9482353f0d474e8782f092d7988dd0fcaf388`.
No VM was booted and no large payload corpus was retained.


## Development repair and retained refusals

The repair shares the original decoder under one lock. A waiting caller may
assist only while the original worker handles inbound work. It classifies a
fixed nine-byte prefix after the canonical outer-header validation, then uses
unchanged full-frame decoding and pending-operation correlation for acceptance.
Incoming requests and control records remain in sequence for the original
worker. That worker alone owns authorization, endpoint/cache mutation and delivery.

Consumed prefixes retain their original deadline and frame-budget charge.
Cancellation, reply publication and inbound completion wake the enrolled caller;
full scope checks run outside the event gate. Original-worker termination takes
and drops Rx even when the caller retains its routing reference.

SmallV1 explicitly reserves another 8 KiB of fixed metadata: receiver metadata
is now 328 KiB. The existing 8 MiB operation ceiling remains enforced. The
concurrent core-frame peak is at most five full-sized allocations: held inbound
request, prepared reply, encoded reply, assisting raw frame and assisting decoded
frame. This fits the existing six-frame receiver allowance. The decoded frame is
moved into the single response slot; removing pending before publication prevents
another assisting decode. Sender and TLS/carrier workspaces retain their separate
existing charges. This is an allocation accounting argument, not an RSS limit.

The first candidate passed the progress condition but one full-library run later
failed on peer cancellation. Its cause is unresolved; subsequent passes do not
remove that observation. The source-revision-specific final results must be
consulted separately before treating this as a qualified repair.

Native run [43619933134](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43619933134)
at `f0e28718427f14806bbf7b3ad4485d4293474065` exposed an error-custody regression:
the original worker join did not preserve the required Limit refusal when the
assisting reader detected the frame bound first. The assertion printed no actual
returned variant. The [selected original failure](native-refusal-failure.json)
retains that negative evidence. The assertion remains mandatory.

Successor implementation `a3da1ce0` retains the original decoder error while
holding Rx; the original worker synchronizes with that ownership before consuming
it once. Consequential cancellation/close cannot hide that refusal. An independent
worker authorization/integrity/storage failure keeps its own result. No deadline,
wire format, cache semantics, extra reader or authorization route is introduced.
PR #278 remains a development checkpoint until final-source and platform checks
pass. PR #260 and the installed MVM, WAN, scale and FIPS gates remain separate.
