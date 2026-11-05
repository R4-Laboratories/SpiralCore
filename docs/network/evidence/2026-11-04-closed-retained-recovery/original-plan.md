# Retained telemetry restart adoption — scoped ADR and test-first plan

2026-11-04 — **Proposed design; no implementation or execution performed.** This closes neither recovery nor installed-service delivery. It specifies the next bounded implementation and the additional transition required to restart useful service. The full goal still requires actual canonical producer/guest transfer, original shutdown/retirement, two-guest failure/recovery, sustained sessions and final platform/WAN qualification.

## Decision

Implement **closed adoption of the existing active epoch first**, then a distinct **debt-preserving new-generation activation transaction**. Closed adoption imports all known debt and grants inspection/custody only. It never reopens the old supervisor or returns a native issuer. Useful restart requires the second transaction, fresh canonical admission and actual backend/holder qualification; it is not achieved merely by loading a catalog.

Keep records in their existing canonical locations. Keep every historical scope charged at its original full R+K, including unresolved scopes. Do not delete, truncate, decrypt, reset counters, clear the beacon or synthesize EndpointRetirement. Do not infer absence of debt from missing data. A missing/corrupt catalog with a nonempty beacon remains blocked; automatic reconstruction from a guessed directory scan is outside this increment.

Prefer a new generation over reopening the old generation. Existing installed grant/session identities include generation and ledger serial. The fresh ManagedResourceBudget starts its serial counter at 1; the catalog preserves only retained rows, not a complete durable history of all issued grants/workers. Seeding from the largest retained serial alone cannot establish that no old activation identity is reused. A same-epoch resume would require an independently specified durable nonreuse/activation-incarnation design and review of stale-epoch, signed-plan, driver wait-failure and capability semantics. It is not the minimal safe option supported by the current source.

## Source basis and precise present gaps

Read the full goal objective at `/Users/casey.allard/.codex/attachments/2bb12934-0052-415e-a820-7cd9b48dd1dd/goal-objective.md` (SHA `9f611438b6aaa87bcf25b9f948ccf9dda43cadaf82ba047110f86628b45c53a1`) and the actual six requested source/contract files. Principal source hashes at design time:

| File | SHA-256 |
|---|---|
| installed_owner.rs | 69f0b9284d62edf1b14ed1d53be0e20bd2e4841542aff4a22a16dcabe175bdfd |
| installed_owner/retained_telemetry.rs | 5f8e2703a9931aabf7fdbcaa990372d1ff3268614b463bed4dac5c119ef1a985 |
| installed_retained.rs | f8a8540bf2de420c261ad53a672be5c32e75db4b5784112faf7b21ca4e5193da |
| installed_resources.rs | ee7ee5faccb4181f18c33c534dfce7aa64de10ea379bf73741142f2ac1fbfa62 |
| installed_supervisor.rs | 55b85c40fdfaecc53217233290fa3a11763a9f89f6d1849c1bbeb2f146bf3d46 |
| installed-retained-telemetry-v1.md | 5a180f27610efad61046e36b0508aba3c00bc76f62209a172dd3f6f9271d9ed4 |

Relevant actual behavior:

1. InstallationOwner::open retains original owner EX but leaves supervisor=None. begin increments Clean to Active and calls no_endpoint_disks; reconcile/finish also deny any nonempty owner beacon or catalog. This is correct denial, not an adoption path.
2. holders_for_transition obtains a fresh holders.lock EX description. Successful exclusion is meaningful only for the resources actually required to retain the original holder. It does not retroactively create a collected Child or a positive EndpointRetirement.
3. Catalog SCRETN01 is tied to ONE installation/profile/generation in its header. It has at most 16 entries, a fixed 4,352-byte row and maximum 69,752 bytes. A row has origin-like parent/serial/session/state IDs, phases 1–4, records/key IDs only in phases 3/4, and a committed records length/hash only in phase 4. The 224-byte state marker and 112-byte owner beacon bind the original epoch/profile. Neither can simply be relabeled to a new profile.
4. RetainedTelemetryInventory currently owns new live ManagedReservations; RetainedTelemetryScope is only an in-process Weak lookup. A decoded RetainedRecord must not reconstruct that original grant/scope or its native receipt. New accounting custody can be created only by the recovery controller after kernel exclusion.
5. ManagedResourceBudget::reserve requires admission_open and current active epoch. Closed recovery therefore needs a narrowly private import transaction; briefly setting open=true in order to call reserve is forbidden. Existing close_admission is monotonic and must stay so for ordinary supervisors.
6. InstalledConfig::profile_hash includes source/model definitions through include_bytes. Implementing recovery can change the new executable's profile hash. Old state/catalog must open with the explicitly expected OLD InstallationKey and validate that original profile. Passing the new config key or silently overwriting Storage.key is invalid.
7. Existing recover_endpoint_disks is separate disposable deletion authority after exclusion. It must not remove telemetry state. Its public entry obtains holders EX itself, so do not nest it underneath an already-held independent EX description; factor a private checked leaf for the recovery controller if needed.

