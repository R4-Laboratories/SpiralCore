# Finer scan diagnostic: frozen-build recipe

The design and limits are in DESIGN.md; the parent registration is9c9650d. All source
is isolated under this owned directory and pinned to d03fe16, hardware SHA on. Only
endpoint instrumentation/module export and the new fixed-size scan_profile module
modify the source snapshot. No production source, authority, catalog or quota changes.

Build and template preparation are allowed separately from performance execution:

1. Freeze `snapshot-established.sha256` after source qualification/review.
2. Run `bash build.sh` once; preserve source/helper/feature/tool/environment/executable
   manifests. The reused guarded driver pins native compiler/target and rejects flags.
3. Run `build-diagnostic/receiver-probe prepare --root ABS_ROOT/experiment` once.
   This creates all four closed templates through the public Endpoint API, compares
   their bytes and synchronizes every publication/directory. Setup costs are separate.
4. Run `bash template-manifest.sh > templates-established.sha256`; preserve all
   1612 template file hashes (1604 publications plus8 endpoint markers/owners).
5. Bind actual tests, builds, source, review and templates with
   `build-diagnostic/profile-analysis --qualify ABS_ROOT ABS_ROOT/qualification.json`.
   Preserve the resulting qualified-input manifest before performance authorization.
6. Only after the parent quiet signal, run `bash run-profile.sh` exactly once. It refuses
   an existing measurement directory and stops on any failed observation/process, with
   evidence retained. There are nine fixed processes, eight observations each.

Fine clocks measure six original source expressions, not pure syscalls/allocator events.
Enumeration includes construction of the namespace d.path() argument. The unspanned
scan residual contains bookkeeping and iterator/entry destruction. Counter-only keeps
the same expression wrappers/counts without fine clocks. Empty mode measures a black-box
empty span using the same number of CPU/wall clock pairs, then performs the real expression.
All instrumentation overhead remains visible; do not subtract it as an exact correction.
Span, coarse-scan, handler and operation clocks are nested, never additive across levels.

The native Rust analyzer validates all72 source/executable/fixture/sequence/inventory/
readiness/frame/clock records,144 scans,57,744 metadata API calls and the expected byte
count. It retains all three values per cell, medians/min/max, and block-paired control
ratios. Screening results are only leads, never acceptance of a production candidate.
Independent review is required. Exact allocator/syscall counts and energy remain
unmeasured; invalidUTF8 actual filesystem entry qualification remains NOT_RUN.

The analyzer caps text inputs at16MiB, streams hashes in64KiB chunks up to128MiB,
and rejects integer-overflow/underflow in supplied frame/span/stage/campaign totals.
The first release build predating these analyzer checks is retained as qualification
attempt1 and is never a measured executable. The measured probe source is unchanged.

Useful nonempty qualification outputs are retained through review and binding. After
analysis, owned diagnostic logs may be retired only after preserving structured counts,
test identities/commands/failures and hashes, without rewriting historical bindings.
