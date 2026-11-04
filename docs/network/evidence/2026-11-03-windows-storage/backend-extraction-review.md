# Windows storage backend extraction — scoped source review

Disposition: **PASS for this source delta; native corrected-source qualification
remains a release gate.** No build, test, VM or artifact requalification performed
by this review. Fresh roster verify/check: d1927d9e7ce51354, security-reviewer active.

The pre-extraction file read from HEAD hashes to the previously reviewed
`db97d858103a0c4ca0cf4272691de9f730ff4e0c2d19113332cee54a8726a77a`.
Its diff against the new backend is exactly eight visibility substitutions from
`pub(in super::super)` to `pub`; all implementation bodies are byte-identical.
Thus original-input path refusal, held ancestor handles, actual-object checks,
protected SID/DACL validation, exclusive creation, write-through same-parent
publication/identity comparison, flush and exact-handle deletion are preserved.

The new crate exports those eight safe functions and its three-variant Error.
Raw-pointer helpers/FFI ownership types remain private; no unsafe public API,
raw HANDLE export, callback, broad lint allow or fallback was added. Returned
File values retain ordinary Rust ownership. The new crate denies unsafe
operations outside explicit unsafe blocks; transfer's forbid(unsafe_code) is
unchanged. The target-specific dependency preserves the same pinned windows-sys
version/features. Exhaustive wrapper mappings preserve Storage, Unsupported and
Limit, including I/O-to-Storage behavior. Endpoint publication-before-ACK and
the Unix backend have no source delta.

The two unsafe ACL/junction tests moved to the backend and still assert actual
refusal/no permission repair/no target mutation; setup failure is not skipped.
Outer path-alias tests remain at the Endpoint boundary. Atomic fixture serials
are test-only and resolve equal-clock name collisions without weaker assertions.
The contract and boundary/native-CI notes honestly require separate nonzero
Windows outer4/backend2 and Unix outer3 scopes; MSVC compiler checks do not
establish native execution. Retain the original unsafe-lint RED/artifacts. No
native Windows MVM, WAN or power-loss result follows from this extraction.

Reviewed SHA-256 pins:

```text
2606e53019a1768ec9bc2db361d438e968e993145491a98e5f955e1548f6609c  backend/src/windows.rs
0e09ed65b0cee67dced36645c9c696321f198f656b7a25da2d514e3470856558  backend/src/lib.rs
0ce997ff140d1bff85af072e45b51a68348bfec647b4363b28275f8ea1f2fdd1  backend/src/tests.rs
237a1d7c83dd80490d07ef2c3f31329e0aec4da273130949052ae232aa02e481  backend/Cargo.toml
8830933a25e8a445134c2c75cdd299f25ee32385fa8df0fcf522e70def77a4b0  transfer/storage/windows.rs
622d14b9403987e651c62842c61bac3730bc41c3d62945810d6efb8408b945a1  transfer/storage/tests.rs
e71e16a4c197db94979e6ddd8f72248727cb62f3e66d7ff3a5709d9c50919f05  windows-storage-v1.md
4e410dcd017fceea31718cebf4459941ebc180e3c5109355f455c072a51ceb36  backend-boundary.md
af2444dc67273d18b6a53dd0056a8dbc3cb16553c3e03484310343e4a0c36d39  native-ci.md
```