## Minimal Stage A API and custody

Use private types in the existing Root installed-owner/supervisor modules, not a new MVM wire capability. Names below are proposed exact seams; write their contract and API-absence tests before product code.

```rust
// All fields private; non-Clone, no public from-path/FD/catalog constructor.
pub(crate) struct RetainedRecovery { /* owner EX, holders EX, closed authority,
    SAME pool, fixed metadata, pinned catalog/rows, bounded outcome state */ }
pub(crate) struct ClosedRetainedDebt { /* the completed closed controller */ }
pub(crate) struct RecoveryBlocked { /* SAME original custody, error detector */ }

impl InstallationOwner {
    pub(crate) fn prepare_retained_recovery(
        self,
        compatibility: &RetainedRecoveryCompatibility,
        layout: &CanonicalRecoveryLayout,
    ) -> Result<RetainedRecovery, RecoveryStartFailure>;
}
impl RetainedRecovery {
    pub(crate) fn adopt_closed(self)
        -> Result<ClosedRetainedDebt, RecoveryBlocked>;
}
impl ClosedRetainedDebt {
    pub(crate) fn snapshot(&self) -> Result<RetainedDebtSnapshot, RecoveryError>;
    pub(crate) fn revalidate(&mut self) -> Result<(), RecoveryError>;
}
```

RecoveryStartFailure distinguishes failure before original ownership was obtained from a failure retaining an obtained original owner. RecoveryBlocked retains all already-acquired exclusion, actual File pins and charged candidate state; it exposes no issuer, general reserve operation or activation method. Retry reuses that bounded controller; it must not accumulate leaked owners/ledgers per retry. Drop on uncertain imported custody uses the existing bounded quarantine pattern, retaining the actual controller/credits until process exit. There is no successful cleanup/refund interpretation of Drop.

RetainedRecoveryCompatibility is a compiled, explicitly reviewed allowlist of supported OLD profile/source/model identities and their exact R/K/layout definitions, not a caller-provided limits DTO or arbitrary old hash. CanonicalRecoveryLayout derives from the trusted existing canonical installation/runtime-root configuration and retains the actual authorized parent directory Files. Catalog absolute paths alone do not authorize opening arbitrary same-UID files: each must be a supported direct canonical state child under those original pinned roots. Tests can construct private component fixtures, but no shipping constructor accepts untrusted metadata as permission.

Ordinary InstallationOwner::open remains unchanged for intact state. A later transfer-recovery opener may handle only an exactly recognized pending transfer under original owner EX; it must not bypass all current no_temp checks. Stage A initially refuses state/catalog temporary or partial metadata rather than pretending it is an intact catalog.

### Stage A order

