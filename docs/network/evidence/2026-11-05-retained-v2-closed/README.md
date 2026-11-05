# Closed retained-debt transaction and crash rollforward

2026-11-05 UTC. **Reviewed filesystem/custody component; useful service restart and native activation remain unfinished.** Source commit [c0a10d50](https://github.com/R4-Laboratories/SpiralCore/commit/c0a10d50f3a129027fc14025bd5931808e9356e3) extends the published v2 codec checkpoint. The [local-storage contract](../../../api/installed-retained-telemetry-v2.md) retains its binary layouts and now explicitly distinguishes committed CLOSED custody from later service activation. No network protocol or canonical MVM envelope is changed.

## Implemented behavior

The transaction persists a fixed immutable transfer intent before catalog/state effects, publishes SCRETN02 and its paired owner key/record, and moves the original owner/holders exclusion, actual retained File pins and actual funded ledger into a new CLOSED authority. The original beacon, marker, ciphertext and wrapped-key bytes remain unchanged. There is no new Open supervisor, native issuer, generic reset, refund or cleanup of unknown state.

A private closed-transfer observer handles only exact supported interrupted state/catalog combinations while the ordinary owner reader remains unchanged. Resume funds the metadata/table and rolls forward the SAME target generation. Metadata funding precedes heap catalog decoding; row funding follows decoding and precedes row-file acquisition or mutation. Checksums describe state and never substitute for authority.

## Independent review and corrective regressions

The initial independent review held two P1 conditions: immutable intent publication could overwrite an unexpected destination, and resumed transactions omitted some excluded namespaces before effects. It also identified a late current-state descriptor lost across refusal. Both the HOLD and source freeze are retained.

Seven actual filesystem regressions then failed on the original code while two valid controls passed. The successor:

- Uses atomic non-replacing rename for the canonical immutable intent, including a collision injected immediately before publication; there is no replacing fallback or hard-link intermediate.
- Reuses exact namespace eligibility checks in fresh preparation, resume and immediately before mutation; unknown catalog/disposable state stays unchanged and closed.
- Arms the actual late STATE candidate in a fixed slot before fallible reads, retaining it on error and failure Drop.

The safe rustix API is verified in the exact existing 1.1.6 source. Linux execution is established; Apple's RENAME_EXCL mapping is source-only evidence until native tests execute. The only dependency change is the live crate's edge to that existing package with fs enabled. All **419 package names/versions/sources/checksums remain unchanged**.

## Executed qualification

| Final selected scope | Observed result |
|---|---|
| B2 filesystem cases | 13 pass, including the seven reviewed failures and six existing cases |
| Fixed metadata ABI envelope | One pass; overlaps the default suite |
| Default protected library suite | 172 pass, 22 ignored |
| Explicit StageA acquisitions | Five pass |
| Explicit malformed-marker and original old phase-2 cases | One pass each |
| Scoped release Clippy | Exit 0 under `--lib --tests -- -D clippy::all`; 75 ordinary messages retained |

The ordinary Clippy diagnostics comprise 74 dead_code and one unused_imports; the release build separately emits two ordinary dead-code messages. This is not zero-warning, all-target, canonical-host or native qualification. Counts overlap and must not be added.

The filesystem cases include four actual process-exit boundaries with same-target recovery, immutable intent corruption/partial/replaced-object refusal, original EX/File retention, ordinary reservation refusal, metadata retained-stop followed by generation 3 restart, and current-source v2 recovery. Phase 4 records in these cases are explicit synthetic format fixtures: they are NOT canonical native EndpointRetirement receipts or real guest transfer evidence.

All **139 supplied compiled inputs** match the source commit. Final protected executable SHA-256: `4b733f949ee531c91e5f43141d15bb5848845fdb57f9343d1099c76d0f7c9a4a`. The independent successor accepts only the exact CLOSED component and resolves the two P1s/observation gap.

## Open gates and records

Live v2 inventory integration, one-use service activation, actual holder conversion, interrupted retained-stop reopening, complete crash/fault/retry coverage, all-row capacity and actual native finality remain open. A canonical installed guest must still transfer exact data, stop through original owners and produce its real retained seal; useful restart must then retain old debt while a newly admitted guest operates. Two guests, sustained operation, fairness, supported platform/WAN and FIPS/CUI gates remain under [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42), [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43), [#49](https://github.com/R4-Laboratories/SpiralCore/issues/49) and [#83](https://github.com/R4-Laboratories/SpiralCore/issues/83).

Original source, HOLD, failed phases, package comparison and independent PASS are included. The first receipt archive is 942,724 bytes with 286 indexed records plus its index; successor 384,013 bytes with 117 records plus index. All member hashes are retained and verified. Original executables stay on Wick; no executable, payload store or VM disk is included. Managed metadata worksheets remain provisional and do not certify allocator/native/kernel/RSS memory. This component adds no new network-byte, transfer-time, energy or WAN performance result.
