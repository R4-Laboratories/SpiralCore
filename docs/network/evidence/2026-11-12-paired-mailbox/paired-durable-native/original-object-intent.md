# Retrying the original MVM object Call

On Linux, a source writer persisted an MVM Call, released its file lock, reopened the sealed record, and sent it over a Unix socket to B’s local-durable journal. A second exchange sent the identical frame. B returned the same signed V2 receipt both times. The five-byte fixture contains an embedded and a trailing zero (`53 43 00 11 00`). A seven-byte Reply was sealed separately under the lease for that Call. A follow-up test also compares B’s leased application bytes directly against the complete five-byte fixture. [Destination readback assertion](https://github.com/Casey-allard/mvm/commit/3e5b7e76e92ec3c43659e50d036027eef21f697b). [Product and test source](https://github.com/Casey-allard/mvm/commit/fd4b9c54b4ed52f29e86fcd45d4429cd85cd5212) · [Raw trial rows](original-object-intent.json)

| Measurement | Five fresh test processes on Linux x86-64 |
|---|---:|
| Full frame read at B, per exchange | 5,378–5,408 B |
| Signed verdict written back, per exchange | 1,044–1,060 B |
| Source publication, two attempt charges, reopen and verification, median | 0.608 ms |
| First socket exchange through verified receipt, median | 1.098 ms |
| Identical socket retry through verified receipt, median | 0.588 ms |
| Whole test process, median wall / combined CPU | 19.829 ms / 19.775 ms |

Each trial opens a new journal and generates fresh keys. The Rust test reads the length-prefixed bytes at the destination and writes the serialized signed verdict to the socket. Both byte counts are identical between the first and second exchange within each trial. The signed admission covers the original delivery ID and ciphertext commitment. Socket timings start before spawning the receiver thread and end after receipt verification and joining the thread. The source-preparation interval is measured once per trial. Process CPU includes fixtures, authorization, storage and positive and negative controls. The two endpoints share a process. These are local journal retry costs. The installed guest-to-guest warm repeat and canonical WAN transfer remain open in #202 and #47. This fixture compares no codecs.

## Installed host contract

The source owner locks the original custody directory. Its immutable record binds the sequence, digest of the complete application submission, signed capability and sealed envelope. A new transmission charges a durable attempt. A missing original with a surviving attempt record refuses recovery. The same custody directory bounds new source records at 64, retries at eight per record, and record size at 64 KiB. No reclamation or sustained capacity beyond that limit was exercised here.

The host send API accepts the original destination helper handle. Before writing it matches the opposite boot, mailbox address, policy and sender against the signed role, then authenticates the socket against the live child process. The host rechecks authority after connecting and again before returning a receipt. A Reply exchange ends at the earliest of its original Call lease expiry, envelope expiry, caller deadline and the 12-second I/O limit. The test uses real canonical grants, disk, socket and journal. The installed listener still needs to dispatch the guest submission through both original owners and the receiver’s live object endpoint. [Installed integration task](https://github.com/R4-Laboratories/SpiralCore/issues/202)

## Reproduction

Use the linked MVM fork revision, Rust 1.97.1 and its lockfile. Linux passed three source-store tests, three selected host-pair tests, one legacy retry test and 61 core mailbox tests. The missing-original regression failed before the fix. It now refuses to mint new ciphertext. Review also checked shared capacity, scoped ownership, receipt substitution, deadlines and socket authentication. Two existing host-test warnings concern an unused VM name and an unused fixture variant.

```sh
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cargo test --locked -p mvm-hostd --lib object_intent_slot_tests
cargo test --locked -p mvm-hostd --lib mailbox_launch::object_pair:: -- --nocapture
cargo test --locked -p mvm-hostd --lib a_retry_reuses_exact_sealed_intent_after_helper_restart
cargo test --locked -p mvm-core --lib mailbox::
```

Each measurement launched the selected host test executable with `--exact mailbox_launch::object_pair::tests::original_seals_exact_object_bodies_and_restores_only_the_matching_retained_lease --nocapture`. The parent recorded monotonic wall time and the child’s user/system CPU and maximum resident set through `wait4`. All 4,512 tracked blobs were checked against the test host before measurement. The rows retain the binary digest and individual times.
