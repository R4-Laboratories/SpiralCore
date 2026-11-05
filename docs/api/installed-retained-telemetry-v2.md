# Retained telemetry v2: debt-preserving restart and retained stop

2026-11-04. B1 format/transaction contract, proposed for independent architecture review before product implementation. This is a local Root storage format, not a MVM wire/crypto envelope or a permission constructor. Stage A source remains separately identified by development commit `325f662b41bf1c5fbcc9ce978fc7c22d8fa4c070`. The shipping/native profile is inactive. Codecs and private serialization fixtures cannot qualify useful restart or manufacture an EndpointRetirement.

## Binary catalog, frozen B1 layout

All integer fields below are unsigned little-endian except embedded owner records, whose existing SCOWNER1 big-endian encoding is preserved verbatim. Checksum means SHA-256 with an exact domain; there is no CRC32, encryption, MAC or authority claim. Unknown tags, nonzero reserved bytes, trailing/short data, zero/overflow identities and noncanonical re-encoding refuse.

SCRETN01 and its decoder/zero-tail semantics remain unchanged. SCRETN02 has the SAME 88-byte header, 4,352-byte row and 32-byte final checksum, with at most sixteen TOTAL rows across live and historical generations:

| Header byte interval | Meaning |
|---|---|
| 0..8 | ASCII `SCRETN02` |
| 8..40 | Installation ID, unchanged across transfer |
| 40..72 | Current activation profile |
| 72..80 | Current activation generation, nonzero |
| 80..82 | Row count, <=16 |
| 82 | Mode: 0 Active, 1 StoppedRetained |
| 83..88 | Zero |
| after final row | SHA256(`spiralcore/retained-telemetry-catalog/v2\0` || all preceding catalog bytes) |

Each row preserves v1 bytes 0..4296 with identical field meaning, byte order, phase/cap validation and path rules. The former reserved tail is used only in v2:

| Row byte interval | Meaning |
|---|---|
| 4296..4328 | Immutable origin profile, nonzero |
| 4328..4336 | Immutable origin generation, 1..=current generation |
| 4336..4352 | Zero |

Origin installation is the common header installation. Durable row identity is `(origin profile, origin generation, original serial)`. Session, state path and state `(device,inode)` stay globally unique. Serial reuse in DIFFERENT origins is allowed; duplicate origin tuples refuse. Preserve row order for reverse proof. All existing v1 per-row validation remains; origin metadata is not an old grant, weak scope or receipt.

Exact logical catalog length is `120 + 4352*N`, at most **69,752 bytes**. The decoder checks metadata length and count before any allocation, then exact canonical length/padding/domain/hash/re-encoding. A declared length/count never authorizes capacity.

## Canonical inverse / no-loss transfer proof

A v1-to-v2 target preserves EVERY old row byte 0..4296, adds the old header profile/generation to its origin tail, and changes only the activation header/magic/checksum/mode. Reconstructing v1 sets the target header back to the exact intent's old key/generation, sets all six old reserved header bytes and all 56 old row-tail bytes to zero, and uses the original v1 domain. Its entire reconstructed file SHA-256 must equal the intent's old-catalog SHA; its length/count/order must also match. No key digest is backfilled.

For v2-to-v2 transfer, restore the old activation header/profile/generation/mode and v2 checksum while preserving all row/origin bytes. Again require exact old-file digest/length/count. The target is exactly the same ordered membership, with no added/deleted/evicted row or altered phase/ceiling/File ID/records seal. New live rows are a later operation under the new active owner, not part of this transfer.

## Immutable transfer intent, frozen B1 layout

Canonical name `.retained-telemetry.transfer`; bounded staging name `.retained-telemetry.transfer.next`. Magic `SCRETX01`; fixed size **768 bytes**. No stage counter is rewritten: actual old/new state and catalog bytes determine the finite recovery position. All comparisons/repair occur under actual original owner EX, holders EX, canonical state protection and compiled profile compatibility. The intent is transaction metadata, never authority.

