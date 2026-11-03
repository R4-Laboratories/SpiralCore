# Installed-owner core security review

Disposition: **BLOCK at 99882ffa**, pending the two bounded corrections below.
Independent source review only; no product/test edits, builds or execution.
Reviewed the active bootstrap security-reviewer instructions, the approved API
and plan, and the exact core component. This is not native resource integration,
full B6a, host-scope accounting, B6b or automatic VMM termination qualification.

Pins (SHA-256):

- `src/installed_owner.rs`: `41796b960e4f05309ecc5b933651445d9bba5888fed79e5d35309c9bd5343252`
- `docs/api/installed-host-owner-v1.md`: `c0a8a1112d653a70023e35ed5b75d0a016c9af96336e66158988f086f16248ba`
- `docs/plans/2026-11-03-installed-host-owner.md`: `e2fd19546a3c77babc0c46a6ec1384766bae8c533989d81cacb8a94cf0dc5970`

## Blocking findings

1. **HIGH — locked holder descriptor is not bound to the pinned inode.**
   `Storage::exclusive_holders` (354–358) and `HolderLease::from_storage`
   (371–381) call `check()`, then separately open/lock `holders.lock` without
   checking the actual locked File's ObjectId against `ids[2]`. An ABA schedule
   can expose original A during checks, replacement B during the actual open,
   then A again during `read`/`publish` checks. An SH guard then protects B, or
   retirement takes EX on B and publishes Clean while live SH holders retain A.
   No record forgery is necessary. Static replacement tests do not cover this
   schedule. Compare the exact opened descriptor's identity with the pinned
   holder identity; refuse/fence mismatch. Add a deterministic ABA regression
   proving no guard/retirement succeeds against an unrelated lock object.
2. **MEDIUM — post-rename sync failure loses the persistent uncertainty fence.**
   `publish` renames `.state.next` to `state.bin`, then calls directory sync
   (340–351). If that sync fails, only the current owner's AtomicBool fences;
   the temporary name is gone. Drop/open can accept visible Clean and begin,
   contrary to the pinned "any uncertain publication fences admission" rule.
   Existing holders EX still prevents a demonstrated overlapping-holder escape;
   this is a concrete durability/contract failure. Preserve a publication-intent
   marker through confirmed rename+directory sync, or separately review a
   narrower contract with explicit successor revalidation. Test the exact
   rename-success/directory-sync-error/drop/reopen window, not only leftover temp.

## Preserved boundaries and qualification limits

The component otherwise retains distinct owner EX and holder SH, checks current
Active generation after SH acquisition, uses bounded canonical state with
checksum/key/object binding, checks increment overflow, never cleans in Drop,
and performs no PID signaling, guest activation or mailbox authority operation.
Durable Active plus live-holder Busy is preserved by the reviewed paths.

`lifecycle-second.txt` and exit receipt report 9 PASS (8 behavior tests plus child
entry), 0 failure; these were read, not rerun. Neither blocking fault is in that
matrix. Linux/macOS native worker/VMM custody and full main-crate integration
remain unexecuted here; component subprocesses establish neither claim.

Before installed native qualification, make these concrete storage assumptions
explicit or enforce them: `check_private` checks UID/POSIX mode/type/link count,
not macOS extended ACL access; `open_root` refuses a final symlink but does not
walk/reject all ancestor symlinks. No supported-local-filesystem check exists.
`initialize` syncs installation contents but not the newly created directory's
entry in its parent: do not claim tested power-loss bootstrap durability from
SIGKILL tests. These are bounded provisioning/durability gates, not claims of
secret isolation from trusted same-UID/root operators or a reason to expand
this review into unrelated platform/security work.