1. Caller supplies the explicit old key and obtains InstallationOwner::open, with original private directory, owner/holders identities and owner EX. Validate current Active old epoch and the exact old compatibility entry. Acquire a fresh holders EX description and retain it through the entire closed object. Existing owner/holder aliases in this or another process must make this fail Busy; no PID probing/rearming.
2. Create one closed InstalledAuthority and its one managed ledger. No public InstalledSupervisor, native issuer, worker grant, session or parent provider escapes. Use a private recovery state enum rather than reopening any ordinary closed authority.
3. Debit the existing fixed retained bookkeeping allowance (1 MiB host metadata and 8 KiB fixed disk headers) from that same ledger BEFORE allocating the catalog buffer/entry/File-owning table. A private linear import permit is bound to this actual recovery controller and actual ledger Arc; it accepts only the source-fixed recovery claims, never general allocations or activation.
4. Bounded-read the exact original beacon and catalog using nofollow/CLOEXEC/private ownership, ACL/filesystem and inode rechecks. SCRETN01 and the beacon must match the old key/generation and canonical checksums/encoding. Reject missing catalog with nonzero beacon, overflow, duplicates, >16 entries, unknown phases/caps, altered generation/profile, partial intents or namespace inconsistency. An apparently empty catalog with latched debt is not a zero-debt certificate.
5. Validate row ceilings against the approved old model, not merely numbers decoded from disk. Calculate all per-tenant and host claims with checked arithmetic outside the short accounting lock. Atomically import every FULL R+K (8 MiB+32 KiB for the current old profile), all tenants/slots and fixed allowances before any admission. Do not charge observed file lengths instead. New in-memory reservation serials and old durable row identity are separate fields: no reconstructed RetainedTelemetryScope, arbitrary supplied serial or old grant authority. Ledger import must be all-or-none; an allocation/panic/late-validation failure retains the closed candidate and cannot expose a partially funded service.
6. Acquire the existing Core protection guard for each distinct canonical state parent, using independently opened verified OFDs. Bounded deterministic parent ordering, nonblocking contention and no accounting lock across I/O. Retain each original state File and exact IDs, marker File and any known records/key Files. Revalidate names/IDs/privacy immediately before and after observations. Do not reopen from untrusted path data outside the authorized layout.
7. Check the phase-specific table below. Finish only with a closed funded table and explicit unresolved reasons. The snapshot reports catalog debt, observed bytes and unresolved state separately; no full-scope success/cleanup bool. Keep shared-parent protection locks while a missing/incomplete denial marker could otherwise permit deletion. Release them only after an authorized, separately tested marker-repair transaction durably establishes the same denial barrier; do not merely infer a marker from memory.

### Entire durable state table

| Persisted row | Required conservative result |
|---|---|
| Phase 1 (catalog creation intent, state ID known) | FULL R+K charged. Marker may be absent or exact; arbitrary/malformed marker blocks. Files may have ambiguous creation; no inferred absent/no-effect proof and no bind/launch. Retain state/protection custody as unresolved. |
| Phase 2 (protected, no bound record/key IDs) | Exact original marker/state required. Absent records/key is a valid unresolved outcome, not zero debt. Present files are only observed private candidates; absence of committed IDs prevents claiming original bound ownership. FULL R+K stays quarantined. This must cover the existing actual process-exit fixture. |
| Phase 3 (bound) | Exact private state/marker plus both committed file IDs and caps. Missing/replaced/oversized file blocks; do not reinterpret as phase2. No committed sealed digest or native receipt exists, so retain unresolved full debt and never promote it to phase4. |
| Phase 4 (sealed) | Exact state/marker/committed file IDs, lengths/caps and bounded positional read of the stored records length/hash; length/identity stable before/after. This validates retained bytes against the existing seal, not a new native receipt. Wrapped key remains opaque; v1 records only its ID/cap, not a historical content digest. Do not claim to validate a nonexistent old key digest or decrypt/reauthenticate it. |
| Missing/corrupt catalog; nonzero malformed beacon; catalog/marker/file mismatch; unrecognized transfer intent | Closed blocked custody; no normal begin, new admission, truncation, blind directory reconstruction or refund. |

For the first useful-restart profile, only a table whose rows are all validated phase4 is eligible for continued admission. Phases1/2/3 remain supported **closed unresolved-debt recovery**, not customer restart. Repairing interrupted phases later needs exact transaction recovery and/or actual canonical backend finality; no fabricated EndpointRetirement. This restriction must be visible as a supported recovery matrix, not silently described as general restart support.

## Stage B: required useful-service transition

### Version and profile migration

Introduce an explicitly specified catalog **v2**; do not change SCRETN01 semantics in place. Its aggregate header identifies the current activation installation/profile/generation. Every row additionally preserves immutable origin profile and generation together with its existing original parent/serial/session/state/File IDs and sealed facts. Original state markers remain bound to their origin and are not rewritten to pretend new provenance. Uniqueness of durable row IDs becomes `(origin_profile, origin_generation, original_serial)`; sessions and state object/path identities remain globally unique. The fixed total table capacity remains 16 across ALL generations, not 16 per generation. Adopted rows carry a distinct recovered historical custody kind, never an actionable old scope/VM grant.