| Byte interval | Meaning |
|---|---|
| 0..8 | ASCII `SCRETX01` |
| 8 | Source catalog version, 1 or 2 |
| 9 | Kind, 1 Restart or 2 RetainedStop |
| 10 | Target catalog version, exactly 2 |
| 11..16 | Zero |
| 16..48 | Installation ID |
| 48..80 | Old activation profile |
| 80..112 | Target activation profile |
| 112..120 | Old generation |
| 120..128 | Target generation |
| 128..144 | Original installation directory `(device,inode)` |
| 144..160 | Original owner-lock `(device,inode)` |
| 160..176 | Original holders-lock `(device,inode)` |
| 176..208 | SHA-256 of entire old catalog file |
| 208..240 | SHA-256 of entire target catalog file |
| 240..272 | SHA-256 of original unchanged 112-byte denial beacon |
| 272..448 | Exact canonical OLD owner record (176 bytes) |
| 448..624 | Exact canonical TARGET owner record (176 bytes) |
| 624..632 | Old catalog file length |
| 632..640 | Target catalog file length |
| 640..642 | Preserved row count, 1..16 |
| 642 | Old header mode; v1 implies 0 |
| 643 | Target mode; Restart=0, RetainedStop=1 |
| 644..736 | Zero |
| 736..768 | SHA256(`spiralcore/retained-telemetry-transfer/v1\0` || bytes 0..736) |

Embedded owner records must canonical-decode and match the installation, profiles, generations and the THREE same original object IDs above. Both owner phases are Active: no Clean intermediate. Restart requires checked target generation=old+1 and a compiled old-model-to-exact-current-config migration. Stop requires identical old/target owner records/profile/generation. Digests, source format and source mode are checked against the actual pinned source, never copied as an authority answer.

The original beacon remains byte-for-byte unchanged. V2 validation uses its immutable first-debt lineage; it does not demand that every later activation equal that original epoch and does not overwrite the beacon. Generic begin/reconcile/finish continue denying any nonempty beacon/catalog, including transfer files.

## Fixed capacity arithmetic

No quotas are increased. Per retained row K remains **32,768 bytes**, including temporary overlap:

- Two catalog row images: 2*4352=8,704.
- Two bounded status images: 2*4096=8,192.
- Two bounded wrapped-key images: 2*1024=2,048.
- One protection marker allowance: 1,024 (actual current marker224).
- Total **19,968**, leaving **12,800** per row. At16 rows the same per-row arithmetic applies; no second per-generation table is created.

The SAME pool's fixed 8,192-byte global-header allowance covers two catalog header+checksum images240, two transfer-intent images1536, two owner-state images352, one existing state intent48 and original beacon112: total **2,288**, leaving **5,904**. Zero-length lock files add no logical file bytes. At most two catalog images coexist, including an unlinked image still held open; no third backup is permitted. Stopped mode adds no separate persistent receipt file.

The existing 1 MiB managed metadata allowance must cover both current and candidate data. Conservative requested-Rust-storage worksheet: two raw full catalog buffers139,504; three <=96KiB decoded/encoded candidate row/path sets294,912; 128KiB accepted+fresh descriptor/path/controller envelope131,072; 128KiB ledger/table/Arc/allocator-request envelope131,072; 32KiB fixed intent/state/parse/hash temporaries32,768; 128KiB additional setup/overlap131,072. Total **860,400**, margin **188,176**. These last object/request envelopes are constructor/ABI/capacity guards REQUIRED before B2 activation, not a measurement or proof from sizeof alone. B1 must avoid retaining three decoded catalogs: inverse proof can re-encode one candidate serially. Validate actual container capacities/row counts and guarded object sizes/source limits; unsupported layouts refuse. Native allocator/TLS/kernel FD and whole-process RSS are separate explicit domains, not hidden zero-cost terms. Logical disk accounting is not physical allocation proof. Child status/key writer gates and same K coupling remain mandatory.

## Private B1 interfaces / B2 ownership boundary

The format leaf exposes crate-private `CatalogV2`, `OriginRecord`, `TransferIntent` and `TransferKind`; no public path/FD/quota/grant constructor. Proposed codec seams are `CatalogV2::{from_v1,encode,decode,reverse_source}` and `TransferIntent::{encode,decode,verify_reverse}`. They accept private epoch/record facts for codec work only and never perform I/O or return an owner. Private format fixtures may construct phase4 byte records solely for codec/durability tests. They are not native seal or restart evidence.

The B2 controller seam is `ClosedRetainedDebt::prepare_restart(self, &CompiledRetainedMigration) -> Result<PreparedRetainedRestart, RetainedRestartFailure>`, then a consuming `commit(self) -> Result<InstalledSupervisor, RetainedRestartFailure>`. The migration producer derives its target profile from the ACTUAL known new config/model; caller-supplied arbitrary profile hashes/limits are not supported. The original expected old key must match a compiled supported source/model identity, including the qualified previous source and current-source recovery compatibility before customer release.

B2 requires ALL rows validated phase4, no untracked disposable namespace, complete actual original backend/holder coverage and healthy unaliased custody. Phases1/2/3 stay closed unresolved debt. Any alias, poison, unknown phase or final wait failure retains the same controller and charges. No positive native receipt is fabricated.

