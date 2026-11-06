# Windows fatal-limit terminal probe

Base: `5a0ab4431bcd408da02fc8f73ace4f392c3e487c`, PR #72.
Corrected `tests/duplex_tcp.rs` SHA-256:
`fd4722a783431efc390d5c55c25bf4c093f9a07081d283edf01f8f2bf77a4b22`.

The Windows negative test received error 10054 (`ConnectionReset`) at the
last EOF probe. The endpoint had already refused the thirty-third request
under its unchanged limit, leaving rejected request bytes unread. The original
test required `read().unwrap() == 0`, which treats the native reset as a failure.
The original hosted output is retained separately from the correction.

The correction accepts exactly EOF or `ConnectionReset` for this last probe.
A response byte, timeout, `WouldBlock`, or any other error still fails. All
32 preceding ACK header/body reads and kind-101 checks, the thirty-third
injection, the local operation failure, and the joined `Error::Limit`
assertion remain unchanged. No product code, admission limit or ACK rule changes.

Independent code/Rust procedural review: PASS. The reviewer compared the
frozen correction with the base and read the retained focused native macOS
result (one test passed, exit zero). That review did not execute Windows;
the corrected source requires the hosted native matrix before release.
