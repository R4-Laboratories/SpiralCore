# Installed relay shutdown: source review and required successor

2026-11-04. Independent Root code/Rust/security review of the seven product
files frozen in `owned-relays-origin`. The original patch SHA-256 is
`01a990df8cffe4bf2208e6b0f5cb2b91519d676df2710ea39d8bb431c8504dd1`.
This review does not approve native activation or a whole-pipeline shutdown
latency guarantee.

The same original auxiliary partition, plan/session and host/guest key checks
precede the guest handshake and TLS setup. The factory retains the original
relay witness and thread handles. Foreign equal-looking partition metadata
does not substitute for the actual owner. Existing real blocked reader/writer
tests cover useful I/O shutdown cases.

**HOLD for bounded installed shutdown:** observing/joining the relay handle by
the supplied deadline does not bound dropping its last wake/mapped-stream
owner. That Drop can invoke the core reader's `JoinGuard::join` without a
deadline. Normal two-second blocked-I/O tests do not prove this remaining
case. The successor must retain the same original reader handle on timeout,
explicitly join it under the original deadline, and preserve custody for retry.
No replacement reader, background cleanup thread or replacement quota is
authorized by that change.

**Retention limitation:** timeout quarantine intentionally retains one original
auxiliary/installation owner until the child process exits. A later codec-pool
drain alone does not prove all auxiliary credits/holder leases were released.
Report this conservative disposition separately from positive full cleanup.

TLS setup also needs an actual wake owner immediately after the canonical TCP
connection and before a stalled TLS handshake. An established relay shutdown
witness does not cover pre-relay setup.

These findings were confirmed by the implementation author. An additive
nonconsuming reader-join API and installed cleanup phases are in development;
their tests and exact source require a separate successor review. This file
records source findings, not new transfer bytes, CPU, guest or WAN measurements.
