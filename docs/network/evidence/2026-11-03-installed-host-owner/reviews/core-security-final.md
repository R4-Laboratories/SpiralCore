# Installed-owner core: final security delta

**PASS for this component source; no remaining blocking finding in the reviewed
delta.** Supersedes the two findings in `core-security.md` for the exact source
below; preserve that original BLOCK and its failed regression receipts.
No product/test edits, builds, tests or remote operations by this reviewer.

Production source at `d4f2ed412ae961c5d2bd267caf0ec1a2628157c1`:
`src/installed_owner.rs` SHA-256
`b75172d56b284adf9b69417fac3d6835fd9cb2edf311b871cb518222bda1ae57`.
Unit tests: `4d9b5151bdf013e2c27e553dd74d69599fa058b3d141017a64a2e722382f66bf`;
lifecycle tests: `c876dba62727bab03b0b2d4ae38c385e2a25954e9cb41787f9b14876096179fd`.
Current installed-owner API: `1a92ede6f85701288ead3a2bcc23d6dea0cb158a40806af55d5819198b527d71`.

- **HIGH ABA resolved:** SH and EX share `locked_holders`; the exact opened
  descriptor must match the pinned holder ID before locking, followed by fresh
  namespace validation. An unrelated B descriptor cannot pass restored A
  pathname checks. Owner transitions fence on identity/storage errors; actual
  contention remains Busy without publishing Clean. Deterministic unit cases
  cover SH acquisition, finish EX and reconciliation EX with original A live.
- **MEDIUM publication fence resolved:** intent file and directory barriers
  precede all state mutation. The marker survives temp write/sync, rename and
  state-directory sync, so an unconfirmed visible Clean cannot regain admission
  after owner drop. Intent is removed only after that state barrier. Post-commit
  cleanup failure errors/fences the current owner; an absent marker can safely
  permit a successor under the explicitly amended contract. Initial intent
  barrier failures leave original state untouched. No automatic repair/reset.
- **Privacy/FFI confinement:** initialization validates a private existing
  parent and syncs the new directory entry. Descriptor filesystem admission is
  bounded and rejects unsupported profiles. macOS ACL FFI uses live validated
  descriptors, releases each returned ACL once, rejects any entry, accepts only
  the documented absent-property and empty-valid-ACL results, and propagates
  other errors. ENOENT here is not a general pathname-success rule. Apple
  primary-source corroboration/probe is reused from `core-code-rust-final.md`;
  no new platform probe was performed. Final-only NOFOLLOW and trusted fixed
  ancestors remain explicit; no hostile same-UID/root isolation is claimed.
- **Lifecycle retained:** fixed state length/checksum/key/object validation,
  checked generation, current Active check after SH, owner EX through changes,
  no explicit unlock and owner authority dropped last. Drop never publishes
  Clean. The component signals no PID and creates no guest/VM authority.
  Fault hooks and controls are cfg(test), thread-local and scoped.

Retained records read, not rerun: macOS focused debug harness 14 PASS (4 unit +
10 lifecycle entries); Linux 4 unit PASS plus explicit serial lifecycle 9 PASS
(13 total, including one subprocess-helper entry). The initial Linux parallel
Busy failure and controlled pre-exec inheritance diagnostic remain retained;
their subsequent successes do not erase them. The original incident's precise
attribution remains inference, as `linux-first-diagnosis.md` states.

Linux mapped production coverage is 402/428 (93.93%), 26 lines uncovered,
branch NA; macOS ACL-only lines are outside that Linux percentage. Main release
receipts total 115 PASS and strict Linux Clippy exit0. Injected EIO boundaries
are not physical power-loss qualification; filesystem admission is not executed
qualification of every allowlisted filesystem. Hosted exact-head gates and
final publication/merge disposition remain independently required.

Approval covers the installed-owner core only. Worker/default-path integration,
actual privileged Firecracker lease retention, macOS HVF lifetime coverage,
owner/worker death with live VMM and positive native readmission remain open.
This is not full B6a, B6b, scope=host, plaintext/tail aggregate accounting,
increased session retention, WAN or native Windows MVM qualification. The linked
Firecracker lease contract is a next-step proposal, not an executed consumer.
