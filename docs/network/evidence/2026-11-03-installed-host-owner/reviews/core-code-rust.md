# Installed owner core: CODE/RUST review

**BLOCK — one durability-contract gap; component source only.** Reviewed commit
`99882ffab1c8fd7228d93223b5efb9e10eff1145`, exact production module SHA-256
`41796b960e4f05309ecc5b933651445d9bba5888fed79e5d35309c9bd5343252`,
against the installed-owner API contract and plan. No builds, tests or product
edits were performed. This is not full B6a/native resource-lifetime approval.

**[P2] Uncertain publication can lose its admission fence on reopen**
(`installed_owner.rs:326–352`, `460–464`). If renameat succeeds and the following
directory sync fails during finish/reconcile, state.bin visibly contains Clean
and .state.next no longer exists. commit fences only the existing owner's
AtomicBool. Dropping it and calling open reads that Clean record without a
durability-recovery step; begin can issue the next epoch. This does not preserve
the explicit contract that any uncertain publication fences admission. The
existing leftover-temp test does not exercise this post-rename failure. Resolve
the durable/reopen admission rule and retain a focused injected post-rename sync
failure proof before treating this contract as implemented. This finding does
not assert that the preceding holders-EX check admitted a live holder.

The remaining inspected boundary is coherent: independent non-Clone owner EX
and holder SH descriptions, no explicit unlock, owner descriptor dropped after
the supervisor SH, no Clean publication in Drop, checked generation increment,
Busy retaining Active and retryable finish, fresh Active/key/generation checks
while SH-held, and EX exclusion before retirement. The begin EX-to-independent-SH
gap is protected by owner EX plus durable Active; a delayed holder must validate
the current record under its own SH. No PID or timeout substitutes for a lease.

Descriptor-relative children, final O_NOFOLLOW, regular/private/single-link
checks, held directory/lock identity comparison, sticky owner fencing on current
record mismatch, fixed 176-byte canonical parsing and bounded 177-byte reads
preserve the declared storage/input boundary. Publication uses exclusive private
temporary creation, file sync, rename and directory sync; it is specifically the
last failure/reopen case above that remains unresolved. Trusted same-UID/root
record forgery is outside the stated checksum threat model.

Read actual lifecycle-second.txt: eight behavioral tests plus one subprocess
entry helper pass (nine reported), with zero-test lib/doc scopes explicit.
The real-process owner SIGKILL/live-independent-holder/Busy/holder-exit/Retired/
generation-two proof is useful component evidence. Initial typed-refusal failures
remain retained. Neither these runs nor this review prove actual worker/VMM
custody, main-crate integration, full release/Clippy, native boot or installed
aggregate-budget closure; those planned gates remain open.

Contract SHA-256: `c0a8a1112d653a70023e35ed5b75d0a016c9af96336e66158988f086f16248ba`.
Plan SHA-256: `e2fd19546a3c77babc0c46a6ec1384766bae8c533989d81cacb8a94cf0dc5970`.
