# Windows quota scan repair

This is a bounded implementation detail of the authorized ordered-cache repair.
The [design](../specs/2026-11-10-windows-quota-batch.md) preserves authoritative
filesystem accounting and all per-file checks. The existing Windows sustained
workload is the performance RED; the new quota API test also fails with E0425
under the pinned Windows-target metadata check. No protocol changes are planned.

1. In `crates/spiralcore-storage-windows/src/windows.rs`, add bounded kernel
   GUID-name and single-child validators, then a private synchronous directory
   owner. Request directory-list access with read-only sharing, retain original
   ancestry, and verify identity/volume through acquisition. Depend on the
   reviewed design. Risk: high, because this is a filesystem trust boundary.
2. In the same file, implement a scalar-only `quota_usage` scan of the anchored
   root and namespaces. Recheck effective identity and each publication's
   original access, ACL, type, links, reparse state, parent and held-file size.
   Preserve checked totals and count limits. Depend on step 1. Risk: high.
3. Extend the existing two native suites in
   `crates/spiralcore-storage-windows/src/tests.rs` with positive accounting,
   ACL changes, hardlinks, sharing/rename refusal and release, and anchored
   junction cases. Record paired old/new metadata timing and CPU with synthetic
   files. Keep their enumeration count stable. Depend on steps 1–2. Risk: medium.
4. In `crates/spiralcore-transfer/src/endpoint/storage/windows.rs` and
   `crates/spiralcore-transfer/src/endpoint.rs`, route Windows accounting through
   the bounded scan. Other platforms and all content reads retain their path.
   Depend on steps 1–3. Risk: medium.
5. Retain successful native primitive output in
   `.github/workflows/mvm-mailbox-portable.yml`, run pinned metadata/Clippy checks
   and Linux component regressions on Wick, then execute the complete native
   matrix. Compare the unchanged 4,096-block, 240-second workload and exact
   traffic/readbacks with the recorded baseline. Depend on steps 1–4. Risk: medium.
6. Update `docs/network/evidence/2026-11-10-ordered-scope/README.md` and issues
   #44/#49 with successes and remaining failures. Obtain independent Rust,
   security and infrastructure confirmations, then merge only the tested head
   into `origin/main`. No installed-MVM or WAN completion follows from this
   component result. Depend on step 5. Risk: low.

The release gate retains every original count, fidelity, cache and operation
deadline assertion. A missing native platform gate remains incomplete.
