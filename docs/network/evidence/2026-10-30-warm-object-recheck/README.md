# SC-WAN-005 evidence packet

[Result and interpretation](../../warm-object-results-2026-10-30.md).
[Registered measurement contract](measurement-contract.md).
[Successful first-attempt WAN run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41140288919).

This packet measures the existing verified-base sparse path against prepared
whole-object zstd/Raw at 1 MiB and 16 MiB, with exactly 32 changed bytes. It
separates prior-object reuse from SC-OPT-014's codec-context/schema comparison.
Thirty paired trials per size produce 120 paid Raw seeds and 120 updates, all
exact after stopped-store reopen. Sparse updates use 600 HTTP-body bytes; all
Whole attempts select Raw after timed compression. No production codec changed.

Local implementation source: `5a4f141ac64aa09533a6561d6ae72b3f04856e95`.
WAN source: `605ee6f1b2f5012c497e65fd56a0499f2de94253`.
Compiled sources and workflow are identical between those commits; the latter
adds local observations and launch reviews. Plans bind the measured revision,
source files, actual fixture identities, authorized namespaces and trial order.

| Evidence | Purpose |
|---|---|
| [qualification/harness-tdd.json](qualification/harness-tdd.json) | Retained RED/GREEN, fixture/cost/receipt tests and final source/binary hashes |
| [qualification/workflow.json](qualification/workflow.json) | Actual source/URL/auth shell gates and all six shell syntax blocks |
| [qualification/launch-qualification.json](qualification/launch-qualification.json) | 324 passing all-target tests, two preexisting ignored, focused rerun after equivalent lint correction, Clippy and release build |
| [qualification/procedural-review.json](qualification/procedural-review.json) | Sequential procedural gate; status reflects its pre-local-run snapshot |
| [qualification/code-rust-review.json](qualification/code-rust-review.json) | Independent source and 24-object local result review |
| [qualification/security-infra-review.json](qualification/security-infra-review.json) | Cross-author bounded security/infra approval |
| [local-qualification/](local-qualification/) | Three local pairs per size, actual disjoint receiver process, all 24 exact readbacks and cache cleanup |
| [wan/launch.json](wan/launch.json), [wan/github-run.json](wan/github-run.json) | Actual launch source, attempt, platform/feature scopes and hosted completion |
| [wan/hosted/](wan/hosted/) | All nonbinary downloaded artifact members, source/compiler/features, plans and nested raw session measurements |
| [wan/summary.json](wan/summary.json) | All update, seed and paid lifecycle distributions and paired ratios |
| [wan/additional-analysis.json](wan/additional-analysis.json) | Actual cohort sums and the retained losing 16 MiB timing pair, derived without another transfer |
| [wan/custody.json](wan/custody.json) | 36 checked artifact members, 83 source hashes, three identical cross-host plans, binary and verification hashes |
| [final-review.json](final-review.json) | Independent raw/result/report arithmetic review |
| [wan/cleanup.json](wan/cleanup.json) | Six stopped processes, retired credentials and three removed synthetic stores |

The three `wan/receiver-*.jsonl` files retain every admitted request. Each has
one startup row and 80 admitted observations; one unauthenticated preflight
request explains its ordinal gap. All decoder-cache counters remain zero because
Sparse needs no zstd decoder and every Whole update falls back to Raw.
The three `wan/verification-*.jsonl` files retain all 240 independently regenerated,
fully compared readbacks. No trial is discarded or replaced. Original raw output
is preserved byte-for-byte; source whitespace checks do not normalize artifacts.

Mac verifier SHA-256:
`ff8a8ca42cfbd79168a69375f28af54cd548891631a7f8b1b583a24e17c9b08e`.
Mac receiver:
`572667bda7eba40c269e92c58e7bc6747e96a026c5cc92d06532328e4d26620c`.
Linux sender:
`b269e46ad8dc057e6f8c05d43ed4e2f0308013a35c53fbaf7f90d827271f31f6`.
Rust 1.97.1 release builds use native hardware SHA on the Mac and default features
on Linux. The Linux executable is retained locally and in Actions artifact
`13017854002`, with 30-day retention. The downloaded manifest also names that
binary, whose hash is verified in custody; binary blobs are not duplicated in Git.
The source, lockfile, fixture definitions and compact raw measurements remain
in Git after artifact expiry. No paid VM was provisioned.

Reproduce from the pinned source using `warm-plan`, `warm-paired`, `warm-verify`
and `warm-summary` as specified in the contract. Fresh namespaces, stores,
capabilities and live endpoints are required. The retained supervisor documents
this campaign's process ownership, but its private run paths and retired
credentials must not be reused. Stop all receivers before verification. Only
remove generated stores after exact readback and evidence custody succeed.

HTTP-body bytes do not count the whole packet/relay path; endpoint CPU is not
energy. Exchange elapsed includes remote compute and waiting. The full result
retains scope exclusions, paid setup, control variability and the one large
update whose WAN time lost despite lower bytes and CPU.
