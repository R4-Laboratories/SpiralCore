# Current-MVM disk custody integration

Date: 2026-11-02. Tracker: #43. The retained two-guest Linux attempt at source
`2a4a199` failed before receiver boot: current MVM refuses persistent writable
Firecracker disks without prepared descriptor custody. The original failure
and successful one-guest results remain in
[the intermediate report](../linux-native-intermediate/README.md).

Linux now acquires canonical `LockedWritableImage` from a clone of the already
validated backing descriptor and its canonical sidecar. Sidecars are opened
without truncation with `NOFOLLOW`, `CLOEXEC` and mode `0600`.
`PreparedWritableDisks::bind` validates slot coverage and pathname identity;
`VmmDriver::boot_custodied` performs the actual writer handoff. Sender and
read-only verifier boots use empty custody. The original backing descriptor
remains available for explicit synchronization after the owned VMM exits.
`trusted_builder=false` and the resource gates are unchanged.

The Mac development worker uses the low-level HVF API. It now retains canonical
`LockedWritableImage` backing/sidecar ownership through boot and final sync,
constructing writable and read-only `DiskImage` values from the exact validated
descriptor. It no longer validates one file and then reopens a potentially
different pathname. This is cooperating-writer exclusion and descriptor
continuity for the development witness, not production workload admission.
Adopting the high-level production driver requires its `boot_custodied` API.

Focused independent Rust/security review approved both implementations:

| File | SHA-256 |
|---|---|
| `src/linux_host.rs` | `a4f73545405c2c04bb4ed71dd0ab5853d1fc19efe4676b92eb676e674ce2f917` |
| `src/host.rs` | `d2b091bfe691d7d7766de0603d2c0005bf8fe4e453bfd5244102872398afc89e` |

Linux tests first failed to compile because the new helper did not exist.
The implementation then passed three focused custody tests and the full
19-test Linux library suite on Wick using Rust 1.97.1 release and the locked
public dependency. Strict Linux-host library/test Clippy passed. Cases include
empty sender/read-only custody, persistent receiver ownership, competing writer
`WouldBlock`, lock metadata preservation, descriptor release, replaced backing
pathnames and sidecar symlinks.

Mac tests likewise observed the missing-helper RED, then passed 12 selected
host tests (nine Mac host tests including three new custody cases, and three
existing Linux parser tests). Strict release library/test Clippy passed.
New cases cover writer exclusion through device/guard release and final sync,
read-only inode/byte continuity after path replacement, wrong size/type and
backing/sidecar symlink refusal. Existing canonical primitives supply the
advisory lock; these tests do not claim to prevent noncooperating writers.

Actual guest qualification must rebuild from this frozen source and use a new
result directory. Unit tests and source review do not convert the earlier failed
two-guest attempt into a success. No mailbox envelope, codec, crypto, guest NIC,
network permission or protocol format changed in this correction.
