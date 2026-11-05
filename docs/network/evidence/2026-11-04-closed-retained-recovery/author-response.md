# P1 acquisition custody — author response and executed checkpoint

2026-11-04. This is an AUTHOR response, not independent acceptance. Original independent HOLD `recovery-independent-review.md` SHA2115b84b73b7375ae4652b973f7e1ad6002fc8e885932a73ac93173f95946f09 remains preserved. Fresh independent Code/Rust/Security review is required for this successor.

Five frozen source inputs match the actual final 135-input compiled closure. Archive `source.tar.gz` SHA dc42704cdd6d2a5a799a87cc05ea226bcd648a42182fedb1f89ed7083c7ada7f. New recovery leaf SHA c924391854633195876811116f634742b1e9a1916060200a4ff4e03a38fa968a. The four other input hashes are unchanged from the prior author checkpoint. No manifest/lock/package/version changes, local Cargo, native guests or Git publication.

## Causal RED and implemented delta

`validation/recovery-acquisition-red` built successfully then ran five selected parent tests, all failing at the intended custody assertions: catalog observation lost on CRC refusal; holders EX released after explicit late-current fault; marker observation lost on cap refusal; state observation lost on post-open privacy refusal; original owner EX released on explicit post-holder unwind. Each test first generates a fresh private fixture with the SAME protected old product executable SHA45ddf9d61f3bdb853f554788645aa3a176aac9294712f7da482755d33e996659. These are ordinary std Child process fixtures, not native VMs. Late-current/unwind cases are explicitly cfg(test) fault injection, not natural-exploit evidence. The historical old fixture was not modified.

The bounded fix is confined to `installed_owner/retained_recovery.rs`:

- A Preparation owner is armed from function entry and owns the original owner, actual holders observation and subsequent closed funding across returned errors AND unwind. Successfully acquired holders EX is never held only in a fallible closure-local temporary. Failed preparation Drop retains actual custody, not copied identities.
- Closed-recovery-specific raw open helpers use nofollow/CLOEXEC/nonblock, immediately assign a successful fresh FD into an EMPTY private slot, then perform existing private ownership/ACL/filesystem/identity/cap/protection checks. No incoming raw FD is adopted and ordinary public owner/open APIs are unchanged.
- The catalog has a distinct unvalidated observation slot before read/decode/nonempty/final epoch checks. It becomes the accepted catalog only after those checks succeed.
- Every row has bounded private observation slots for state/marker/records/key/status. A post-open state failure retains that observed FD; optional-file cap/private failures retain their actual offending FD. A qualified state is separately armed in the accepted row before later checks. Unvalidated observations cannot produce ClosedRetainedDebt, Snapshot, Sealed, a scope, a grant or admission.
- Revalidation fills separate bounded fresh observations and compares them with accepted originals; it never overwrites the originals before comparison. Only complete successful comparison drops duplicate observations. Errors retain both sets; failed adoption remains blocked and whole-custody Drop remains quarantined.

The fixed source-qualified ceilings and ordinary reserve gate are unchanged. Validated phase2 still imports FULL R+K; malformed catalog has no trusted decoded row count, retains closed exclusion and fixed bookkeeping rather than interpreting corruption as empty/zero debt. There is no refund, reinitialization, stage promotion, new quota, reopen or deletion path.

## Exact executed successor scope

Raw prefix: `/var/tmp/spiralcore-retained-catalog-20261104/validation/`.

| Phase | Actual observation |
|---|---|
| recovery-acquisition-red | Release build0, selected tests0PASS/5FAIL, test101; exact five custody failures retained with original source/object maps. |
| recovery-acquisition-green | Release build0; acquisition tests5PASS/0FAIL (0.16s); previous malformed-marker1PASS (0.03s); actual old-profile phase2 positive1PASS against NEW separately generated old fixture. |
| Same protected successor, ordinary library | 153PASS/9ignored/0failed, 12.46s. The seven explicitly executed ignored cases above remain ignored in this ordinary invocation; two other ignored cases are unrun here. No VM/count inflation. |
| Scoped release Clippy | `--lib --tests -- -D clippy::all` exit0; no lint masks. JSON has38 ordinary unused-import/dead-code diagnostic messages because production recovery constructors remain unconnected, including the two private error fields. This is NOT zero-warning or `-D warnings`/all-target acceptance. Release test build JSON has2 ordinary unused-error-field warnings. Both JSON and stderr are retained. |

Final protected binary: `recovery-acquisition-green/protected-test`, SHA d22faf6febf65a30564f8617d1bdf5f80a7f4effd614f75517065b779b1acc12. All135 source inputs match before/after selected tests, protected broad execution and Clippy. Protected binary before/after hashes match. Historical fixture and fresh positive fixture state/catalog/beacon/marker maps each match before/after. Tests inspect actual File dev/ino via this-process Linux `/proc/self/fd` observations and actual independent kernel flock contention before/after failed-object Drop; no PID attach/reap authority or public FD getter is introduced.

P2 bounded diagnostic accessors/observed-length snapshots/same-controller retry remain explicitly unfinished. The complete phase1/2/3/4, table/tenant/cap/overflow, replacement and failure matrix is not established by these seven selected tests. Native producer/EndpointRetirement qualification, Stage B generation/profile migration, useful customer restart, retention disposition, final capacity/platform/WAN and compliance acceptance remain open. Source is frozen for independent successor review, and the heavy lane was returned at authoritative batch terminal0.