The actual ledger, charged slots, fixed metadata ownership and historical File table move together into a NEW activation authority. No default/zero ledger or duplicate fixed-metadata reservation is created; the old closed flag is never reopened. Historical rows retain actual imported accounting-slot ownership distinct from durable source serials. They participate in full held-set equality, duplicate membership and the same sixteen-entry capacity. No old scope Weak, VM grant, registration or receipt is recreated. Fresh canonical plan, signature, parent, epoch and backend admission remains required; new state must not overwrite a retained old same-name directory.

## B2 durable order and finite recovery

1. Hold actual owner EX and holders EX, full same-ledger debt and original File pins. Freshly revalidate complete eligibility and compiled old-to-current profile mapping. Source Storage.key/Record remain paired; create a separate pending target Storage view over the SAME object IDs, never mutate the old key early.
2. Write the immutable complete transfer intent via its bounded staging file, sync file, rename canonical intent and sync original directory BEFORE catalog/state publication. Unknown/partial intent stays closed; readback alone after failed sync is not completion. Recognized retries use the SAME target, never another generation.
3. Write/sync target catalog using the existing bounded next name; rename canonical and sync directory. Original markers, beacon, ciphertext and wrapped keys are untouched. Reverse proof must reconstruct the exact source catalog.
4. Publish exact target Active owner record through its existing intent/next/rename/directory barriers using the pending target Storage view. Only after verified durability install target Storage.key and target Record together. Any error retains both views, intent and actual ownership; no generic current/begin bypass or Clean publication.
5. A restricted recovery opener under NEW original owner EX/holders EX recognizes old-state/old-catalog, old-state/target-catalog, target-state/target-catalog and exact known state-temp combinations against the immutable intent. It rolls forward the same target after full re-funding/validation. Target-state/old-catalog, unknown bytes, incomplete/unrecognized intent or lineage mismatch stays closed. Do not broadly delete state intents to make ordinary open succeed.
6. Clear only exact completed metadata intents after known durability; error stays uncertain. Transfer healthy unaliased actual custody into the new authority. Qualify the native holders EX-to-lifetime-SH handoff while owner EX remains held: no presumed flock atomic conversion, no dup-as-independent-lock, no issuer until verified. Preserve original children/backend finality; exclusion alone is not EndpointRetirement.
7. Only successful final checks expose the new supervisor. Admission includes old FULL R+K and new resource reservations in the same aggregate; all16 occupied refuses another scope. No expiry/delete authority or unlimited restart/fleet claim is supplied.

## Graceful retained stop is a different terminal outcome

Keep existing global `try_finish` zero-drain/Busy semantics. Add a distinct consuming `prepare_retained_stop` transaction only after actual guest/session/helper/collector cleanup and positive original custody proof. Persist v2 StoppedRetained mode and a RetainedStop intent at the SAME epoch/profile, sync all exact files and transfer full nonzero ledger/table/locks into an opaque `StoppedWithRetainedDebt` controller. This is durable preservation, not Clean, a refund, telemetry deletion or application acknowledgment.

The daemon may report that typed nonzero-debt stop outcome and exit while the ORIGINAL outer process owner retains the controller through actual process exit; the parent still positively waits its original Child before releasing its process partition. In a surviving process the stopped controller remains charged/closed and can feed the same StageB restart transaction. It cannot be discarded to create a new zero-ledger supervisor. This permits an honest daemon-exit path without weakening global finish; per-installation release to offline custody inside a still-running multi-installation process needs its own reviewed ownership transition if later required.

## Execution milestones

B1: freeze this schema, capture genuine missing codec/intent API RED, implement pure versioned codec/inverse/intent validation, then selected release/strict source-identity qualification. Tests include v1 preservation, all16 origins, duplicate tuples vs allowed cross-generation serial reuse, padding/length/tag/domain/overflow corruption, exact inverse proof, immutable intent and Restart vs Stop generation rules. B1 cannot claim implemented restart.

B2: actual filesystem/ledger/owner transaction and retained-stop wiring, each after behavioral RED; inject crashes at every file/directory barrier and between catalog/state key updates, then actual child restart must choose the SAME target with old debt/data intact. Test failed/contended holder conversion, aliases, poison, no metadata double charge, full-table saturation, old projection refusal, graceful nonzero-debt stop and positive parent Child collection. Recovery/source hashes, protected binaries and failures remain distinct.

Useful-service acceptance still requires an actual canonical native producer to create a phase4 row through real EndpointRetirement, followed by a new admitted guest transfer after restart while old R+K remains charged, then two guests, failures, long sessions and final platform/WAN/capacity qualification. Such a row is not yet available. Private codec fixtures do not close this gate.
