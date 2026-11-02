# Independent candidate CPU review

Reviewer: `mvm_tests`. Date: 2026-11-02. Verdict: **PASS; no open material finding.**
The reviewer authored none of the reviewed implementation or test changes.
Scope is the three production files and two test fixtures below, against baseline
`4b8cb79fe5fca9e9527a2923924c887d9e7ddbf8` and the committed candidate-CPU design.
This was bounded Rust/code/security review with no additional builds or tests.

| File | Reviewed SHA-256 |
|---|---|
| `crates/spiralcore-transfer/src/schema_selection.rs` | `824da6db84ff52ce8a43b5aeb062f4e3c30c24443f129d7c7d94903621f5b8ab` |
| `crates/spiralcore-transfer/src/schema_zstd.rs` | `d023c09664c86fdf960d100e74508990902382ef104e409de9e76c92a85b2398` |
| `crates/spiralcore-transfer/src/shared_transfer.rs` | `469ff639c60a2b1388d1a0b868bb1581aa8bb8d9fd3c83146fcb11772e40b8c0` |
| `crates/spiralcore-transfer/tests/fixtures/candidate_before.rs` | `19615d46cf5856c085aa81b9189820b5026b127790e0b26396e5562348555d8a` |
| `crates/spiralcore-transfer/tests/fixtures/candidate_cpu_checks.rs` | `a83a8fc2b028fa98446fbab5f2a9ca957d84a7e2a75e9b58df5bdcbdd7f0fde0` |

## Production conclusions

- The new schema certificate acts on the exact canonical intermediate. Its
  private fields prevent constructing an arbitrary intermediate/target-length
  pair outside the codec module. The selector obtains and consumes it through
  the same prepared schema encoder. If the certificate is inconclusive, the
  unchanged `compress2` profile receives those same intermediate bytes once.
- Both schema and Whole contexts use the unchanged `new_encoder` parameters,
  with no dictionary or prefix. Reusing Whole's existing session-resetting probe
  adds no third context. The 16 MiB proof-domain check precedes the Whole helper,
  so a valid larger schema intermediate falls back rather than failing its
  unrelated whole-object limit. The existing bundled-source/version and frame
  shape guards remain authoritative; no streaming-prefix heuristic was added.
- Encoded-length bounds are converted using the existing framing cost and
  checked setup addition. The original Raw/Whole/Sparse/Runtime/Schema tie
  comparison decides pruning. Samples can only suppress a probe and cause full
  encoding; they cannot prove exclusion. Inconclusive certificates preserve the
  full encoder, while actual codec/reset/resource errors propagate.
- Residual sharing requires complete equality of the rolling bytes and immutable
  prediction. The single residual's `None`/error behavior is preserved. Runtime's
  exact cost still uses the existing checked cost function over its actual setup
  length plus residual frame lengths. Sparse strictly dominates that identical
  residual with the extra Runtime setup, so omitting a duplicate Runtime payload
  cannot alter the final winner. No unchecked constant cost or digest shortcut
  replaces validation.
- Existing encoder poison checks occur before the new schema prune. Bounds and
  intermediate buffers remain transient within the existing admission model.
  No session promotion, namespace, receiver decoder, ACK, identity, wire mode, or
  Whole control implementation changed. This is code-level admission review, not
  a measured process-memory qualification.

## Frozen baseline and validation

The reviewer compared the fixture with the pinned Git blob: all **16,795 bytes**
of its original non-test selector prefix are identical. Only provenance comments
and the appended test witness wrapper differ. The witness compares encoded Offer
bytes and every Data body, not merely mode, output length, digest, or accounting.

One validation gap was identified and closed during review: the frozen selector
still calls the current public schema encoder, so that alone could change both
sides of a comparison. The final test's independent `original_schema_frame`
uses canonical `schema::encode` plus a fresh unchanged `new_encoder/compress2`.
It avoids both new intermediate methods and checks their exact output against
the old operation. Receiver round-trip checks remain separate.

The reviewer inspected the final retained `core-final-green.txt` and exit `0`:
all **three focused tests passed**. They cover both probe/readiness settings,
generated and edited outputs, Raw/Schema fallbacks, equal and unequal bases,
append/truncate/empty inputs, the 128 KiB probe boundary, valid schema expansion
over 16 MiB, lower-bound soundness against full encoding, ties, context reuse,
rejected input, and exact baseline Offer/Data equality. These finite fixtures
support the bounded proof reuse; they are not exhaustive verification of all
possible input strings.

The corrected existing regression run `core-regressions-02.txt` exited `0`,
including the Combined trust-state negatives. The earlier exit `101` remains
retained as the reported test-target invocation error; it is not presented as a
production failure or silently removed. Production performance claims await the
matched before/after campaign. No network result is inferred from these tests.
