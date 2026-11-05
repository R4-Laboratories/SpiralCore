# Descriptor-relative mailbox socket cleanup

This successor checks and removes the mailbox socket with `fstatat` and
`unlinkat` on the original held private-directory descriptor. It verifies
the socket type, device and inode before removal. Retirement requires
directory sync, current named-directory identity and observed absence
through the held descriptor. Linux bind still uses the proc-fd alias.

Nine protected Linux x86_64 tests passed. The new case renames the original
directory while its socket is live, then verifies that relative observation
still identifies the same socket and that cleanup refuses until the named
directory is restored. The same protected binary replayed the canonical
65,023-byte Call, Reply and encrypted receipts.

The patch applies to `worker.rs` from the preceding Unix worker witness.
Replay reproduced the exact reviewed source. Code, Rust and security review
passed for the staged scope. Socket-entry mutation must remain exclusive
to the cooperating Runtime owner. Aarch64 received source review only.

- [Descriptor-relative patch](descriptor-relative.patch)
- [Test-first compiler result](red.txt)
- [Protected test output](results.txt)
- [Source and executable identities](pins.json)
- [Original Runtime slot contract](runtime-slot.md)
