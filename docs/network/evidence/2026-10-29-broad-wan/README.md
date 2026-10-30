# Broad 16 MiB WAN campaign evidence

This packet begins with qualification records for the registered twelve-case,
four-method, thirty-pair experiment. It is distinct from the earlier v3 localized
edit campaign. New v4 receivers disable optional raw/zstd DAG warming equally
across methods, and include a conventional zstd shared-base control.

The registration is in
[the workload plan](../../../plans/2026-10-29-broader-wan-and-scaling.md), with
[the design contract](../../../specs/2026-10-29-broad-wan-design.md) and
[build custody](../../../plans/2026-10-29-broad-wan-build-reuse.md).
Qualification records alone do not establish WAN performance.

The [completion receipt](completion.json) binds the final report and all twelve
case manifests, aggregates and independent reviews. It records fresh verification
of 4,546 compact case checksum entries and the complete operation/capture census.

The original shell-only `workflow-configuration-review.json` does not qualify
negative rejection paths: native Bash 3.2 continued after failing standalone
`[[ ... ]]` checks despite `set -e`. Preserve that historical receipt, but use
the later Rust-asserted workflow and orchestration tests as rejection evidence.
The repair uses explicit failure exits; no failed scientific trial is replaced.

## Qualification history

- `qualification/native.json` and `coverage.json`: initial extraction and local
  HTTP qualification, bound to their listed working-tree hashes.
- `qualification/native-complete.json` and `coverage-complete.json`: all 167
  native tests, including both expensive HTTP cases, plus one doctest, release
  binaries, formatting and Clippy after the veth fixture repair. This Mac-only
  coverage is 78.8933%; the earlier 82.0463% does not describe this later source.
- `qualification/linux-attempt-1.json` and the first repair record: retained
  failed Linux setup caused by the intermediate tcpdump statistics format.
- `qualification/capture-linux-attempt2-repair.json`: retained missing-first-SYN
  failure, diagnosis and source repair. Its `NOT_RUN` fields describe the state
  when that immutable record was authored.
- `qualification/linux-attempt-3/`: actual successful live Linux qualification
  at `bc8c5722d634a34dddc643c19301ef00064eea84`, plus independent Rust review.
  Normal traffic has 137 saved/received packets, zero reported drops, a captured
  initiating SYN and complete body coverage. Interrupted and continuing-traffic
  cases are correctly incomplete; all owned resources were removed.

The Linux fixture is synthetic local Ethernet/plaintext TCP. It is neither a
WAN speed measurement nor evidence that the historical v3 capture discrepancy
is resolved. Final sender-build reuse requires its own exact-source producer
qualification and manifest before any consumer executes downloaded binaries.

Raw qualification artifacts, packet captures and diagnostic source are retained
locally under `/Users/casey.allard/spiralcore/local/broad-wan-20261029/`.
Compact records bind them by hash. Owned diagnostic text is retired only after
structured findings are preserved; scientific rows and captures remain.

## Recorded WAN cases

The [final result report](../../broad-wan-results-2026-10-29.md) records all twelve
completed, independently reviewed transfer cases: [localized](cases/localized_32/README.md),
[scattered](cases/scattered_32x32/README.md), [prefix insertion](cases/prefix_4096/README.md),
[middle insertion](cases/middle_4096/README.md), [deletion](cases/delete_4096/README.md),
[append](cases/append_4096/README.md), [truncation](cases/truncate_4096/README.md),
[1 MiB overwrite](cases/replace_1m/README.md),
[structured bytes](cases/structured_localized_32/README.md), and
[opaque compressed bytes](cases/compressed_localized_32/README.md), and
[cold random bytes](cases/cold_random/README.md), and
[cold structured bytes](cases/cold_structured/README.md).
Each has thirty pairs across four arms. Warm cases have 240 exact seed/update
operations each; each cold case has 120 exact transfers without seeds. All 2,640
operations reopened exactly, totaling 44,288,504,160 cumulative compared bytes.
Fifty-nine of sixty measured captures are complete. Middle-insertion shard 1 has
ambiguous tuple-reuse attribution and no complete whole-case IP statistic; its
[separate diagnosis](qualification/middle-tuple-reuse/README.md) preserves the
original failure. All registered transfer cases are complete; no timed observation
was replaced, and no incomplete-capture subset is promoted to a whole-case IP claim.

The first cold-random setup failed before launching the measurement client. Its
[separate packet](qualification/cold-random-preflight-attempt-1/README.md) records
the unchanged capture gate's rejection and zero measured operations. The
[path-preservation receipt](cleanup/cold_random_setup_attempt_1_preservation.json)
maps its original local directory to `shard-0-attempt-1-preflight-failure`, verifies
every moved file, and keeps the packet's original paths interpretable. The fresh
setup uses the same canonical plan; this does not replace a measured trial.

Read the [archive-member custody correction](qualification/artifact-member-custody-correction/README.md)
alongside the compressed-byte and failed-setup packets. Their original empty
member manifests remain historical evidence; the correction supersedes the
unexecuted per-member assertions with two actual matching checks. All 613 affected
members agree, including the five cold-random archives checked before sealing.
Archive identities and scientific measurements remain unchanged.

Final measured source is `9dce4c57d1f41cef7365b297cd690849afce34e7`. The
[successful producer](qualification/linux-producer-2/README.md) records current
Linux qualification and 90.526% optimized-profile line coverage. The first
coverage producer timeout and capture setup failures remain separate history.
Case packets retain all statistical vectors and independent reviews; raw ZIPs
and PCAPs remain local and hash-bound. Exact hosted artifact deletion receipts
are in `cleanup/`; each deletion followed committed local preservation.
