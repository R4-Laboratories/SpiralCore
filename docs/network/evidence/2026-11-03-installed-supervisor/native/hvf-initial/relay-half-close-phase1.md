# Native macOS final half-close — scoped Phase1

Source inspected: crates/spiralcore-mvm-live/src/orchestrator/duplex.rs,
SHA-25690c2cc67e7fc40b6a8310cd0511dbaf3cdd1d792f8bb261013edeb6664656e92.
The pump forwards complete canonical frames until read_ciphertext_frame_or_eof
returns a clean prefix-offset0 EOF. It then executes output.shutdown(Write)? at
line90. That final syscall's error is currently reported as a failed direction.
Any earlier read, route, write, timeout or panic is separately fatal and must
remain so. The existing clean_half_close test leaves both application sockets
open through relay completion, so it does not exercise full peer close before
the sibling pump's final shutdown.

Observed failed cell: native/two-whole-mixed-1048576. Both guest records say
ok=true, backend_exit_confirmed=true, stopped_by_watchdog=false. Receiver contains
8 verified events (positions0..7), each exact=true with1,047,202 actual bytes, and
receiver_done exact_reopened=true/operations8. Both guest_done records say
ok=true. Sender_done and receiver_done counters match the relay report exactly:
146 frames each direction;6,637,688 forward envelope bytes and707,495 reverse;
592 framing bytes each direction. The host budget reports zero final links,
slots and serialized buffers. The relay report is forward:null,
reverse:"Socket is not connected (os error57)" and session_failed.

These independent semantic readbacks and complete matching counters support a
teardown-only failure for this cell, not lost object bytes. The report does not
include an I/O phase tag, however, so the precise syscall is not directly pinned
by the JSON. Terminal shutdown(Write) is the strongest source-supported hypothesis;
it must be reproduced with a real native socket before a fix is called proven.

Proposed deterministic real UnixStream regression on native macOS:

1. Start the existing actual relay fixture and complete both handshakes.
2. Send/drain the final reverse frame byte-for-byte. Half-close A's write side
   and observe propagated EOF at B, proving the forward pump reached clean EOF.
3. Fully drop A's LAST application-side FD before B write-half-closes. In the
   existing Drop-owning RunningRelay helper, replace its A field with a separate
   inert UnixStream::pair endpoint before dropping the original, rather than
   relying on Darwin's potentially failing Both shutdown. Hold the spare dummy
   endpoint until teardown; it never participates in traffic.
4. Half-close B's write, then require relay autonomous completion before any
   test-side rescue shutdown, exact counters and zero released resource usage.
   The old native-Mac path should fail at final reverse shutdown with ENOTCONN;
   Linux may already pass and is not a substitute for the actual Mac RED.

A candidate compatibility rule is narrowly terminal shutdown NotConnected AFTER
proven clean input EOF; it must not cover any read/forward/write error, incomplete
frame or partial write. Existing malformed/truncated/reset/timeout and reverse
buffer-drain tests remain strict. No broad IO-error ignore, relaxed fidelity,
window increase, product/test edit, build or native rerun was performed here.
Root owns reproduction, implementation and fresh qualification.

Receipt SHA-256 pins:

- sender.json: f72d82ab9b26bbd85999ffb6ba0547b346d1f930b6ea7fb6e1df30bce8fdf62c
- receiver.json: 9dd882d065da8cbde9265717e5de8f3a1a594784e669845994afb6c1022cc208
- host-events.jsonl: 6c8279536c42c1dca91bb7a206d4520729518a74d96e09226e49f450f2012519
