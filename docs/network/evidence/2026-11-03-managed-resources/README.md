# Installation-managed parent reservations

This is the accounting prerequisite for the canonical installed launcher,
tracked under #43/#49. It is separate from the previously frozen installed
supervisor/native-transfer source and does not establish integrated endpoint
allocation, booted capacity, whole-process RSS, transfer speed or WAN acceptance.

The original `InstalledSupervisor` creates one pool under its existing exclusion
owner. Clones share its host and stable-tenant accounting. Claims separate host
plaintext/codec/carrier/metadata/stack, guest RAM, cache, staging and workers.
Admission is atomic across those domains; rejected claims do not partially spend
capacity. The contract is [installed-managed-resources-v1](../../../api/installed-managed-resources-v1.md).

Actual qualification:

- Linux standalone release: 18 unit entries, one native-death *component* entry,
  three lifecycle entries and eleven resource entries, all pass (33 total).
- Apple Silicon macOS standalone release: the same 18 + three + eleven pass
  (32 total); the Linux-only death entry is not compiled. No guest boots here.
- The full Linux live crate passes all 143 entries across twelve targets,
  including two targets with zero tests. Counts overlap with the standalone
  suite. All-target production release Clippy passes with `-D warnings`.
- Eight concurrent 100 MiB accounting requests admit exactly five while held,
  observe 500 MiB in the same pool, and return to zero after join. This does
  **not** allocate 500 MiB or qualify eight running MVMs.
- Corrupt slot serial, out-of-range/missing tenant, underflow and mutex poison
  refuse new work and retirement, preserve exclusion and never refund as
  success. Last-owner attachment checks the charge during the value's Drop.
  Changed epoch storage closes sibling admission. The active-slot and stable
  tenant tables remain bounded at 128 and 32.

The first missing-API compile failure and the real premature-retirement failure
precede implementation. Subsequent regressions exposed unchecked corrupt-tenant
indexing and missing sibling fencing after lease epoch validation. The stronger
corrupt-current test also produced a double-panic abort before the fix; that
failure remains in `validation/`. A product Clippy attempt initially lacked the
matching module declaration in the copied controlled source; the exact copied
lib hash and passing rerun are retained. No assertion or resource limit was
weakened to make these gates pass.

Coverage uses protected executables, before/after source identities and actual
LLVM profiles on Wick. New-module production lines 1–374, excluding inline tests:
**221/233 mapped lines (94.85%)**. The entire harness export is **77.64%**, includes
unchanged legacy code and test sources, and is not a project-wide 80% claim.
Branch coverage is unavailable. LLVM reports two mismatched functions when
combining normal/test variants; original output is retained. The derivation,
LCOV, report, source/binary digests and scripts are in `coverage/`.

The final real lease bridge must bind the actual opaque MVM admission, native
anchor, original pool/slot/serial and private child configuration before
activation. A generic retained object, byte-count DTO or independent per-process
quota is insufficient. Those integration gates remain open.

The first hosted supervisor matrix incorrectly included Intel macOS. That row
failed compilation because installed supervisor helpers intentionally exist only
for Linux host builds and Apple Silicon macOS; it did not execute tests. Linux
x64/ARM64 and Apple Silicon rows passed. The corrected supervisor matrix retains
those three roles without changing any production platform guard. The separate
four-row Unix installation-owner component remains unchanged. Intel macOS
client/component support does not imply a native Intel macOS MVM supervisor.
The original failure and the review correction are retained explicitly.
