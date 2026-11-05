# Independent Code / Rust / Security review — PASS, scoped CLOSED preparation

Reviewer: retained_debug, not the author of the inventory delta. No product/source edits, build or test rerun were performed for this review. I read the exact delta, four full selected source files and the immediate existing custody/revalidation/funding/drop implementations; independently hashed the selected source and archived receipts; and inspected the recorded commands, test outcomes, source/executable identities and compiler JSON diagnostics. This is source/receipt review, not a fresh execution or native acceptance claim.

Reviewed freeze: `/tmp/spiralcore-retained-catalog-20261104/stage-b/b2-live-inventory-freeze/`.

- delta.diff SHA256 `855ac9467e4f3253c63531a344b92dbb7d1d3620e24bb867f8f10eb9f482f4a9`.
- source.tar.gz SHA256 `c190d24970824272f47c4135b0f9ab629575165dd4bd37fc0a62190a049ec5e9`.
- Four selected source hashes match source.sha256.json exactly;44 indexed receipt hashes match. The140-input before/after maps equal compiled-source140.json (SHA256 `cbe40009b36c3309afe03312b32751f972d15de4d5925c83fec8b95ed899fcf9`).
- `installed_resources.rs` `6d1c291f784cc2c88795deae38f70010f7918b28d7086d3df3d5c417888e99e7`.
- `installed_owner/retained_transition.rs` `8f928b20eec67d4889f60488c5d8ae0dfff06fab0886d731858595af69310c53`.
- `installed_owner/retained_recovery.rs` `54fce5e0b38703d09fedf26fba7e9e2c4913d6568a97b41e70bc64e5eb69e84a`.
- `installed_owner/retained_live_tests.rs` `d92ba1e9c37c853944a385bea1b278b931453948374bb1c6eb8333f87bd63567`.

No blocking issue found in this narrow change:

1. `prepare_live_inventory(self)` consumes the actual committed CLOSED debt into one Mutex. It moves the same boxed Custody, inline fixed table, actual funding/ledger and original File/owner/holder/protection objects. There is no Clone of the controller, second table, new ledger, reserve, OPEN transition, issuer or grant. The test-only table-field address is inside the original boxed Custody, so its address remains stable across the consuming outer move; the ledger address is likewise its actual allocation identity.
2. Validation stays under the controller lock: existing fresh full catalog/file/held-set revalidation, all phase4, then the existing unique/unaliased healthy-CLOSED funding checks. Failures latch the original held.failed. Restoring membership bytes cannot clear that latch. A poisoned controller mutex also stays inaccessible; no poison-clear API is introduced.
3. Preparation refusal owns the original CommittedRetainedRestart. Successful controller Drop still reaches the established RetainedRecovery Drop, which retains the complete bounded custody instead of accidentally releasing funding/locks across uncertainty. Error Debug stays fixed and payload-free. No new raw-FD adoption or unsafe block enters the product delta.
4. Snapshot remains metadata-only and freshly checked: historical count, zero new rows, checked remaining-slot subtraction and the SAME managed snapshot. It creates no admission authority. The future physical mixed inventory, new-row append/activation and ordinary capacity/saturation remain unimplemented and must retain their own gates.
5. Recorded evidence is nonvacuous within its stated scope: actual old-process predecessor, one and sixteen funded historical rows, SAME table/ledger addresses and fixed metadata debit; a correctly encoded16→15 membership subset is refused and left unchanged, then remains sticky after restoration. Synthetic added phase4 records/key/marker data is clearly disclosed as format/accounting/File-custody coverage, not opaque retirement or native producer evidence.

The archived records show exactly three E0599 API-absence errors before implementation; build0;3 selected inventory filesystem cases,14 selected existing B2 controls and1 ABI guard; scoped Clippy exit0. I independently counted ordinary JSON warnings:2 build and79 strict (not zero warnings). Protected executable SHA256 `deebd7bcbce8473d93310b47657987fc43e985033a7360b9045a894beafc29b8` agrees in final and after-test records. Nested helper process executions are not counted as additional unique top-level tests. No full suite was rerun.

PASS authorizes this CLOSED component's integration into a separately validated successor at these exact pins. It does not open authority, qualify a newer SDK graph, complete retained restart/activation/launch/holder conversion, establish16 native guests, or close native/platform/WAN/capacity/FIPS acceptance. Those limits in the author freeze are appropriate and remain binding.
