# Retained-catalog v2 codec and immutable transfer intent

2026-11-05 UTC. **Reviewed pure-storage component; restart/stop I/O and native admission remain unfinished.** Source commit [8856373d](https://github.com/R4-Laboratories/SpiralCore/commit/8856373d45ade643194d130e2f115fe817315dbd), tree `9192952bd14060128f8b74719871c3225b11276c`, extends the published closed-recovery checkpoint `7e1e7fa8`. The exact [v2 local-storage contract](../../../api/installed-retained-telemetry-v2.md) defines formats and the later transaction. This does not change the SpiralCore network protocol or canonical MVM envelope/authority.

## Implementation

SCRETN02 preserves immutable origin profile/generation in each existing 4,352-byte row. The fixed total remains 16 rows across all generations, with a 69,752-byte catalog maximum. The pure inverse reconstructs original v1 bytes exactly, including a 4,472-byte catalog from the independently captured old process. SCRETX01 is a 768-byte checksummed local transfer-intent description; it carries no permission. Restart and retained-stop metadata modes enforce their distinct generation rules.

A shared nonallocating shape validator now rejects oversized/malformed private values before encoder allocation or either reverse-cloning branch. Original v1 encoding/domain/padding remain intact. No transfer-intent file I/O, new Active owner transition, live historical table, graceful retained stop or native issuer is implemented by this component.

## Executed qualification

Linux release selected library checks; no canonical-host/native backend execution is claimed here. All 138 supplied compiled-source/manifest/lock/golden inputs match the product commit exactly. The five changed/added frozen source inputs and their archived bytes match separately.

| Phase | Actual result |
|---|---|
| Missing codec API | Build 101, exactly missing-module E0432 |
| First implementation | Ten format/intent/golden tests pass; scoped Clippy 101 on one test-only Copy-array clone |
| Mechanical test fix | Ten tests pass; scoped Clippy 0 |
| Pre-clone ordering regression | Three pass/four fail at the actual clone-boundary assertions; both valid controls prove observation is nonvacuous |
| Shared validator successor | Seventeen pass/zero fail; scoped `--lib --tests -- -D clippy::all` Clippy 0 |

The final compiler emits two ordinary test-build warnings; scoped Clippy emits 52 ordinary unused/dead-code diagnostic messages. This is not zero-warning or all-target qualification. Counts overlap across phases. Clone-boundary hooks establish ordering, not an allocator/RSS measurement. Final protected executable SHA-256 747b49246ec9d6b4aae2a6b3e3862661bd79ed528c9dd577f227d83b476eac5a; original executables remain on Wick.

Independent Code/Rust/Security review initially held the component for the P2 ordering condition. The independent successor accepts the exact fix and selected causal evidence; both reports are retained. It does not establish B2 or a deployable native release.

## Resources and open work

The unchanged logical sidecar worksheet uses 19,968 bytes per row and 2,288 fixed bytes. A corrected provisional requested-Rust-storage worksheet includes a third transient raw buffer:930,152 within 1 MiB. Actual constructor/capacity/lifetime guards are still required; these numbers are not native heap/kernel/TLS/RSS proof or admitted resource grants.

Next implementation is the real debt-preserving catalog/owner transaction and finite same-target crash rollforward, moving the actual funded ledger/table and original locks. Real phase 4 ciphertext must come from canonical producer/endpoint retirement; a literal phase 4 fixture cannot enable native service. Fresh canonical plan/backend admission, original holder handoff, actual guest transfer/shutdown/retirement, two-guest failure/recovery, long sessions, supported capacity, cross-platform WAN and FIPS/CUI work remain open under [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42), [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43), [#49](https://github.com/R4-Laboratories/SpiralCore/issues/49) and [#83](https://github.com/R4-Laboratories/SpiralCore/issues/83).

## Records and storage

`raw-records.tar.gz` contains 132 original record files, 406,743 compressed bytes, SHA-256 2a29f570740e2d115e0704cf238e6a637c69c7960c34c5ab7b04e4229b2ef6f0. Every member length/hash matches its manifest. Sources, failures, commands, before/after maps and ordinary diagnostics are retained. Protected executables are excluded; `packaging-note.json` records their removal from the initial temporary collection after compact verification. No payload, VM disk or build target is included.

No networking/performance benchmark ran for this component. It adds no new bytes/CPU/WAN-speed/energy result. Historical transfer measurements keep their own source identities. This packet and source branch are development checkpoints; main activation and complete MVM delivery remain open.
