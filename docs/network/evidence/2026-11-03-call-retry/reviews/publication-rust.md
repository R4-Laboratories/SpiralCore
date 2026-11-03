# Results publication: final CODE/RUST/INFRA confirmation

2026-11-03. **PASS — no blocking publication-consistency, CODE/RUST or scoped
INFRA finding.** This follows the results-only
[Step 1 PROCEDURAL PASS](publication-procedural.md) and reuses
[final source/qualification approval](post-audit-rust.md) and
[hosted custody](../qualification/portable/verification.md). The proposed results
publication is approved from these review scopes; independent security approval
and actual results publication remain separate. No artifact re-verification,
tests, builds, performance campaign or expanded audit was performed.

Reviewed the final report and packet README, plus README/parent-plan publication
edits and their context. Local report/contract/plan/evidence links point to the
intended existing records; hosted links retain the exact successful source runs.
The actual source merge `5fb35d928a10a9cb924c78f1d574199b8f8772c5` has parents
`a8a4a9a8daba67c00447a067b18632ed515d95b2` and
`d14271ddec5424326603572693c15aeb1a2f2e48`, and tree
`e5c32fa15b132ca1ad28cfdc5914787079b18df2`, equal to the previously verified
reviewed/head/tested tree. Source approval and actual merge are correctly
distinguished from the pending results publication.

The report describes the additive explicit API accurately: strict send remains
Pending-only; retry also permits an unconsumed service-owned Reply under original
identity, lifetime, generation and semantic commitment. It preserves the Reply
charge and extraction semantics, without claiming an outer-envelope identity
store, new consumer, sequence, timer or resealing. Transport write/admission/Reply/
receipt facts remain distinct. Compiler-absence RED and later passing expansion
cases retain their actual status; the completed-write error test is not presented
as partial socket-write recovery.

All published counts match the retained approved scopes: six focused functions,
27 pre-connect refusal cases, five transport faults; local 127 all-target release
and strict Clippy; 125 coverage tests/19 protected binaries; 31/31 changed mapped
lines and 2,010/2,169 broader service.rs mapped lines. The two example tests explain
127 versus 125. All 159 broader unhit lines remain counted; branch mapping is NA.
The 285-entry source checks, protected pre-execution object custody and remote-only
raw coverage are stated within their recorded limits. No repository-wide or
semantic-path completeness is inferred from line coverage.

Hosted client totals are 92 per native row, 644 across seven rows, plus eight
separate Unix storage checks; absent Windows storage and native-client versus
native-MVM/WAN scope remain explicit. Original seven ZIPs and 236 inner checksum
checks are correctly retained; duplicate extraction cleanup is accurately named.
Hosted Linux is 92 live release and 135 shared (8 object-v1 + 127 adapter), with
strict Clippy/default guest/synthetic analyzers. Four zero-test result scopes and
overlapping targets/stages/platforms are explicit in the linked qualification
receipt; totals are not added as independent coverage.

README points its latest-client link to this checkpoint while leaving older
strict-transfer measurements at their own identities. No new codec-byte, CPU,
transfer-time, energy, WAN or end-to-end recovery benefit is claimed for an API
unused by the normal engine. The plan checks only the registered retry seam and
keeps endpoint reconciliation, general ACK loss, closed-stream resume, retention,
installed ownership, aggregate budgets, scale, platform/WAN and full C8/roadmap
acceptance open. Future untracked reconciliation contracts/plans are excluded
from this publication review. Existing evidence is not rewritten or normalized.

## Final publication document identities

These hashes were taken after reading the final scoped content. Only this new
review was written by this reviewer; root owns final top-level packet hashes,
staging, commits and publication.

```text
5cf4ce9d6d478bc79453f7b96f83c35073881affb1ca0b8b8ad98d29534a7f9e  README.md
1c18f32e695b9284134c3d04879fa7b00a5cba866e014d2128a77d09d29aa459  docs/plans/2026-11-03-persistent-mvm-service.md
d80b241a93f4cec29534a0f60f0a91d2dba2a46df7c10ca46d85397ded06aae1  docs/network/registered-call-retry-results-2026-11-03.md
f7a883c6628e0177df21d46770360b37199c1a5378ecef0b44593675e068eda7  docs/network/evidence/2026-11-03-call-retry/README.md
```
