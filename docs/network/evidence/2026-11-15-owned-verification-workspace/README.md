# Owned publication verification scratch space

Source candidate: SpiralCore [PR #294](https://github.com/R4-Laboratories/SpiralCore/pull/294), db9241e97db46ac260868f40026aeb579248d259, tree 8df5f59f8c71c9b0c729a2f4295f67c0ec17f992, based on 8a36fd282c3fa8a5c9696bc16701e8ecc9f4b1fc. This source changes one shared private verifier: same canonical object-hash domain and little-endian length, fixed 8,192-byte stack scratch, exact body/EOF/digest checks. Original descriptor custody, full preflight and writer-lock lifetime remain. Byte-returning reads remain. No dependency, wire-format, authority, budget, timeout or security-check changes.

Merged release: 6d195a0cb282884b296d6bd433549e3a83eef527, with the same exact reviewed/tested tree.

## Scope and qualification

The new isolated filesystem fixture manually populates two private stores, each with 64 maximum 64 KiB publication-format bodies, 32 object stages, one profile stage and two enrolled namespaces (101 entries). It verifies 8,394,880 exact record bytes during setup. It is not application/network/MVM admission or traffic. Original directory buffers predate the allocation window. Start barriers allow concurrency without proving a deterministic simultaneous peak.

The requested Rust allocator measures every new process request while the retirement window is open, including coordination, assertions and fixture destruction, without subtracting pre-window frees. It does not measure direct libc allocation, allocator metadata/fragmentation, native stacks or RSS. Cumulative requests do not prove the old implementation violated a live memory budget. The existing 1 MiB/store quote and original aggregate permits remain unqualified by this sample alone.

RED source archive and original log retain largest 65,536 / cumulative 16,889,866 newly requested bytes and the expected original guard failure. The initial RED executable's independent hash was not captured before the later rebuild; do not claim that stronger binding. The initial clean source archive and compiler/test log remain evidence of the observed failure. Final candidate direct selected executable 0421ea1dcfd12511d9dd1f09486d2d5784803762c8aaf9741400c5da3d836419 passes one exact test and retains its hash afterward. Final requested payload is largest 250 / cumulative 112,650 bytes; full record readbacks remain exact.

Final direct witness setup: 21.684923 ms wall / 7.519226 ms process CPU. Each retirement timer covers owner-probe opening through return from retire_contents: 589.145479 / 591.848788 ms wall, 23.062260 / 22.539361 ms thread CPU. The distinct window timer covers worker release through all joins and includes assertions/destruction: 591.907039 ms wall / 45.753445 ms process CPU. The separate parent-and-child whole command reports 0.61 s wall / 0.01 s user / 0.03 s system CPU at the coarse precision printed by GNU time; this is not a per-phase or per-side clock. These are observed instrumented single-run lifecycle costs, not a matched timing benchmark or energy measurement.

Complete Wick source: 215 input files, all hashed before/after and checked against the committed checkout. Release offline locked full transfer: 530 passed / four pre-existing ignored across 67 result groups; one doctest passed; strict all-target Clippy passed. Release FlowMux all targets: 87 passed, zero ignored across 12 groups. Sustained row verifies 67,108,864 application bytes each direction, actual TLS 5,395,050 sent / 5,395,015 received at endpoint A with reciprocal peer counters. Setup 22.184 ms / lifecycle 43.626103 s / whole-process lifecycle CPU 4.762010 s; original 240 s gate unchanged. Separate sender/receiver process CPU is not measured in this cooperating host-process row. No new booted MVM, WAN, supported-capacity or FIPS/CUI/security acceptance.

Wick toolchain 1.97.1 / shared Cargo target / jobs 2 / incremental 0. Active receipts show 6 GiB memory, zero swap, 200% CPU; terminal journal records corrected complete run exit 0. Missing compile-time fixtures and later workflow-runtime files were export omissions; original failures are retained separately. Five existing compile fixtures and eight existing workflow files were added unchanged, without assertion, deadline, lint or security relaxation.

Independent source/security/workflow and Wick evidence reviews found no actionable issues. All 27 executed PR checks pass, with four manual jobs skipped/NOT_RUN. Seven native rows pass 87 FlowMux tests and 20 ordered/Small/schema regressions each, retaining exact 64 MiB per direction, reciprocal actual TLS counters and original 2 s/240 s guards. All seven original ZIP digests, 222 member files and 215 embedded checksums verify. The four native Unix filesystem witnesses have 250-byte largest requests and 109,824–112,532 cumulative requested bytes, with 8,394,880 exact record bytes each. Windows retains its four existing metadata guards.

The tested CI merge 2a9db659ec25f8f085a87e9fd55a56848e852409 has the same tree as the reviewed and released source; the public commit/tree/parent receipt is retained. Final independent native artifact review found no findings and recommended the exact merge. It checked API sizes/head/run, CRC/safe paths, full checksum coverage, raw counts and rows, source/executable receipts and Unix/Windows lock bytes. Native executable binaries are not in the ZIPs: the review verifies runner-recorded unchanged hashes and compiler/test provenance, not a direct rehash of downloaded executables. #258/#42/#49 remain open. Ari's current path and signed-runtime/private-MVMD gates are recorded in coordination.md.

## Inputs

- final-inputs.sha256: initial 207-file final export before eight unchanged runtime-workflow inputs were supplied.
- complete-inputs.sha256: complete 215-file export used by the successful whole-suite run.
- wick/: original RED/GREEN/final/incomplete/complete raw logs, actual active-limit receipts, compiler JSON and selected executable hashes.
- failure-journal.md: corrections and evidence limitations.
- *.sh: exact bounded execution commands.
- coordination.md: current upstream ownership/path refresh and limitations.
- native/: seven original ZIPs, API metadata and validated raw measurement rows. [Native table](native-results.md).
- ci-current.json / ci-merge-commit.json: terminal check snapshot and exact public tree/parent receipt.
- review.md: independent source, Wick and native artifact review scopes/results.

The sources/ directory retains three compact archives: clean RED source (SHA-256 0b2f2d9b9b8b976511092fbb541ce0e982d3ebd05afb7c31f3fe6513a205ad65), final 207-input export (ef922b02bfc1976836280a39d9ebd91d26e7ec6665bfb0190b97d8b0588d4492) and the eight unchanged runtime-workflow inputs. The final archive alone is not the complete 215-input export; both it and the supplement reproduce the corrected input set. No source archive or CI badge alone establishes full MVM delivery.
