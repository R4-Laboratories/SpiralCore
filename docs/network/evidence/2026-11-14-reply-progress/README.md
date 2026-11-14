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
