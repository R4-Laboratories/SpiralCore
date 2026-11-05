# Installed mailbox Unix socket worker witness

This staged Linux worker holds a pinned private directory, listener, wake
descriptors, original thread and fixture charge. It fences custody before
bind, wakes accept and partial reads on stop, and preserves custody across
timeout, uncertain startup or cleanup refusal. Successful stop joins the
worker, verifies and removes its own socket, synchronizes the directory,
and closes the remaining handles before the owner can release its charge.

The protected executable passed eight tests. A local peer sent a canonical
Call with a 65,023-byte payload, received the exact Reply and exchanged both
encrypted receipt Casts. The peers agreed on each direction's envelope
counts. This run recorded 124,478 envelope bytes from the caller and
123,977 from the acceptor, plus 16 framing bytes in each direction. These
are adapter counters for one fixture exchange, without a baseline or
transfer benchmark. Synthetic keys and a fixture charge were used.

The shutdown tests cover accept and partial-prefix wakeups, timeout and
retry, replacement refusal, natural EOF, retained charge and live listener
after injected observation failure, a simulated composite API failure after
socket creation, and charge release after verified cleanup. The raw-socket
natural EOF regression failed on the first candidate. A later test asserted
worker exit before thread completion was synchronized. Its failure and the
corrected final run are both retained.

- [Worker source](worker.rs)
- [Canonical test template](worker_canonical.rs.in)
- [Final test output](ready-final-results.txt)
- [Source, dependency and executable pins](ready-final-pins.json)
- [Original EOF regression](eof-red.txt)
- [Superseded timing-sensitive test output](reviewed-results.txt)

To replay, use the pinned adapter checkout at
`9153dfe20be9518a2277519c312dd0709664610d` and substitute its absolute
`crates/spiralcore-mvm-adapter/tests` directory for both
`@ADAPTER_TESTS@` tokens in the test template. Save it as
`worker_canonical.rs` alongside `worker.rs`. With Rust 1.97.1, run
`rustc --edition 2024 --test worker_canonical.rs` with `-L dependency`
and one `--extern` for each pinned rlib. Run the binary with
`--test-threads=1 --nocapture`.

Code, Rust and security review passed within this staged scope. Cleanup
assumes exclusive cooperating ownership of the original private directory
and a stable Linux proc-fd namespace. Runtime integration still needs an
actual funded grant, descriptor-relative cleanup or an explicit procfs
stability check, cancellable rate and budget waits, and signed guest
bootstrap. This witness started no guest.
