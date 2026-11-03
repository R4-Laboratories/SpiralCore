# Installed owner core: focused final CODE/RUST delta

**PASS — no blocking finding in this component delta.** Reviewed
`d4f2ed412ae961c5d2bd267caf0ec1a2628157c1` against original core `99882ffa`;
production installed_owner.rs SHA-256
`b75172d56b284adf9b69417fac3d6835fd9cb2edf311b871cb518222bda1ae57`.
The earlier core-code-rust.md BLOCK remains historical and is resolved below.
No product edits, builds, tests or broader audit were performed by this reviewer.

- **Exact descriptor/ABA:** both SH and EX use locked_holders. The actual newly
  opened descriptor must match the pinned holders object before locking, and
  the namespace is checked again after locking. A wrong B descriptor cannot
  pass just because the pathname returns to A. Owner transition errors fence;
  genuine contention remains Busy. Original lock/drop ownership is preserved.
- **Durability:** the bounded exclusive intent is written and file/directory
  synced before any state mutation. It remains through next-file sync, rename
  and the state-directory barrier. The earlier post-rename uncertainty now
  leaves a durable marker that fences reopen. Marker removal occurs only after
  the confirmed state barrier; a later cleanup-sync error fences the current
  owner, while an absent marker can safely allow a successor. The amended
  contract explicitly distinguishes that post-commit case and does not claim
  unconfirmed initial intent creation survives power loss. No silent cleanup or
  epoch reset was added. Initialization requires a private parent and syncs its
  new directory entry before success.
- **Filesystem/privacy:** descriptor fstatfs admission rejects unknown/network
  profiles and unsupported platforms. The Linux/macOS allowlist is admission,
  not executed qualification of every named filesystem. macOS descriptor ACL
  inspection rejects any extended entry, releases obtained ACL storage once,
  and propagates other failures. NULL with ENOENT/ENOATTR is narrowly accepted
  after live-descriptor metadata validation; it does not turn general ACL errors
  into success. Apple primary [acl_file.c](https://raw.githubusercontent.com/apple-oss-distributions/Libc/main/posix1e/acl_file.c)
  routes descriptor ACL retrieval through fstatx and FILESEC_ACL;
  [filesec.c](https://raw.githubusercontent.com/apple-oss-distributions/Libc/main/gen/filesec.c)
  returns ENOENT for an absent ACL property. Apple's
  [acl_entry.c](https://raw.githubusercontent.com/apple-oss-distributions/Libc/main/posix1e/acl_entry.c)
  confirms zero on an entry and -1/EINVAL at the end of an empty valid ACL.

Read retained deterministic trust-boundary REDs: wrong-FD SH and EX acceptance
and post-rename unconfirmed Clean readmission both failed before their fixes.
GREEN covers those cases plus failed initial intent barriers and post-commit
cleanup boundaries. Test hooks are cfg(test), thread-local and scoped; they
force the actual open/restore or barrier boundary without release controls.
The ACL RED retains all five real privacy fixtures; the original no-ACL ENOENT
failure and the corrected focused run remain distinct.

The current core-final-macos.txt/exit record is **14 passed, zero failed/ignored/
filtered**: four unit boundary tests and ten lifecycle entries (nine behavioral
tests plus one subprocess helper), with zero doc tests explicit. This is a debug
focused harness of the exact production file; prior release records remain
separate. Unit source SHA-256 `4d9b5151bdf013e2c27e553dd74d69599fa058b3d141017a64a2e722382f66bf`;
lifecycle source `c876dba62727bab03b0b2d4ae38c385e2a25954e9cb41787f9b14876096179fd`;
current contract `88a09bf2caf820a4cf274a4e1a03eaea900d6dcc2fab78096d61a1d6d6199b1d`.

This resolves core-source blockers only. Injected filesystem failures do not
establish physical power-loss qualification; Linux filesystem/native execution,
main/default-path and budget integration, actual worker and privileged VMM lease
custody, complete release/Clippy/coverage/CI and full B6a/B6b remain separate gates.
No native VM, WAN, performance, energy or completed-delivery claim follows.
