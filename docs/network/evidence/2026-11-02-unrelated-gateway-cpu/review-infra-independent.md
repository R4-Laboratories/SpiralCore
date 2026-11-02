# Independent infrastructure delta review

Reviewer: `mvm_tests`, independent of these changes. Date: 2026-11-02.
Verdict: **PASS; no blocker.** Scope is the four-file infrastructure delta and
new registration. No builds, tests, source edits, or additional audit performed.

| Reviewed file | SHA-256 |
|---|---|
| `.github/workflows/spiralcore-candidate-cpu.yml` | `649ad4eb51d6241a02714caac8dc8df3e6a0e80cc809adc40e5b394c4792b13c` |
| `tools/gateway-network/runtime-object-netns.sh` | `e699d92c61246ea077b1a050f80410e52044bcd1451f46b824d420206e4adb8f` |
| `tools/gateway-network/runtime-object-infra-check.sh` | `a5c04a7d08a7d9f0df3d412cb1393606ff2d7814cf2a511eb82fc275e29697ec` |
| `tools/gateway-network/analyze-runtime-objects.rs` | `61abbf98ddd256a54613c74a1af2c3d83e5116fd63e141b87a7dabadf4ec57df` |
| `network-registration.json` at review, launch still gated | `b04484183554c3312310fedb7abc3a74abd3f82fb8141811425f9105465c0cf9` |

The workflow statically pins baseline checkout and its expected revision to
`4a8ec79c9b185c85407e9767653dbcc4af97bf7e`, and all registration/source-manifest/
launch paths point to this milestone. Existing baseline-first build, executable
freeze, Whole-control equality, compiler settings, permissions and credential
handling are unchanged.

Harness and analyzer explicitly map the new unrelated-CPU schema to `4a8ec79`.
The older candidate-CPU schema remains pinned to `4b8cb79`; neither accepts the
other's baseline. Binary binding now reads the already strictly validated pin,
rather than silently hard-coding the older revision. The new negative checks
exercise both swapped pins. Shared paired-policy helpers preserve the existing
18-session, 264-operation, 216-delivery dose and 1,717,047,288 requested bytes.

No transport, fixture generation, namespace routing, key handling, cleanup,
before/after logical parity, receiver attribution, setup accounting, or arithmetic
formula changed. The historical Runtime and Combined policies remain separate.
Retained historical Runtime, Combined and candidate analysis receipts each report
byte-identical reproduction with exit zero; analyzer tests and Clippy also exited
zero. These receipts were inspected, not rerun by this reviewer.

Only registration lifecycle fields remain for root's launch decision; the reviewed
pin, dose and policy are fixed. Actual network execution, measured gains/losses,
artifact custody and final arithmetic qualification remain subsequent steps.