The v2 format specification must freeze canonical field widths, padding, maximum total bytes, exact checksummed domains and V1→V2 mapping before implementation. Adding origin fields fits within the existing <=8 KiB per-row sidecar envelope; prove the complete new maximum, temporary-image overlap, transfer intent, owner-state intent and header sums within R/K/fixed-8KiB before use. If the proof fails, refuse the existing profile; do not raise quota silently.

A sealed, compiled migration compatibility entry must bind expected OLD key/profile/model to the exact supported NEW config/profile/model. This is necessary because code-derived current profile_hash changes. Check the installation ID is unchanged and new generation is exactly old+1 (overflow refuses). Keep the actual original installation/owner/holders inode identities. Never construct a fresh installation beside the old one or accept an arbitrary target profile from catalog JSON. During the transition retain an old-key Storage view and a distinct pending new-key view; only an explicit successful commit installs the latter together with its matching Record. No early in-place Storage.key reassignment.

Keep the existing nonempty owner beacon unchanged as permanent denial evidence of first debt. V2-aware validation recognizes its ORIGINAL profile/generation through the approved lineage/transfer, rather than requiring it to equal every later activation epoch. Generic no_endpoint_disks continues to deny ANY nonempty beacon. New generation's protect/update path must support this source-qualified original beacon; do not call old latch_retained_debt with a new epoch and then overwrite the old bytes on mismatch.

### Atomic logical transaction over existing durable publication primitives

Add one bounded fixed-name retained-transfer intent, not a new MVM envelope. It records the exact installation, old/new profile+generation, old/new state-record digests, old catalog digest, target v2 catalog digest and original beacon/object identity. It is a checksum/transaction description, never authority. Original owner EX, retained holders EX, compatibility policy, same ledger and actual file validation provide the operation's authority/custody.

1. While STILL closed and holding both exclusions, revalidate the funded full table and all phase4 origins. Resolve/recover disposable inventory only using its existing positive cleanup authority and the SAME retained exclusion; refactor its private leaf to avoid recursively acquiring an independent EX. No old disposable namespace or untracked attempt may be ignored before new admission. Finish all required actual backend/holder coverage checks.
2. Build bounded target v2 bytes preserving every old row and full ceiling. Persist the transfer intent and sync its file and original installation directory BEFORE publishing target metadata or state. The target catalog plus intent must be sufficient to verify every preserved old row/canonical v1 digest after a crash; no dependence on a process-only old File after rename. Avoid a third complete catalog copy outside the funded sidecar envelope.
3. Write/sync target catalog temp; rename to the canonical catalog name; sync directory. Publish the exact new Active owner Record using the existing state intent/next/rename/barriers. All state/object inode identities remain original. Do not publish Clean as an intermediate step or call normal begin over debt.
4. Verify new state, catalog, original beacon lineage and full SAME-ledger reservations again; clear only the exact completed transfer/state temporary entries after known durable barriers and original-inode checks. A failed directory sync remains uncertain, not successful because readback looks correct.
5. A crash at ANY step is recognized only under a new original owner EX plus holders EX. The dedicated recovery opener handles the finite old/new state/catalog combinations and resumes the SAME target transaction after re-funding/revalidation. Unrecognized or partial intent stays closed. In particular ordinary InstallationOwner::open currently refuses state .intent/.next before construction: the restricted transfer opener must explicitly handle those exact matching bytes rather than removing generic guards. Roll forward to the committed target; no rollback that loses retained debt. Repeating recovery must not advance another generation for the same pending transfer.
6. Transfer the actual closed table, ledger and original owner into the new supervisor; do not rebuild/refund them. Install its lifetime SH holder before releasing recovery exclusion. The implementation must qualify the platform lock conversion/hand-over without an unprotected authority gap; do not assume dup creates an independent lock or that flock conversion is universally atomic. Owner EX remains held, and no issuer escapes until the holder handoff and final checks succeed. Any ambiguity returns SAME closed custody.
7. Only now may a private one-use transition publish a new Open supervisor/native issuer. This is a new activation controller, not reopening a normally closed one. It receives the existing funded ledger/table; fresh worker/grant serials are scoped by the new generation, and fresh canonical signed-plan/config/backend admission remains mandatory. No old grant, stream, registration, endpoint receipt or failed final wait is resurrected. Source-supported holder exclusion is not a substitute for unqualified backend finality.

