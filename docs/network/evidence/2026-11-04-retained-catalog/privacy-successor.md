# Retained telemetry privacy successor review

**PASS for the narrowly reviewed privacy source corrections at the hashes below.** The prior conditional P2 repeated-directory check is resolved in source. The specific missing-catalog continuity defect now has a fail-closed source correction; this is not approval of a general recovery/adoption implementation. Production retention, platform and compliance qualification remain HOLD.

This is an independent successor source review of only the retained catalog delta. The original review remains historical at `privacy-review.md`. No product code was changed, build/test was run, or native process was operated by this reviewer. Root reports an actual Wick GREEN run with five entries (including one helper) and an actual nested child; that is Root-owned execution evidence and is not represented here as reviewer execution. No native macOS ACL execution has been supplied by this review.

| Reviewed input | SHA-256 |
|---|---|
| `crates/spiralcore-mvm-live/src/installed_retained.rs` | `f8a8540bf2de420c261ad53a672be5c32e75db4b5784112faf7b21ca4e5193da` |
| `crates/spiralcore-mvm-live/src/installed_owner/retained_telemetry.rs` | `5f8e2703a9931aabf7fdbcaa990372d1ff3268614b463bed4dac5c119ef1a985` |
| `crates/spiralcore-mvm-live/src/installed_owner.rs` | `69f0b9284d62edf1b14ed1d53be0e20bd2e4841542aff4a22a16dcabe175bdfd` |
| `crates/spiralcore-mvm-live/src/installed_supervisor/retained_tests.rs` | `e03871cb1024d09ecc7ab2ef48f9265ab803e5a40fdceea0a656b3cf0985d090` |

## Corrected behavior

1. Before append or update, the durable catalog must equal the complete set of original records held by the inventory. The exact comparison includes both count and full record equality. The append path snapshots the old set before arming the proposed new record; the update paths supply the complete current held set under the original inventory lock. Missing metadata after A therefore cannot silently become a successful B-only catalog. A mismatch refuses before catalog/marker publication and closes admission while keeping original pins and reservations. This check grants no fresh authority from a decoded catalog.
2. Before the first retained publication, a bounded epoch-bound denial beacon is written through the original exclusively owned owner.lock descriptor, checked against the original inode, and synced. Existing unknown/partial beacon bytes refuse; no truncation, replacement, unlink or clearing was added. Generic no-debt epoch transitions reject any nonempty owner beacon even when the catalog has disappeared. This is conservative denial after loss, not reconstruction, deletion permission, adoption, or proof that every crash state is recoverable. Hostile same-UID truncation/replacement remains outside the admitted trust model.
3. Both binding and sealing now call the shared `require_private_retained_directory` predicate after original state-protection checking and before phase/file/catalog effects. That predicate composes `check_private(directory, true)` and `check_filesystem(directory)`, so the earlier macOS ACL/local-filesystem predicate is repeated rather than relying only on an initial observation. The conditional P2 source omission is closed. macOS native ACL execution remains an explicit unrun platform gate.
4. Actual records descriptors must be O_RDWR plus O_APPEND at binding and sealing. The callback refuses a write-only handle before Bound publication, avoiding a known deferred EBADF at final positional read. No alternate file is synthesized. Fixed-name and actual-descriptor identity/private-file validation remain in place. Positional reads preserve the shared producer offset. A descriptor flag condition is not proof of canonical producer integration or valid ciphertext format.

The reviewed fault tests are focused: actual metadata deletion before a second append, child process exit followed by catalog-loss reconciliation refusal, write-only descriptor refusal with later readable retry, and changed private-directory mode refusal. They retain original markers, catalog snapshots and/or usage as applicable. The test source exercises ordinary filesystem privacy on Linux, not macOS extended ACLs; its presence alone is not execution evidence.

## Privacy and public-claim boundaries retained

The catalog and denial beacon contain plaintext sensitive metadata/opaque identities. SHA-256 checksums are not encryption, authentication, authority or a FIPS validation. The encrypted records and wrapped-key files remain opaque original artifacts; this component does not validate the ciphertext format, execute key rotation, decrypt, rekey, or silently delete them. Original tenant/session/capability and actual directory/file pin checks remain necessary independently of catalog content.

These fixes add no raw-record, wrapped-key or arbitrary-path diagnostics. They introduce no new plaintext telemetry copy, network disclosure or disposal operation. Preservation is conservative: resource debt and original pins remain held on uncertainty. That means retention can continue until a separately authorized lifecycle is implemented; it is not a completed minimization/expiry policy.

Keep file preparation, binding, endpoint retirement, sealing, disposable cleanup, VM teardown and application acknowledgment distinct. Neither `Drained` nor signed `destroy_on_exit` creates audit-data deletion authority. General I/O-failure retry, closed-debt adoption, authorized retention/expiry/disposal, strict producer composition, actual native positive retirement, aggregate memory/disk qualification, capacity and WAN acceptance remain open. The inactive production profile must not be relabeled supported by this review.

For the unclassified/CUI target, no concrete FIPS 140-3 validated cryptographic module/version/operating environment/approved-mode profile is established here. There is no certification or government/system compliance approval. The source PASS closes only the stated privacy corrections and supports the limited fail-closed continuity behavior; all broader production/retention and platform/security claims remain HOLD pending their own evidence.
