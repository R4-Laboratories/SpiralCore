# Coupled shared-runtime implementation and evidence

This packet records the existing ternary/Kuramoto/cellular-automaton runtime's
Rust port, its verified checkpoint/recipe/residual transfer integration, and
actual local and WAN measurements. Read the
[result and limitations](../../shared-runtime-results-2026-10-30.md) first.

## Identity and scope

| Item | Identity |
|---|---|
| Runtime/harness/local source | `b6627deb7fb8c80a46c0e89d5e074afaf50f37f0` |
| Successful WAN source | `35114daef384186dacd6123221ec9b5db60b3fcc` |
| Native measurement executable | `49c0aa577105718fd4a7019ee719d1815e2b1f16e6d27e909632ecb228386d93` |
| Native receiver executable | `7b79c143c6e010804ed9a6d990c3333485344607e150d7200965a51f5eba1162` |
| Successful GitHub run | `41062532491`, first attempt |
| Immutable artifact | `12982310131`, archive SHA-256 `cd3918063453ef3c81a7dec063711265f50327aa02b5b66d4bfe4bfc085a1648` |

The native binaries are reused for the WAN pilot with an empty Git diff for all
Rust crate/reference inputs between the two pins. Only the workflow and its
validation/review records change. The Linux sender is freshly built from the
successful WAN pin. Its binary hash and full compiler/feature/source records are
in `wan/attempt2/remote/` and the independently checked artifact custody.

## Packet map

- `measurement-contract.md`: fixed workloads, dose, order, budgets, timing and
  byte scopes, controls, seed accounting and WAN extension, written before runs.
- `reference/`: pinned Python source excerpts/vectors, exact Rust compatibility
  vectors, actual RED/GREEN receipts, independent engine review and checksums.
  Re-extracting from the original Python wrapper needs the preserved local
  sources. Rust/vector tests are available directly from the checkout.
- `qualification/`: native release tests/build/checks/coverage, harness tests,
  workflow gates and actual failures. Original stdout formatting is preserved.
- `pre-measurement-review.json`, `infra-review.json`,
  `infra-context-fix-review.json`: separate source and infrastructure reviews.
- `custody/`: frozen source and native executable identities. The source list is
  relative to the **local source pin**, so verify it against that Git revision
  rather than a later changed workflow file.
- `local/`: two plans, original JSONL observations, command exits, timing window
  and executable checks. Exactly 480 operations are retained. This directory is
  intentionally committed despite the repository's general `**/local/` ignore
  rule; it contains public synthetic experiment evidence, not private caches.
- `independent-analysis/`: separate Rust validation, 17 mutated-negative cases,
  exact custody, all local metrics/scenarios, independent jq arithmetic and seal.
- `wan/`: failed pre-execution launch metadata, original receiver plan, launch
  variables and cleanup observations. The rejected first launch has zero jobs
  and produces no WAN measurement.
- `wan/attempt2/`: the successful run's API/job/artifact metadata, byte-identical
  native/remote plans, downloaded remote receipts and 72 exact stopped-receiver
  verification rows. `remote/receiver-verification-pending.txt` is the immutable
  sender-side statement at upload time; the subsequent local verification
  resolves it and is retained separately.
- `wan-analysis/`: independent sender/receiver joins, source and archive custody,
  27 malformed-data rejections, all WAN metrics/scenarios, jq cross-check and seal.
- `pipeline-audit.json` and `post-audit-review-*.json`: the completed procedural
  audit and independent final source/security reviews, with authorship boundaries.
- `SHA256SUMS`: the final packet seal. From the repository root, run
  `shasum -a 256 -c docs/network/evidence/2026-10-30-shared-runtime/SHA256SUMS`.

## Measurement conclusions

The registered local campaigns compare actual 142,738-byte and 14,851,384-byte
traces with ten paired trials per case and four methods. Every target and seed
reopens exactly. Runtime provides a large conditional body-byte reduction, with
local CPU and elapsed-time penalties.

The WAN pilot sends the larger objects from Linux through the temporary HTTPS
relay to the native Mac receiver: three cases, three paired trials, four methods,
36 targets and 36 seeds. All 72 objects reopen exactly. Runtime uses 747/766 body
bytes on generated/edited traces and beats raw completion time on those trials,
but loses to zstd and conventional adaptive transfer and has no CPU advantage.
Unrelated input falls back to raw with added prediction cost. Cold-start and
generation accounting remain explicit.

Neither the local protocol-body equivalents nor WAN HTTP bodies are complete
wire-byte measurements. Exchange time includes receiver execution and is not
pure transmission time. The three-trial pilot does not establish general WAN
acceptance, stable tails, energy savings, physical-clock agreement, MVM operation
or deployment compliance.

## Retained local artifacts and cleanup

The exact native binaries, both analysis executables, complete downloaded ZIP
and Linux sender executable remain under the task-owned directory
`/Users/casey.allard/spiralcore/local/shared-state-20261030/`, with their identities
recorded in the corresponding custody receipts. They are not silently added as
unreviewable Git binaries. The stopped successful receiver cache is retained at
`wan-cache-02` for inspection; it occupies 522,292 KiB on disk in this snapshot.

Both receiver/tunnel process pairs were stopped and their recorded PIDs were
absent afterward. The dedicated experiment secret, four variables and both local
capability files were removed. Their before/after name inventories are retained;
no capability value is published. No other experiment's configuration was changed.
The original local runtime and saved sessions were not modified.

Formal tests, failures, raw observations and review receipts are intentional
research evidence. They are retained byte-for-byte at the user's request.
Disposable compiled analyzer copies were moved to owned local storage with hash
checks. No raw measurement was dropped, overwritten, normalized or rerun to
improve the result.
