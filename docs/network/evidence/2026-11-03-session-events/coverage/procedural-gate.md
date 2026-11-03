# Scoped procedural gate — C8 session events

Date: 2026-11-03. Verdict: **PROCEDURAL PASS for final parallel confirmations**
at qualified source `5c41d6ac2d9b40e6eead62e201d5e9b020f96019`.
This is the pipeline-audit step, not final content/security approval or a merge,
full C8, WAN, fleet-capacity, energy or certification result.

The eligible harness-optimizer followed Step 1 of the local documented
`a-team-bootstrap/.claude/commands/quality-gate.md`. No literal `/quality-gate`
command/tool was exposed or executed. This gate used this task's retained
commands/results and source receipts; no unrelated shell history, global
process arguments, private bootstrap or credentials were read or published.

## Source and RED-to-contract chain

The pinned base `231ab32103f58e52c206d6e5917d883de4e3a928` is an ancestor of the
qualified source. [Execution pins](../execution-pins.json) and the
[284-entry manifest](../qualified-source-sha256.txt) identify the implementation.
Coverage verified all 284 entries before and after; its three product hashes
match those execution pins. No product changed during coverage or this audit.

[Retained checkpoints](../README.md) distinguish semantic failures from fixture
and missing-API failures. The raw records include authenticated receipt ordering,
ACK payload loss, retained-progress API, FIFO readiness under explicitly signed
test capacity two, duplicate Reply admission, optional registered observation,
Late/replayed-Duplicate input, receiver replay, and exchange-deadline boundaries.
The capacity-one refusal and original object-store fixture failure remain
retained. Subsequent contracts and implementation address those mechanisms;
qualification-only send/event boundaries are not presented as fresh REDs.

## Executed checks and builds

- [Final live release](../validation/final-live-release.txt): **92 passed**.
- [Shared adapter conformance](../validation/final-shared-conformance.txt):
  **116 passed**, plus [object-v1](../validation/final-object-v1.txt): **8 passed**;
  the combined shared scope is **124**, not 124 in the adapter log alone.
- [Final strict Clippy](../validation/final-clippy.txt) and recorded normal
  host/guest/example builds succeeded. The guest ELF receipt is retained.
- Hosted [Linux qualification](../qualification/linux-run.json) succeeded at
  run `41629079360`. The separate portable run `41629079515` provides seven
  native-process/TCP-loopback rows: **81 tests per row, 567 total**, plus eight
  Unix-storage tests. Its [custody receipt](../qualification/portable/custody.tsv)
  records seven API-matching ZIP hashes, 188,522 bytes and 236 inner checksums.
  The extracted identities pin executed merge
  `3e7c94b23cfc4a8b1bdf0bc09cb7d8e832619527`; they do not establish native MVM
  execution on macOS/Windows or WAN support. The existing source/tree review
  binds that merge to the qualified head's source tree.

## Fresh selective coverage

One owned clean of adapter/live packages reused the existing Wick Cargo target.
The unchanged [wrapper](coverage-wrapper.sh) instruments only those two crates.
Four fresh instrumented binaries were copied outside Cargo, protected mode 500,
and executed from those copies before normal target reuse. Each object used its
own profile merge/export; all test/export stderr files are empty, and compiler
warning count is zero. [Test counts](test-counts.json) are **91/91**:
receipt 21, TCP 4, live library 57, campaign 9. These overlap the broader release
checks and must not be added to them as unique tests.

The pinned [new-side diff](changed-source.diff) defines changed lines. Coverage
unions executable `DA` lines across the isolated exports, excluding the canonical
test suffix beginning at line 621 and the session test module beginning at line
363. Receipt production has no embedded test suffix. No source exclusions beyond
those test boundaries or product edits were introduced.

| Production scope | Changed covered/total | Changed line coverage | Whole-file covered/total | Whole-file line coverage |
| --- | ---: | ---: | ---: | ---: |
| `receipt.rs` | 23/23 | 100.00% | 801/836 | 95.81% |
| `campaign/canonical.rs` | 159/181 | 87.85% | 452/489 | 92.43% |
| `campaign/session.rs` | 105/105 | 100.00% | 242/257 | 94.16% |
| **Three-file union** | **287/309** | **92.88%** | **1,495/1,582** | **94.50%** |

The changed-production executable-line gate exceeds 80%. This is not repository
coverage. [Twenty-two uncovered changed lines](uncovered-changed-lines.tsv), all
in canonical.rs, remain explicit; broader [uncovered production lines](uncovered-production-lines.tsv)
are separate. [Branch mapping is unavailable](branch-summary.tsv): no mapped
branches were exported, so no branch or MC/DC percentage is claimed.

[Verification](verification.txt), [binary identities](executed-binaries.json),
[retained binary checksums](binaries-retained.sha256) and per-binary receipts are
retained. Profiles, profdata, LCOV, full build JSON and the executed instrumented
binaries remain on Wick under `/var/tmp/spiralcore-session-events-20261103/coverage`.
Only compact receipts/scripts/statistics/checksums were copied locally; their
archive SHA-256 is `6561bbe307d0dae7dd2a70c558c8f0092bd8426caa4d72fa694fcd06dd49fa18`.

## Measurement separation, losses and custody

Protected normal host, guest and broker binaries match their qualified pins and
remained mode 500 and byte-identical before/after coverage. Instrumented results
are never timing measurements. No performance campaign was rerun for this audit.

The [existing summary](../summary.json) records **22 host cells / 11 pairs**,
192 exact publications and independent readbacks, identical paired targets and
all owned store cleanup. Four native Linux cells record 36 exact publications
and 36 extra keyless readbacks, matched counters and retained resource-budget
checks. [Host custody](../host-local-custody.txt) and
[native custody](../native-local-custody.txt) preserve the compact records before
owned payload/disk cleanup. The native cleanup is not evidence of a wider fleet.

The summary retains every pair, 23 positive-change/loss observations, setup,
readback, cleanup, lifecycle and separate CPU domains. Fixed exploratory order,
shared-host variability, source/binary baseline differences and actual object
lengths remain visible. No statistical speedup, causal attribution, energy,
send-only timing or WAN claim follows. Prior RED/tooling/fixture failures were
not overwritten or relabeled as success.

## Remaining authority and next step

The plan still leaves C8 open. Automatic retry and explicit receiver resume,
larger persistent capacity, retention cycles, WAN and unsupported native
platform/backend gates remain separate. This audit did not close an issue or
mark a goal complete. Historical checkpoint counts are distinct from the
final-source result totals. Root has updated the plan, progress README and
results report; final publication still awaits the independent confirmations.

No local completed-task file-claim row remains `in-progress`; other labs and
historical records were left untouched. No procedural evasion or scoped blocker
was found. Root may now request the documented parallel final code/Rust and
security confirmations. Final content approval and publication remain theirs.
