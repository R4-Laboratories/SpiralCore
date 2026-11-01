# Shared gateway evidence

Start with [the result report](../../gateway-results-2026-11-01.md) and
[independent-analysis.json](independent-analysis.json). Source199475fa is the
measured gateway implementation; MVM smoke records additionally bind exact host
and guest executables and the unchanged pinned upstream MVM patch.

`linux-run/` retains the original compact public receipts and logs from successful
GitHub run41361459962. Its original `artifact.sha256` manifest covers862 files in
the complete archive, including executable binaries and header-only PCAPs that
are intentionally omitted from this Git checkout. Restore the full immutable ZIP
in [artifact-storage.json](artifact-storage.json) to verify that complete manifest.
The original capture counters and per-session IP/UDP/Ethernet summaries remain
here for every session. `capture-gzip-custody.txt` records an intermediate streamed
verification; those temporary derived gzip copies were removed after archiving
and independently downloading the exact original ZIP.

Initial RED/build failures and the zero-job invalid workflow run are retained.
They are qualification history, not successful runtime evidence. Current success
is established by `workflow-success.json`, `linux-run/campaign/complete.json`, the
96 endpoint receipts and the independent source/capture/binary checks. The
optional unlaunched wrapper patch is explicitly not part of the measured source.

`mvm-warm/` and `mvm-whole/` contain actual two-guest and keyless recovery results.
This n1 continuity smoke is separate from the n3 packet campaign. Old local paths
in command and custody receipts identify original execution locations; they do
not imply the large artifacts remain expanded there.
