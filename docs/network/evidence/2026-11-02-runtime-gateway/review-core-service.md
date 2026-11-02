# Independent core and object-service review

Reviewer: `mvm_tests`, acting as active code-reviewer, security-reviewer and
rust-reviewer. The roster checks passed. This review excludes this reviewer's
transport/object-channel source and tests. No new builds or benchmarks were run
for the review; existing execution receipts were inspected.

## Shared facade: PASS

No material findings in the changes to `spiralcore-gateway-core/src/object.rs`
or `tests/object_runtime.rs`, authored by `mvm_boundary`.

- Runtime construction moves one immutable `PreparedPrediction` into the existing
  `RuntimeSession`; generic construction cannot silently substitute a prediction.
- Every context-bearing operation checks the namespace. A mismatch or malformed
  context drops `prepared`, preventing subsequent reuse even with the old context.
- Checkpoint publication is explicit and verified by the existing transfer
  client. Missing/evicted receiver state is not concealed by automatic reseeding.
- Warm and Runtime receipts forward the underlying policy results. The facade
  does not fabricate candidates or claim a combined optimum. Fixed profiles
  return an explicitly fixed-policy receipt. Candidate byte costs keep their
  existing HTTP-body accounting label.
- `retained_charge` delegates to existing prepared/session accounting and becomes
  zero after permanent invalidation. It is documented as retained quota, not RSS,
  peak work, or an independent allowance for every concurrent sender.
- The added tests exercise real RuntimeV1 output and residuals, exact store
  reopen, missing checkpoint/receiver invalidation, permanent epoch rejection,
  real warm SparseV1 selection and repeated-object acknowledgment. Existing
  retained evidence shows runtime tests 3/3 and all core tests 12/12 passing.

Reviewed identities:

| File | SHA-256 |
|---|---|
| `crates/spiralcore-gateway-core/src/object.rs` | `e14762163e6b7dfb6f0ea2dfa22efd7e06fd83947d1025a6d3a8b3de9384f171` |
| `crates/spiralcore-gateway-core/tests/object_runtime.rs` | `f90f270e4ecba1d3f575d25fc2abd42b822f4dbc0ce911697b38a2043a63c704` |

## Service namespace and receipt correction: PASS

The released namespace-binding correction was independently inspected at
`src/object_service.rs` SHA-256
`52824d56092d6af79682d58d71f41341261bf31fdbe17c95f9a9f42393e48d21` and
`tests/object_service.rs` SHA-256
`a915f7e524842a4689b93c06a0ca09d767023b8f069de104e4d9a6460267ee70`.
This review excludes the earlier one-line schema-cap correction made by this
reviewer at the root's request.

The initial control batch must equal the local namespace binding exactly,
including kind, transfer ID, sequence, body and frame count. It runs after pinned
authentication and before `Endpoint::open`; neither endpoint accepts a scope
chosen by the payload. The initiator checks the exact echo. Scope/epoch mismatch
tests use real QUIC and require both endpoint failures with no store creation.
The retained `service-scope-red.*` shows the previous acceptance failure, and
`service-scope-green.*` shows all three service tests passing after the fix.

`publication_path` is recorded only after the existing transfer client validates
correlation and publication identity. It detects Begin-to-Published replies,
including cache hits after a whole outgoing batch was already sent. Offered mode
and actually sent bytes remain separately recorded, so this does not turn a
cache hit into a false claim that the batch consumed zero network bytes.

The service retains one prepared sender at a time, bounds groups/operations and
received batch/publication counts, and uses the existing endpoint limits. It
checks reopened object identities and optional exact expected files before its
end acknowledgment. CPU/wall intervals include sender input loading, selection
and verified reply; receiver batch timing explicitly excludes stream wait and
SCWBAT framing decode while whole-process totals include them. Report construction
is deferred until the session finishes. Handshake cancellation is now explicit.

## Remaining finding: output-path ownership (P2)

At the reviewed source, `Config::validate` line75 only rejects equality between
`stats_path` and `ready_path`. Both paths are later written through `write_json`,
whose final rename replaces its target. A configuration with `ready_path` equal
to an input or private key can therefore destroy that file when readiness is
written; using `stats_path` does so on completion or error handling. Configured
campaign output paths are distinct, so this does not invalidate that registration,
but it is a preservation defect in the usable file/object interface.

Reported to the root with a recommendation to establish fresh output ownership
and reject canonical input/credential/store aliases before any output write.
No source edit or extra build was performed during the independent review.
Subsequently, the root assigned this reviewer to implement the correction. The
observed preservation RED/GREEN and final source identities are retained in
`service-output-qualification.md`; independent closure of that newly authored
correction belongs to `mvm_boundary`, not this reviewer. The namespace-binding
correction and core facade verdicts above remain independently reviewed.
