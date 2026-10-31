# SC-OPT-017 evidence: selector setup and compressible fallback

The [results report](../../selector-setup-results-2026-10-31.md) records completed
local and real-WAN results, including negative controls. The implementation and
measurements follow the [design](../../../specs/2026-10-31-selector-setup-design.md),
[plan](../../../plans/2026-10-31-selector-setup.md) and
[registered contract](measurement-contract.md).

## Source and qualification

- Native common-source commit: `980d5cc08b01833bb4a86f4bb3f7962b42de5055`.
- WAN launch source: `aa22837a2bbfc0ab9c4cee7d380a768cab768e16`.
- Baseline production overlay: `schema.rs` and `schema_selection.rs` from
  `4bb79f976e0939f3ef8b48c3830b07086b98f280`. Both builds keep the qualified probe
  enabled; all other measurement and production sources are shared.
- [Source revision](qualification/source-revision.txt),
  [frozen source hashes](qualification/source-files.sha256), and
  [build procedure](qualification/freeze.sh).
- [All-target summary](qualification/all-targets-summary.json): 373 passed,
  zero failed, two historical ignored tests.
- [Harness GREEN](qualification/harness-green.txt),
  [production GREEN](qualification/production-green.txt),
  [coverage](qualification/coverage.json), and [Clippy](qualification/clippy.txt).
- Preserved [profile RED](qualification/profile-red.txt),
  [profile failed first GREEN](qualification/profile-green.txt),
  [corrected profile GREEN](qualification/profile-green-attempt2.txt),
  [harness RED](qualification/harness-red.txt),
  [fit RED](qualification/schema_sender-red.txt), and
  [selector RED](qualification/warm_selection-red.txt).
- [Procedural review](qualification/procedural-review.json).
- [Independent production review](content-review.json),
  [cross-author harness/workflow review](qualification/harness-infra-review.json),
  and [combined local advancement gate](qualification/local-advancement.json).

Native baseline executable SHA-256:
`58f28ec512b2726fa4ec6461b93e57ca7597012bfa7bc30dbf7a21dbdc7f4344`.
Native candidate/loopback receiver executable SHA-256:
`a16732a9d87fbeaa4dff76e54364f16c5e2d2dfc7c03d14b086fc5313472a79e`.
Linux and Mac binaries have separate identities; cross-platform executable
equality is not required.

## Completed native observations

- [Baseline stage profile](baseline-profile.jsonl) and
  [summary](baseline-profile-summary.json), with the
  [recovered exact profiler-source custody note](baseline-profile-source/README.md).
  The profile predates later harness tests and is not represented as that later
  source snapshot.
- [Candidate stage profile](candidate-profile.jsonl) and
  [summary](candidate-profile-summary.json). Each profile retains all twenty
  iterations and distinguishes nested stage CPU from the outer setup total.
- [Preimplementation compressible control](baseline-control/summary.json).
- [Paired local transfer summary](local/summary.json): 180 actual seed/update
  objects reopened exactly, 90 scoped sessions and 60 actual fits. Raw plans,
  session rows, receiver observations, verification and cleanup receipts are
  retained under `local/shard-0/` and `local/shard-1/`.
- [All local controls](controls/summary.json), with separate
  [zero](controls/zero-1m/summary.json),
  [periodic-64](controls/periodic64/summary.json), and
  [deceptive random-gap](controls/random-gaps/summary.json) results. All sixty
  observations and 120 reopened objects remain, including the deceptive-case
  CPU regression.

The local setup and random-sender gates pass; the zero-control preparation gate
also passes. All three thresholds were registered before candidate measurement.
No failed observation or warmup was discarded. Stage profiles are sequential
attribution; the randomized paired campaign supplies the acceptance ratios.

## Completed WAN observations

[GitHub run 41156972123](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41156972123)
succeeded on its first attempt. The existing Rust summarizer accepted all six
90-operation shards and independent stopped-store verification:
**540 objects recovered exactly, 270 sessions, 180 actual fits, zero discarded
observations**. Every paired old/new winner preserves mode, complete normalized
request frames and body bytes: 600 B on the random cells, 570 B on the zero cell.

- [Accepted summary](wan/summary.json), including all nine target cells, paired
  comparisons against the old selector and Whole, and actual paid lifecycles.
- [Artifact custody](wan/artifact-custody.json): all 59 downloaded members and
  all 92 source hashes verified; native and WAN source files agree. It also records
  the intentional supervisor stop after successful sending; stopped-store
  verification followed. The full downloaded artifact remains locally preserved; compact
  records exclude executable bytes from Git.
- Hosted registration, plans, raw observations, build/test logs and source pins
  are retained under `wan/remote/`. Receiver observations and independent
  verification are bound by the summary's six receipt entries.
- The rejected HTTP-400 preflight is retained separately. Successful admitted
  receiver rows start at ordinal two; no preflight is counted as a transfer or
  retry. The campaign contains exactly its registered 540 successful operations.
- [Cleanup receipt](wan/cleanup.json): all six verified stores, twelve owned
  processes, nine repository variables and the temporary capability were removed.

The [final independent review](final-review.json) binds the published tables and
raw arithmetic. `SHA256SUMS.txt` indexes the complete retained packet except the
index itself; original output bytes, including whitespace, remain unchanged.

Linux baseline executable SHA-256:
`c9db07c2f226a2c1ea261fe3554a13d682cb83bb07eec996758463170b72ae62`.
Linux candidate executable SHA-256:
`fd530b15ab439a3f2a3b9cd21146cc7ab9c018a7025fdf2beaececbd7e726e73`.
Mac WAN receiver executable SHA-256:
`143a900ca11c0618216d7d38c7915877078209ab99e1c6edaaeac571d59b9502`.

The report retains the 1 MiB random update's worse CPU subtotal and completion
versus the old selector, the zero cell's equal bytes versus Whole, setup-inclusive
costs, all paired losses and the local deceptive-sample CPU regression. The
unchanged large-object sender path supplies no new compute-mechanism claim.

Use the [contract's command list](measurement-contract.md#commands-and-environment)
for reproduction. Fresh output paths and fresh receiver stores are mandatory.
Capability tokens are environment inputs and must never be printed. Actual setup,
Raw base seed and update costs remain separate; completion includes sender
computation and durable acknowledgment. Body bytes do not represent complete
packet or relay traffic. Local timing, sequential stage profiles, WAN completion
and sender/receiver CPU retain their separate scopes.
