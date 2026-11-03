# Frozen Windows storage candidate: independent code/security review

Disposition: **BLOCK** on two concrete path-validation/ownership cases below.
Source-only review; no build, runtime test, VM or product edit performed. Fresh
roster verify/check confirmed active security-reviewer, logical roster
`d1927d9e7ce51354`; active role instructions read. This reviews the shared endpoint
delta and exact native filesystem source, not a broader architecture audit.

1. **P1: relative roots can escape the retained owner lock after a CWD change.**
   `endpoint.rs:250-252` retains `root.to_path_buf()`, while
   `endpoint/storage/windows.rs:144-145` independently resolves each operation
   using `std::path::absolute`. Ordinary relative, drive-relative and root-relative
   roots are currently accepted. Opening `store` under A, changing process CWD to
   B, then publishing resolves B/store while `_owner` still locks A/store/.owner;
   a separate endpoint can own B/store concurrently. This requires no ACL forgery.
   Minimal correction: enforce the contracted original drive-absolute prefix plus
   RootDir before normalization, or normalize once and retain that absolute root
   for every endpoint operation. If relative roots remain supported, require an
   isolated real-process CWD-switch/second-owner regression. If refused, add native
   pre-I/O refusal cases for relative, `C:relative` and `\root-relative` roots and
   prove no marker/owner/stage was created.

2. **P2: traversal rejection happens after Windows has erased the evidence.**
   `endpoint/storage/windows.rs:144-176` invokes absolute before its component
   check. Windows absolute uses GetFullPathNameW for ordinary paths and collapses
   `..`; e.g. a missing intermediate `root/missing/../child` becomes root/child
   and can create a store, despite the contract's explicit traversal refusal.
   Preflight the original path's syntax/components and ambiguous trailing
   dot/space spellings before Win32 normalization; retain final normalized checks.
   Native regression: parent-traversal and trailing-dot/space aliases refuse
   without creating/changing the resolved target. Include ordinary and verbatim
   drive forms. This finding is an actual accepted-input contract violation, not
   a claim that a local caller acquires arbitrary signed mailbox authority.

Primary corroboration: [Rust absolute](https://doc.rust-lang.org/std/path/fn.absolute.html)
documents the Windows GetFullPathNameW mapping and a collapsed parent example.
[GetFullPathNameW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfullpathnamew)
warns that process CWD is global and repeated relative resolution can disagree.
The pinned local Rust1.97.1 path documentation corroborates that mapping.

Other reviewed boundaries: protected current-token SID owner/DACL, handle-based
file type/reparse/link checks, retained ancestor handles during namespace moves,
exact DELETE-handle stage cleanup, exclusive stages and no failed-create cleanup,
file flush before same-directory write-through move, full final byte readback
before ACK and fresh authorization. No additional concrete source blocker found
there. [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
confirms delete sharing controls rename; [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)
documents the selected write-through behavior. Physical power-loss guarantees
remain outside this review. Unix keeps its existing directory-sync publication
path, adds file/hardlink validation, and reads metadata plus payload through one
opened file. No unrelated Unix filesystem redesign is requested.

Recorded evidence inspected: selected Linux storage3 PASS, existing endpoint21
PASS, Linux strict Clippy exit0 and exact Windows-backend compiler adapter exit0.
Those are prior records, not new executions here. Full MSVC graph cross-check
failed for missing lib.exe; Windows ACL/junction/native publication runtime is
NOT_RUN. Native positive-count CI and exact final source qualification remain
required even after these source findings are resolved. Native Windows object
storage does not establish native Windows MVM, WAN or power-loss support.

Reviewed candidate SHA-256 (relative to crates/spiralcore-transfer):

| File | SHA-256 |
|---|---|
| src/endpoint.rs | `f1237a96b5a69ff1f378c548a971951bd4d47dfef4bff2fef04d2353bc5568b2` |
| src/endpoint/storage.rs | `47b5f7a559db912321527af0d14add0ad4cc2be3020eaa2ef83cb8b368d5c0e6` |
| src/endpoint/storage/windows.rs | `30ed1a4bc550ba012c3ee868d2b39f336b45834550b0ef8131b4bd765818bbe4` |
| src/endpoint/storage/unix.rs | `c2dd971d9e53fe412756c3f94aa1081cdfa0a451520b76fcc3820e86f2ec45fa` |
| src/endpoint/storage/tests.rs | `f02d2bd8957f272258a400b17356ed9fc11859b15263f0fe87846bd1d6726cc1` |
| Cargo.toml | `f917e0d43f224d95bd1aa03f647c3c137aacc14e50b3e4008dd98ef31c96ec7c` |
| Cargo.lock | `259155c642730ba37ff8a6806d7fd0ef10c1db5a2f66e7730910c40538a48697` |

Executed production/manifest hashes match the frozen candidate. The test hash
differs from the executed receipt only by recorded import formatting. Contract
SHA-256: `76f6572fbfd33038efb28e957411fab3405d69cb012439a632bd55ca8e96f551`.
