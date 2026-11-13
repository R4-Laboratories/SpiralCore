# Windows ordered-stream baseline with the original publication writer

Two hosted jobs timed out during a read or Commit operation. Two other jobs on the same source completed 4,096 exact 16,384-byte blocks in each direction. All four ran both endpoints in one process on one Windows runner. The operation deadline was 2 seconds, the workload deadline was 240 seconds, and each endpoint could retain up to 64 publications. [Component run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43496596957) · [Diagnostic run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43496597282)

| Runner and build | Outcome | Blocks acknowledged and read back per direction | Wall at concurrent snapshot (s) | Process CPU through snapshot (s) | Completed lifecycle wall / process CPU (s) |
| --- | --- | ---: | ---: | ---: | ---: |
| Windows 11 ARM64, default | Timeout | 424 | 17.286923 | 11.671875 | Unavailable |
| Windows Server 2025 x64, witness-timing | Timeout | 230 | 31.350726 | 5.750000 | Unavailable |
| Windows Server 2025 x64, default | Pass | 4,096 | 181.846895 | 103.906250 | 181.901685 / 103.906250 |
| Windows Server 2025 x64, witness-stall | Pass | 4,096 | 152.602786 | 72.937500 | 152.687079 / 72.968750 |

On ARM64, one endpoint recorded a caller read timeout and the other reached its deadline at the Commit reply check. The sender uses the remaining operation budget before and after waiting for a reply. The record leaves open whether a reply arrived late. The test ended after 18.53 seconds. [ARM64 job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43496596957/job/132131307946)

In the Server 2025 timing run, one endpoint recorded a caller read timeout and the other observed a connection reset. At the concurrent snapshot, the right receiver had completed 231 Commit-handler spans and 230 Commit-reply send spans. Its longest completed Commit-handler span was 1.4881464 seconds. The timing record gives no block ordinal for that maximum. The test ended after 33.50 seconds. [Server 2025 timing job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43496597282/job/132131197049)

Both successful jobs verified 67,108,864 application bytes per direction, with final TLS counters of 5,395,050 bytes left-to-right and 5,395,015 bytes right-to-left. The diagnostic and default jobs ran independently. Their elapsed times describe those executions. All four jobs used the original writer, which issued four header writes for a plain publication and five for a publication with a manifest. These results leave the cause of the timeouts and the effect of combining the header writes undetermined.

The hosted merge and PR head have the same Git tree. All four artifact ZIP digests and all 45 members listed in their checksum manifests were verified. The diagnostic jobs also recorded matching executable hashes before and after execution. The default jobs recorded executable names only. The companion file includes exact source and runner identities, per-run artifact hashes, setup measurements and directional TLS counters. [Baseline measurements and source identities](header-write-baseline.json)
