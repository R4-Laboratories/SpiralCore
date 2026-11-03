# Final registered-Call retry CODE/RUST/INFRA confirmation

2026-11-03. **CODE/RUST/INFRA PASS for the frozen source and local qualification.**
This is the bounded quality-gate Step 2 confirmation after the task-specific
[Step 1 PROCEDURAL PASS](../coverage/procedural-gate.md). It reuses the approved
[production review](source-rust.md) and [frozen test/manifest review](tests-infra.md);
it introduces no new source audit. Hosted merge qualification is initially
pending below; local source approval alone does not approve protected merge.
Independent security approval remains separate.

Source PR #64 head is `d14271ddec5424326603572693c15aeb1a2f2e48`. Its entire
crate diff against locally qualified `4d34c6a722ec5223bf3e25431ab0b73e11f39d54`
is empty. Fresh hashes retain the approved exact identities:

```text
5563e59811266047b582651aa6e08682ff3c368d417c2aabdbee8930a1217794  crates/spiralcore-mvm-adapter/src/service.rs
a81c0ff09047261e0430af853ad030a10fa67f0a574a143cbaade9d393d991fb  crates/spiralcore-mvm-adapter/tests/call_send.rs
11c07ee150221854383254f13730e771fa1b013b3eb0d0ed42335ca5bc466cac  docs/platforms/mvm-mailbox-portable.json
```

The strict Pending-only path, additive unconsumed-Reply eligibility, original
identity/generation/semantic commitment and dual lifetime validation, immutable
Reply charge and existing policy-guarded canonical transport path retain their
source approvals. Six test functions, 27 counted pre-connect refusal cases and
five transport fault cases retain their test-source approval. The portable
manifest changes only call_send minimum 2 to 6; no rows, dependency, resources,
permissions or workflow policy changes were introduced. Untracked draft consumer
contracts/plans are outside PR #64 and this review.

The retained expanded target now confirms six passing tests with all other result
counts zero. The normal locked affected-adapter release log confirms 127 passing
tests across 22 all-target scopes, zero failed/ignored/measured/filtered; its exit
receipt and strict Clippy exit are zero. These are execution receipts subsequent
to the earlier source reviews, not reviewer reruns or additional semantic REDs.

Coverage receipts confirm 125 passed across 19 separately retained instrumented
library/integration binaries, each protected before execution and separately
exported; all test/export stderr is empty and compiler warning count is zero.
The normal all-target gate's two extra example tests explain 127 versus 125.
Changed mapped executable production lines, including moved validator lines,
are 31/31 (100%); the broader mapped service.rs scope is 2,010/2,169 (92.67%).
All 159 broader unhit lines remain listed, with no test exclusions; branch mapping
is NA, not a branch-coverage claim. Matching before/after source receipts contain
285 entries each. These are scoped line percentages, not whole-project or
semantic-path completeness.

No normal engine invokes this additive API. No performance campaign or Idle,
timer, retry-consumer, reconnect/resume, completed-receiver, native-backend, WAN,
energy or capacity claim follows. Full C8 and wider roadmap acceptance stay open.
Only this review and the owned hosted qualification packet are written by this
task; no builds/tests/performance, product/test edits, commits or pushes occur.

## Hosted qualification status

At initial confirmation, Linux run 41649886275 was in progress and portable run
41649886300 had its matrix job complete with seven native rows queued/running.
Both are for the exact PR head above. **Protected merge remains conditional on
successful exact-source CI and independent artifact/source/count custody.**
A subsequent resolution will retain the original ZIPs and all inner checksums,
actual native identities/counts, tested merge tree and Linux scope observations
in `../qualification/` rather than infer them from a green badge.

## Hosted resolution: final source approval

Both exact-head hosted runs above completed successfully, independently verified
from retained original metadata/logs and artifacts. **The CI/custody condition is
resolved: final CODE/RUST/INFRA PASS for source PR #64.** Independent security and
protected merge remain separate; no full C8/goal completion is asserted.

[Hosted qualification verification](../qualification/portable/verification.md)
retains seven original ZIPs (193,853 bytes), all seven API digests, all 236 inner
checksums and actual native OS/architecture/Rust-host observations. Every row
passed 92 tests, including call_send 6/TCP11/receipt21: 644 portable executions
plus eight separate Unix storage executions. Windows storage is absent. Original
ZIPs remain; only duplicate extracted copies are removed after verification.

Every row and the Linux checkout log identify tested merge
1746e57c1adaf57a32269ee2d7bde213d4fed5ce, parents a8a4a9a8 and d14271dd. Its entire
tree e5c32fa15b132ca1ad28cfdc5914787079b18df2 equals the reviewed head tree.
Canonical dependencies retain ae12e9deb41cc9a3264196ce22ebc72d4042883d.
The original Linux log confirms 92 live release and 135 shared executions
(8 object-v1 + 127 adapter), all zero failure/ignored/measured/filtered counts.
Four zero-test scopes and stage/platform overlap are explicit. Strict Clippy,
default guest compile and both synthetic analyzer negatives pass. Four additional
call_send functions increase shared adapter only; the live release count is
unchanged. Windows CRLF source projections match the same tested Git blobs.

No runtime, API, product/test/manifest or draft-consumer content changed during
this confirmation. The approval is for the exact reviewed/tested tree, not an
unobserved future merge. No new builds/tests/measurements/CI, commits, pushes or
broader audit were performed by this reviewer.
