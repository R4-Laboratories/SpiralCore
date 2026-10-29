# SC-OPT-003: same-source SHA backend diagnostic

Registered plan: repository commit f6a01442e9a0a6d4e1ce2cc4a9698409fe7258a2.
Candidate production source: 80449939bd92348593cc88555cf7fde0847ceffd.
No production Rust change is permitted. The measured probe `src/lib.rs` and
`src/main.rs` are byte-identical to SC-OPT-002. Both configurations use the same
manifest and expanded lock. The only active build feature difference is the
default-off probe alias `hardware-sha256`, forwarding to the production feature
`spiralcore-transfer/hardware-sha256` and thus `sha2/asm`.

The unchanged probe supports receipt variants old/new. **old means software;
new means hardware.** Paths, driver events and build records use software/hardware.
Both variants are newly built from the same candidate, not reused old code.
The optional backend is locally qualified only on aarch64-apple-darwin; upstream
sha2-asm rejects Windows. There are no target-cpu=native or global target-feature
overrides. Build guards reject uncontrolled flags, wrappers and Cargo config.
CC/CXX/AR and Rust compiler paths are explicit, with resolved Apple tool hashes,
package version, feature metadata, compiler identity and environment preserved.

`verify-inputs.sh` checks unchanged measured Rust, the exact feature-only probe
manifest difference, and the sole allowed lock delta: sha2-asm0.6.4 and its sha2
edge to existing cc. The production sources must match both the candidate and
the reverted software baseline. Both builds must have identical input manifests.

`schedule.json` fixes the same 16 cells and eight AB/BA blocks. All three CPU
gates must pass: N0 raw normal >=1.10x; N0 sparse normal >=1.50x; N768 sparse normal
>=1.20x; each needs >=6/8 hardware-faster pairs. Primary metric remains
local_operation_total.cpu_ns, including synchronous frame/recipe preparation,
Endpoint handling and bounded instrumentation through Published and request-buffer
teardown. Handler/stage CPU is a subset. Setup/open/reopen costs are separate.
All results, failures, wall times and control cells remain; no observation filtering.
This is not a network, WAN, energy, FIPS or unsupported-target qualification.

The existing closed SC-OPT-002 templates are reused read-only at
`../receiver-efficiency-20261029/experiment/templates`. Every observation uses
the reviewed helper's new synchronized private clone. `template-manifest.sh`
pins the closed template bytes before and after the new campaign. The previous
source, lock, binaries, raw receipts and templates remain unchanged. Shared
working-copy directory must be empty at campaign start; no other campaign may
run against it concurrently. Outputs are new exclusive paths under this directory.

`src/bin/identity.rs` is a separate non-timed exporter: 34 cases matching the
canonical-oracle fixture recipe (empty/trailing zeros/leaf and ternary boundaries/
random/structured) plus the actual receiver base and target, for 36 total.
It records exact manifest bytes, object/manifest IDs and node fingerprints, and
writes complete sorted length-framed node encodings. `qualify-identity.sh` compares
the complete off/on files byte-for-byte and checks the independently pinned receiver
object IDs. It also checks that an existing output directory is rejected without
changing its files and a relative output path is rejected without being created.
Large complete node files stay local; compact identity rows and file hashes can
be committed. Known SHA padding/multiblock vectors and canonical legacy oracle
tests are qualified separately by the parent under both configurations.

Exact commands (Bash is intentional; zsh reserves the variable name `path`):

```sh
bash verify-inputs.sh
bash build-variant.sh software
bash build-variant.sh hardware
bash qualify-identity.sh
# Only after helper tests, build/driver review and explicit parent go:
bash run-paired.sh
```

`qualification.json` is written only after actual probe tests in both configurations
and the identity checks pass. The run driver requires it and refuses to overwrite
existing campaign output. Each process exit and all raw observations are retained.
`setup-findings.json` records pre-build lock resolution and tool-version issues;
none were timed observations.

## Repository packet at qualification checkpoint

Status at this checkpoint: functional qualification passed; timing NOT_RUN. The
plan was committed before implementation. Both frozen executables, the independent
identity review, and the fixed schedule are qualified before the paired campaign.

`transfer-validation.json` records 125 transfer tests and one doctest under each
configuration, fmt and strict clippy. `imported-store/` records 71 tests and the
save/restart example under each configuration, exact source/fixture checks, and
the separate qualification-only manifest alias. `probe-test-results.json` retains
every measured-helper test outcome; `qualification.json` binds those checks to
specific executables and scripts. `independent-qualification-review.json` and
`review-method.md` provide the separate identity/feature/provenance review.

Complete node files and frozen executables stay in the named local experiment
directory, identified by committed SHA256 manifests. The compact identity rows,
feature-resolution metadata, source and build commands are included here. Build
and test console diagnostics are represented by structured results and original
output hashes; `.log` files are excluded from this repository packet. Their
original local paths in qualification bindings describe the completed check, not
additional committed files. Scientific measurement JSONL will be preserved in full.

The helper scripts retain the original host paths and native target. Reproduction
on another checkout requires explicit path adaptation and new source/build
receipts; it must not silently reuse the identities of these frozen executables.