Continue enforcing fresh parent/anchor/original registration and endpoint rules. The new service must allocate new scopes under distinct canonical state directories; it cannot overwrite an old same-name retained VM directory or append through an old sealed writer. If the canonical name generator can collide, add/use its existing supported fresh name path and refuse reuse before effects; no new alternate routing or protocol. Debt consumes the same tenant/host pool and table slots before new admission, so saturation refuses honestly. With 16 retained entries, infinite restart/VM turnover is not supported absent separately authorized archival/disposition; preserve that capacity limit and do not claim an unlimited service.

## Test-first implementation sequence and ownership

No products were edited for this plan; every test below is planned, not executed.

**Batch A — smallest next implementation:** Root owns its installed-owner/retained/supervisor/resource modules; canonical MVM files are unchanged. Add a written additive recovery contract and API-absence probe. Then implement original owner+holders custody, private closed ledger import and full phase-table validation. Reuse the existing protected old helper to create actual old-profile phase2 debt and `process::exit` without Drop; the new process receives the explicit expected old key/qualified compatibility. This exposes the real include_bytes profile boundary instead of constructing old debt with the new binary and calling it migration.

| Planned regression | Required observation |
|---|---|
| Actual old process exits with phase2/no record/key | Closed adoption charges FULL R+K+fixed metadata, retains state/marker, reports unresolved; no issuer or new reserve/worker/session. |
| Old owner or any holder remains live, including independent process | Busy; exact directory/catalog/beacon/data unchanged; no PID fallback. |
| All four phases; 16-row table and per-tenant totals | Exact full claims/slot counts, not observed lengths; phase2 absent files nonzero; 17th row/saturation/overflow refuses. Phase4 private serialization fixtures are format tests only, not native finality proof. |
| Missing/corrupt catalog/beacon, wrong key/profile/cap/hash, replaced/private-mode/ACL/link/path object | Closed refusal with SAME acquired custody; no data deletion or generic epoch reset. Bound/sealed missing files never become empty debt. |
| Fault/panic after metadata reserve, during row validation/import and on late freshness check | No partial open service, no duplicate import/refund, bounded retained owner; retry idempotent or explicitly blocked. |
| Simultaneous reserve during paused import | General admission is closed throughout; no temporary open window. Same-pool snapshot equals restored full debt before any consumer is exposed. |

Only after this component is qualified should Stage B add its separately reviewed v2/transfer contract and test-first implementation. Existing owner/storage/resource tests must remain; exact source/lock/binary readbacks and scoped Clippy precede a development checkpoint.

**Batch B — useful restart:** First verify that normal start still refuses debt. Add genuine transfer/API REDs, then v1→v2 codec and compatibility checks, full-table preservation and all durable barrier crash injection. Test crashes before/after intent file/directory sync, target rename/directory sync, owner state intent/rename/directory sync, cleanup and final holder handoff; restart each with original locks and prove exact old ciphertext/keys/markers retained, same target generation, no early issuer, no transient zero/duplicate debt. Test source-profile migration, unknown profiles, generation overflow, all16 historical slots, new-generation serial reuse scoped safely, old signed projections refused and old same-name state untouched.

**Batch C — required acceptance, not replaceable by fixtures:** Produce an actual phase4 row through the strict canonical original records/key producer, actual installed guest transfer, original endpoint joins and real EndpointRetirement. Crash/restart, adopt/transfer it, start a new admitted guest while the old FULL R+K stays charged, transfer exact data, stop and prove new disposable retirement. Repeat two guests, process/backend/collector failures, final-wait uncertainty and sustained Calls. Native Linux/macOS roles, resource/RSS capacity and real WAN remain their own gates. A private test row or a closed adoption PASS cannot satisfy this acceptance.

## Consequence and immediate handoff

The next concrete coding task is **Batch A actual old-process phase2 closed adoption with FULL debt**, including the private ledger-import boundary and profile compatibility test. It has a narrow owner-side source scope and does not depend on inventing a native receipt. In parallel with that sequence's natural checkpoints, the already-authorized strict producer/native path must advance so a real phase4 fixture exists for Batch C. The exact next step from closed inspection toward useful customer restart is Batch B's reviewed versioned debt-preserving generation/profile transfer; merely returning ClosedRetainedDebt is explicitly insufficient.

No telemetry expiry/deletion/adoption-as-authorization was found in the source. Retain by default, preserve upstream transient volume/session/final-wait semantics, and expose blocked/unresolved outcomes clearly. Recovery is complete only for the explicitly tested state/role matrix; the full installed-service goal remains active.
