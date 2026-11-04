# Native macOS clean-close correction

Independent native collaboration reviewer `/root/retained_call_review` reviewed
the root-authored source delta and delivered CODE/RUST/security PASS. The
NotConnected exception runs only after clean frame-boundary input EOF; payload
reads, framing, writes and other shutdown failures remain fatal. No authority,
resource-refund rule, codec or serialized envelope changed.

Reviewed source SHA-256:

- `orchestrator/duplex.rs`: `d1f47cb317c4b93e9ba3795c7fdf90b5f06d57c35ecf48a03ac39706b7d83801`
- `orchestrator/duplex_tests.rs`: `54820c25ba5c5d7ba17e6f66e47587c9c255c03ca749495c94321cf0dcf78990`

The new real-socket clean peer-drop test reproduced Darwin ENOTCONN before the
product change. The original blocked-write fixture separately failed on unchanged
f1 source: shutdown(Both) retained the peer descriptor and did not cause the
intended write failure. Full descriptor drop supplies that event. A directly read
64-byte exact body proves real partial kernel delivery before closing. Linux
keeps the existing positive successful-write-return requirement. macOS can report
EPIPE with no positive partial-write return; its distinct test requires exact
observed body, unfinished frame, fatal forward error, autonomous completion within
the same three-second watchdog, and zero retained transport credits. It does not
invent successful-write counters or call them physical traffic measurements.

Actual final focused suites: seven native macOS tests and seven native Linux tests
pass, with no failure or ignored entry. Native macOS complete all-target release
suite passes148 entries across12 targets, including two zero-test binary/example
targets; those zero targets are not additional coverage. Strict all-target macOS
release Clippy passes with warnings denied. Those scopes are overlapping, not a
combined independent test count. Original failures remain retained.

Four subsequent native HVF two-guest cells (matched whole-zstd/Combined at1MiB
and16MiB requested tiers) report exact reopened objects, positive backend exits
and Clean installation retirement. They are local fixed-order native observations,
not WAN, native HVF supervisor-death recovery, fleet or certification evidence.
