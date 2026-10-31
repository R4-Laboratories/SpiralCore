# SC-OPT-018 evidence packet

Read the [results report](../../persistent-session-results-2026-10-31.md) and
[registered contract](measurement-contract.md) before interpreting raw timings.
The completed local experiment, incomplete first WAN pilot, bounded diagnostic
and completed subsequent pilot remain separate datasets.

| Evidence | Binding and status |
|---|---|
| `qualification/source-revision.txt`, source/binary manifests | Frozen local/pilot source `58990cec4d190f815fccd51df9cadf94029ab78f` |
| `local/shard-0` through `local/shard-9` | Canonical plan, paired sender records, admitted requests, stopped-store exact verification and cleanup for every trial |
| `local/summary.json`, `local-review.json` | 660 exact operations, complete distributions and independent recomputation |
| `receiver-stages.jsonl`, `receiver-stages-summary.json` | Separate 44-object diagnostic at `066e6cfacd92d6a761741c9fa4b61df01aa4ba00`; large instrumentation screen failed |
| `wan/` | First pilot, GitHub `41167011759`, incomplete after 37 acknowledged operations |
| `wan/remote/custody.json` | Verified downloaded ZIP and every original artifact member; compact files copied unchanged |
| `qualification/wan-attempt1-diagnosis.json` | First failure boundary and corrected preflight/request ordinal accounting; original cause not captured |
| `diagnostic-contract.md` | Separate one-shard, at-most-66-transfer transport diagnostic registration |
| `qualification/transport-diagnostic/` | New diagnostic source `dbdbc709f0311a06efe5ffac43e5b334ca8a1a87`, frozen binaries, scripts and hashes |
| `qualification/diagnostic-*` | Actual RED/GREEN, full 410/0/2 test result, Clippy, focused coverage and scoped reviews |
| `wan-diagnostic-01/` | GitHub 41168897547: all 66 objects exact; no reproduction of the earlier failure; store/services cleaned |
| `pilot-02-contract.md`, `qualification/pilot-02/` | One subsequent pilot registered before launch; unchanged 99 source files and qualified binaries |
| `wan-pilot-02/` | GitHub 41169424060: all 396 objects exact, complete paired summary, 53 artifact members verified, six stores/services cleaned |

The local fixtures are regenerated deterministically; no expanded object history
is committed. The failed first WAN store and archived Linux binary remain in the
owned local `persistent-session-20261031/run-01` directory. Private capability
files are not evidence and are removed after the corresponding origins and
GitHub credentials are retired. The source/raw receipts are small enough for Git.

`http_requests`/body byte metrics do not cover complete IP or relay traffic.
`exchange_wall_us` is nested inside operation wall time; it is not pure wire time.
The true outer session interval includes target generation, setup, seed and ten
updates. Receiver-handler CPU is a separate process contribution added once;
socket/logging, relay CPU and energy are not measured. See every loss and outlier
in the raw distributions; none is removed from a successful cohort.

Logs and expected-failure outputs in this packet are retained research records,
including negative qualification and failed campaigns. They are not disposable
debug output. A completed remote sender does not establish exact recovery until
the stopped receiver has independently reopened and compared all original bytes.
