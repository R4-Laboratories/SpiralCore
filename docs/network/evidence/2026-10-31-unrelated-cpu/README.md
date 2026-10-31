# SC-OPT-019 evidence

[Result and limitations](../../unrelated-cpu-results-2026-10-31.md).
Status: exact reconstruction and encoded-byte parity passed; **performance
acceptance failed**. Both local large-replacement CPU gates missed 5%. The WAN
run was prospectively registered as one unqualified diagnostic, not advancement.

## Registered inputs and sources

- [Original measurement contract](measurement-contract.md),
  [phase-2 contract](phase2-contract.md),
  [WAN diagnostic amendment](wan-diagnostic-contract.md).
- Production baseline: `56e15db5c77fe2ecdc03476d8e09664cd4213fd1`.
- Phase-1 source: `a128b3a535964dba569e9bb3c124b0234f6c3a80`.
- Phase-2 runtime: `f662f5a54886365de20be17e6ef8219d94157362`.
- WAN launch: `c83f7656e2a9f92b7d6bc8557af987391d147516`; runtime unchanged
  from phase 2, with separately hashed diagnostic workflow/test declarations.
- Both selectors use the same current harness. The baseline overlays exactly
  `delta.rs`, `schema.rs`, `Cargo.toml` and `Cargo.lock`; Whole uses the unchanged
  prepared zstd encoder. Sources/executables are bound in `qualification/`,
  every campaign registration and the hosted artifact manifest.

## Retained measurements

- `baseline-profile.jsonl`, `delta-profile.jsonl`, `combined-profile.jsonl`:
  72 independent stage observations each. Separate diagnostic runs; clocks are
  not additive and runs are not paired. The delta-only selector regression is
  retained. [Baseline metadata correction](profile-provenance-correction.json)
  preserves the original mistyped caller revision and verifies actual sources.
- [Phase 1](local/summary.json): 700 exact local operations, large CPU gate
  failed. Ten shards each retain plans, paired raw rows, receiver telemetry,
  stopped readback and cleanup. These files are force-tracked because the host's
  global `**/local/` ignore rule otherwise excludes this evidence directory.
- [Phase 2](local-phase2/summary.json): separate 700 exact operations and the
  same failed large CPU gate; no pooling or threshold change.
- [WAN summary](wan-pilot-01/summary.json): 420 exact operations, six paired
  trials per size, 36 persistent sessions. Its inherited dose label remains
  `six-pair-wan-pilot`; the explicit [launch](wan-pilot-01/launch.json) and
  [hosted registration](wan-pilot-01/hosted/registration.json) designate it an
  unqualified diagnostic with `performance_acceptance:false`.

## Real WAN custody and regeneration

[GitHub run 41177063558](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41177063558)
completed once on October 31, 2026. Linux sender → HTTPS Cloudflare relay →
macOS receiver, synthetic/public fixtures only. No retries or discarded samples.
The download ZIP SHA-256 is
`616bf7589ee61a3550570db1318af1c7525cfd37ac3ba7f22dd4be7bf77671ed`.
All 67 manifest-listed artifact members, including two frozen Linux executables, were
verified against `hosted/artifact-sha256.txt` before copying compact evidence.
The two executables remain in the 30-day GitHub artifact and owned local frozen
archive, not in Git. All other artifact members are retained under `hosted/`.
That original manifest uses the `unrelated-cpu/` prefix and is preserved intact.

`plan-N.json` byte-matches `hosted/plan-N.json`. `hosted/observations-N.jsonl`
contains the actual sender rows for all three arms; `receiver-N.jsonl` and
`verification-N.jsonl` bind each to receiver telemetry and exact stopped-store
readback. `connections-N.jsonl` records sanitized connection diagnostics.
`launch-command.sh`, `supervise-command.sh`, `verify-command.sh` and
`cleanup-command.sh` preserve the executed administrative steps. Tool startup
initially selected a not-yet-created extraction working directory and failed
before executing anything; extraction then ran from its existing parent. This
did not rerun transfers or change the received artifact.

To independently regenerate the summary, build the pinned Rust example, then
invoke its `unrelated-summary` command with the six ordered triples
`plan-N.json hosted/observations-N.jsonl verification-N.jsonl`, followed by a
new output path. It fails on missing/duplicate operations, source mismatch,
nonexact readback or baseline/candidate frame/body disagreement. The frozen
local helper used for the retained summary is in `receiver-binaries.sha256`.
The generation-free and generation-inclusive clocks, full distributions and
paired ratios remain in the summary; medians of paired ratios are not ratios
of unpaired medians.

The unchanged source manifest after freeze, artifact metadata and download
verification are retained. After all 420 stopped readbacks passed, the six owned
caches were deleted (3,448,488 KiB allocated); all twelve owned receiver/tunnel
processes were confirmed stopped. The nine temporary GitHub variables, token
secret and local capability were removed. `cleanup.jsonl` and absence receipts
record that cleanup. No payload cache or private capability is committed.

## Qualification and interpretation

`qualification/` retains actual expected RED/GREEN outputs, resolved failures,
release/Clippy/coverage receipts and compact independent reviews. Hosted tests
pass 431/0, with two preexisting ignored tests, across 55 targets. Phase-2 focused
coverage is 97.00% schema lines / 97.33% delta lines, 100% functions; no branch
or whole-project coverage claim. No rebuild overlapped a timed campaign.

Formal measurement/test logs are deliberately retained under the user's explicit
request to record findings; they are evidence, not disposable debug dumps.
Their original trailing whitespace is not normalized. `SHA256SUMS` covers the
committed packet except itself and permits future content-integrity checks.

Body bytes exclude HTTP/TLS/packets/relay hops. Exchange is nested and includes
receiver work. CPU is sender-process plus receiver-handler time, not full-system
energy. Selector frame capture is inside timing in both selector arms but absent
from Whole. The report retains this overhead, local failed gates, the WAN small
warm CPU regression, paid seeding, p95 maxima and unshaped six-pair limits.
No direct-peer, 30-pair acceptance, energy, FIPS or MVM qualification is implied.
