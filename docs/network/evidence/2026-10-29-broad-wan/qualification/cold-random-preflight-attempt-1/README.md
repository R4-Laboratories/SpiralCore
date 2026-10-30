# cold_random shard 0: recorded preflight setup failure, attempt 1

Run `40984573497` at source `9dce4c57d1f41cef7365b297cd690849afce34e7`
failed before the benchmark client launched. This is a separately preserved setup
failure under the registered rule in
`source/2026-10-29-broader-wan-and-scaling.md:163`. It is not a completed cold case,
a measured performance result, or a replacement for an unfavorable measured trial.

The capture controller reported readiness, then sampled twice at 159 ms and 700 ms.
Both samples recorded zero packets captured, four received by the filter, zero kernel
drops and a 24-byte PCAP file. The samples were stable, but the frozen `quiet()`
predicate also requires captured and received counters to agree. It rejected this
preflight before the source reaches the benchmark `Command::new` call. The lifecycle
retains null client-start, client-end and client-exit fields, an empty post-client
quiescence list, and the exact error: `startup counters or file extent are not quiet
and complete`. Capture shutdown returned zero without forced cleanup. The underlying
reason for the four received packets is unproven; they are not evidence of four lost
benchmark packets.

There is no top-level `remote/rows.jsonl`, `remote/summary.json` or `remote/capture.json`.
The top-level PCAP is only its 24-byte header. The three nested capture-qualification
PCAPs and their synthetic rows belong to the earlier successful Linux fixture
qualification; they must not be counted as WAN measurement observations. No full-case
metrics or exact-reopening success are claimed.

The later local verifier tried to open the absent measurement rows. Its ENOENT error
is secondary to the remote preflight rejection. The retained local orchestration
source shows verification before the final watch-status gate; shell redirection
created the preserved zero-byte `verification.json`. `diagnosis.json` separates the
parent-reported local error from the independently checked absent file and source
control flow. The local helper has its own recorded SHA-256 and is not represented
as part of the measured Git revision.

The receiver cache remains in the original failed-shard directory and contains only
the 35-byte `.spiralcore-profile-v1` marker and empty `.owner` lock: zero object
publications. `cache-inventory.json` and `cache-controls/` preserve their exact bytes
and hashes. Process cleanup reported `cleanup_failed: 0` and `operation_exit: 1`;
the retained origin and tunnel exit records are separate from cache deletion. No
cache deletion or directory rename was performed during preservation.

Fresh authenticated GitHub API responses pin the failed first-attempt run, repository,
workflow, source and its one owned evidence artifact, **12946655973**. The original
ZIP is **60,551 bytes**, with SHA-256
`f058116e572e7e2e9eaf931a2d3eae46383b3a9fa55677d2714ceaa60405dc94`;
its actual size and digest match the API. All 53 archive members were streamed and
SHA-256 compared with the existing downloaded files. `evidence-remote-files.sha256`
binds the complete archive inventory. The compact packet copies 43 original remote
files, excluding four PCAPs and six qualification marker/PID files. Those exclusions
and the original ZIP remain local at the paths recorded in `local-raw-custody.json`.

The packet retains the canonical plan, receiver policy, build/source/executable
identities, authenticated reuse proofs, route/offload information, lifecycle and
counter receipts, original cleanup, and exact source definitions used to interpret
the failure. Source snapshots under `source/` are evidence only. No production code,
capture predicate, measurement evidence or previous case packet was changed.

No commit, push, hosted artifact deletion or retry was performed by this preservation
task. Producer artifact **12931876271** is excluded from any cleanup selection. Any
future attempt requires parent coordination and fresh isolated runtime resources;
the original failure remains retained and must not be normalized into success.
