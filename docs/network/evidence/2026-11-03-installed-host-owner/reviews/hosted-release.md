# PR #68 hosted release addendum

**HOLD — verifier/source-tree custody discrepancy; not a product test failure.**
The prior procedural/INFRA hosted holds are not yet cleared by the retained
verification PASS line. No test/build/workflow was executed by this review.

Actual final metadata supports success for owner41697837824 (all four jobs)
and Linux41697837631, both attached to reviewed head
64d93fed72fdac6784d99e6e1a258052bba605f6. Reading native uname/Rust1.97.1 hosts
and exact summaries gives Linux x86_64/ARM64 4+9 each, macOS Intel/ARM64 4+10
each: **54 entries including four subprocess helpers**, zero failed/ignored/
filtered. Read strict focused Clippy outcomes. Independently rehashed all four
original ZIPs against API artifact digests; all match. Each archive's source
manifest agrees with its extracted copy and expected current five-file manifest:
**20 compiled-file pins match**. These positive observations remain valid.

However, every original archive records actual checkout
`dd8362d0f7265849653f58c53c6d5bfd185f7a46`, a candidate tested merge, in
source-commit.txt. The retained verify-owner.sh instead assigns expected64d and
tests every checkout against that head, yet owner-verification.txt ends PASS.
Those exact script assertions cannot substantiate the retained PASS against the
original archive contents. Root confirmed a scoped unmodified rerun also continued
past the false assertions and is retaining script/receipt as **UNACCEPTED** for
diagnosis. Preserve this failure; do not replace archive bytes or reinterpret
head as actual checkout. A PR merge checkout is normal, but must be authenticated.

Before clearing this hold: correct the metadata verifier so mismatches terminate
explicitly; authenticate actual tested merge parents/tree against reviewed head
and base, compare the resulting source tree/pins, and retain its actual successful
output. This requires metadata verification, not another product test campaign.
The tested merge object was unavailable in this reviewer's current local object
database, so no equality of whole trees is claimed here.

Independently recounted Linux log: **115 live checks**, eleven summaries with two
zero scopes; **138 shared checks** (130 adapter+8 object-v1),23 summaries with two
zero scopes. Scopes overlap. The successful run retains strict main Clippy,
default Linux guest-entry check and both synthetic analyzer PASS results.
Original pre-job workflow rejection and original component failures remain
historical evidence. Once authentic checkout/tree/custody verification resolves
this specific discrepancy, reuse existing source/review/local gates; do not
broaden qualification. No source merge, full B6a, native VMM recovery, Windows,
WAN or performance result is approved by this addendum.

## Corrected custody resolution

**PASS — the scoped hosted/procedural/INFRA holds above are resolved.**
Read corrected verify-owner.sh (SHA-256
b399d5afd1ecf3f27fe0b2318f394e8113038162b93214ea1ab621450af51657)
and owner-verification-corrected.txt (SHA-256
71a71bae1383200e1b9540d9dff028894551de5f4bce50092ab2533003b65710),
exit0. Explicit failure branches replace reliance on ambient errexit behavior.
The original script and false-PASS runs remain UNACCEPTED historical evidence.
The corrected run checks the actual four tested-merge commits, four original
archive/API digests,20 current compiled-source pins and54 entries. Retained
wrong-commit and wrong-digest controls each exit1 with the expected specific
failure, not a later PASS.

Retained GitHub commit API records identify tested merge
dd8362d0f7265849653f58c53c6d5bfd185f7a46 with ordered parents
7c220cc841fa441df176a58cb24aeecd4d4d5fad and
64d93fed72fdac6784d99e6e1a258052bba605f6. Its full tree is
3d478a93c3366997913bc0d44f2c5c8e3440d98b, equal to the reviewed-head API
tree and independently queried local reviewed-head Git tree. The archives'
actual checkout is therefore authenticated rather than relabelled as the head.
Prior independent digest/pin/count/platform checks and all final successful
hosted metadata above stand. No product/test rerun was needed for this metadata
correction, and none was performed by this reviewer.

This closes the recorded custody/hosted holds for the qualified PR68 component
source; reuse the preceding source/local/coverage/INFRA approvals. Root retains
final release JSON, explicit selection and protected-merge execution. No actual
merge is claimed here, and full B6a/B6b/native VMM handoff/WAN remains open.
