# Combined core and gateway review

Reviewer: `mvm_tests`, independent of the reviewed production changes.
Date: 2026-11-02. Verdict: **PASS; no material finding in this bounded change.**

Scope is the Combined additions and their effect on existing paths in the five
files below. This is code/security/Rust review, not a new whole-repository audit.
The reviewer authored the two gateway `object_combined.rs` test files and does
not independently approve those tests. Previously authored, unchanged transport
and output-preservation code is not included in this independent verdict.

| Reviewed file | SHA-256 |
|---|---|
| `crates/spiralcore-transfer/src/session.rs` | `6e0afa722147c5eb0165a352f3100c7832bc9e1fe01787722002087c21120e0b` |
| `crates/spiralcore-transfer/src/schema_selection.rs` | `b068e920fb1864adaf686908282b07886f32265ad2eaf576431076c59d1522b9` |
| `crates/spiralcore-transfer/tests/combined_session.rs` | `a70724b5f9037206158532db48affd0e60d4ac60da886c865eedabfd55342c3a` |
| `crates/spiralcore-gateway-core/src/object.rs` | `5f6adb80d2937569d86855ac1f353f6d2b8e491252e0de2b5da7d8ccdc240db9` |
| `crates/spiralcore-gateway/src/object_service.rs` | `a9107e56ffe5b992884fffff5bd98cf3db5657b94c30e7c4e4d8650a74fb48d5` |

## Correctness and trust boundary

- Runtime offers bind the immutable checkpoint identity, Sparse offers bind the
  last acknowledged rolling target, and Schema offers bind the artifact. Raw and
  Whole carry no base. Auxiliary seed acknowledgments cannot replace the rolling
  target. Shared candidate construction retains only an incumbent and the current
  candidate, with the existing deterministic byte-cost tie order extended by
  Runtime between Sparse and Schema.
- Repeat admission requires full equality with private, immutable acknowledged
  bytes. The cached identity remains private. The existing client still validates
  a current Published reply; receiver eviction takes the same Raw offer's upload
  path. No digest-only authorization or hidden repair/reseed was introduced.
- Owned input is promoted only after successful client validation. A malformed
  Published reply after an actual remote commit leaves the old rolling identity
  and invalidates all readiness. Preflight capacity failure preserves state;
  attempted exchange failure clears readiness; namespace/epoch mismatch drops
  prepared state permanently. Unseeded schemas are ineligible, and changed sends
  require an acknowledged checkpoint.
- The gateway facade delegates these rules without synthesizing readiness. The
  service requires both auxiliary artifacts for Combined, rejects generic target
  seed, accounts for checkpoint and schema lengths separately, and retains the
  existing authenticated namespace binding and exact reopened-store verification.
  No wire parser, decoder, crypto, QUIC profile, or publication rule changed.

## Bounds and measurement limits

The 128 MiB scratch allowance is appropriate for the reviewed sequential
candidate lifetimes: no replay or extra Prefix context is allocated during send;
the larger schema path's intermediate/compressed/framed/incumbent buffers remain
below the stated allowance, with prediction, rolling capacity, input capacity,
and both conservative codec allowances charged separately. This is admission
accounting, not measured process RSS or a shared multi-session allocator. Outer
callers retain shared-budget responsibility.

The repeat shortcut avoids new target hashing inside CombinedSession. The
gateway deliberately retains its common independent input hash, file read, and
verified acknowledgment; this is not a claim of globally zero hashing or compute.
Selection receipts explicitly retain the historical HTTP-body cost model. They
are neither actual QUIC/IP bytes nor a guarantee of the minimum among all possible
codecs. Runtime recipes remain fixed and explicitly prepared; no global recipe
search is claimed.

## Validation observed

- Author's retained `core-green-01.txt`/`.exit`: five focused tests pass, covering
  live endpoint publication, fixed/rolling identity, exact-repeat eviction,
  corrupt acknowledgment, preflight capacity, permanent epoch invalidation,
  missing checkpoint/rolling base, and full 16 MiB admission with transport failure.
- Author's retained `core-regression.txt`/`.exit`: existing affected tests pass;
  reviewer inspected the successful receipt and relevant negative test bodies.
- Reviewer executed `facade-green-01.txt`/`.exit`: one focused facade test passes,
  including six mixed sends, auxiliary reseeding without rolling replacement,
  repeat frame counts, later fixed-checkpoint recovery, and exact reopened bytes.
- Reviewer executed `service-green-01.txt`/`.exit`: two tests pass, including
  configuration negatives and one real pinned-mTLS QUIC session with checkpoint,
  schema, Runtime, Sparse, repeat acknowledgments, and exact reopened fixtures.

The corresponding initial facade and service RED receipts are retained beside
these results. No additional benchmark or broad test expansion was performed by
this reviewer. Network savings, time, CPU, and campaign acceptance remain for the
registered measurements; these checks alone establish no performance improvement.

Test authorship/receipt identity (these tests require another author's review):

| Reviewer-authored test | SHA-256 |
|---|---|
| `crates/spiralcore-gateway-core/tests/object_combined.rs` | `27033cb7728c9c4fc7e874e9e874dfb281d9ded26b7ae1ac0e9c33c38263200b` |
| `crates/spiralcore-gateway/tests/object_combined.rs` | `0be686b0418a1391ef0a23202ff21376da7f0586e73f4299296af738b841fdb9` |

Root separately reported reviewing these two tests without a blocker. No changes
were made after their observed GREEN runs.
