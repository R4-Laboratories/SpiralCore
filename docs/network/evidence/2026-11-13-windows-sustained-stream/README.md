# Windows sustained object stream: retained deadline failure

The original Server 2025 fixture sends 4,096 distinct 16 KiB blocks in each direction, verifies exact readbacks and limits each endpoint to 64 retained publications. Its original 240-second writer deadline and two-second operation limit remain unchanged.

[Source-pinned baseline rows and artifact identities](baseline.json) preserve both the preceding success and the subsequent failure. The two runs used the same relevant source trees, compiler, lockfile and runner image. Neither retained artifact contains executable bytes or a binary SHA-256, so identical executable identity is unverified. These are native same-host TCP/TLS component results, not MVM or WAN qualification.

| Original hosted run | Exact completed blocks per direction | Application bytes per direction | Snapshot wall time | Process CPU to snapshot | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| [43405633108](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43405633108/job/131819281047) | 4,096 | 67,108,864 | 239.228741 s | 89.671875 s | Original writer gate passed; complete lifecycle 241.787469 s |
| [43408894339](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43408894339/job/131830165696) | 3,613 | 59,195,392 | 242.026714 s | 112.546875 s | Original deadline failed; both scopes valid at failure |

No byte mismatch was reported. The second run is a failed sustained-operation gate; the completed prefix does not qualify the full workload. Readers subsequently observed timeout or EOF after the writers failed.

A separate three-trial, 128-block commit control measured quota scans at 21.8–22.1% of receiver wall time and 38.7–48.2% of receiver-thread CPU, file sync at 31.2–32.2% of wall time and namespace publication at 20.7–21.5%. Those controls did not profile the failing duplex workers and cannot establish its causal bottleneck. Begin and Commit each rescan for different admission/publication decisions; reusing a Begin total across frames was rejected because intervening filesystem and admitted work may invalidate it.

The candidate reuses the validated UTF-16 filename held in the synchronous directory-enumeration callback instead of re-encoding and allocating it for each relative open. It also reuses that exact code-unit length for path limits. Both quota scans, fresh effective identity, ACL, reparse/type/link, volume/file identity, post-ACL EOF, admission and durability checks remain. The coverage workflow binds the revised source and the same ten test-only excluded lines. No test, threshold or deadline was relaxed.

Rustfmt, whitespace checks and locked offline GNU Windows cross-check and strict Clippy of the storage crate/tests passed. Independent pipeline, code/Rust and security/infrastructure source reviews passed. The native acceptance results follow; a causal performance improvement remains unproven.

## Native successor result

[PR #256](https://github.com/R4-Laboratories/SpiralCore/pull/256) merged as `e1a662c3` after all 28 executed jobs passed. One unrelated candidate-CPU job was path-filtered. The [Server 2025 original job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43415998987/job/131854087298) tested merge `7f30477b`, whose tree matches PR head `ad24f6fe`. [Verified exact rows and artifact hashes](native-result.json).

| Successor measurement | Result |
| --- | ---: |
| Exact application bytes per direction | 67,108,864 B in 4,096 blocks |
| Snapshot elapsed / process CPU | 211.631661 s / 88.843750 s |
| Complete lifecycle elapsed / process CPU | 211.687734 s / 88.859375 s |
| Setup elapsed / process CPU | 0.113671 s / 0.015625 s |
| Endpoint A TLS sent / received | 5,395,050 / 5,395,015 B |
| Retained publications / bytes per endpoint | 64 / 1,051,712 B |
| Evictions / pinned publications per endpoint | 4,032 / 0 |

The peer TLS counters match in reverse; their combined traffic is 10,790,065 B, excluding TCP/IP headers. All four write-ack/read progress counters reached 4,096. The snapshot had 28.368339 seconds of margin before the unchanged writer deadline. CPU is process CPU, not send-only or codec-only time. The artifact has no measured RSS/high-water mark or uninstrumented executable digest.

The job's original test footers report 69 FlowMux cases, 20 ordered/resource/schema cases, three metadata cases and one case each for existing-file opens, eviction and commit-cost controls, with zero failures or ignored tests in those selectors. The downloaded ZIP matched GitHub's digest; all 20 members matched the original checksum manifest.

This is one successful hosted execution. Separate 128-block quota controls measured 619–633 ms, while their inclusive block times were 4.435–6.284 s, longer than in the earlier failed job. Variable scheduling and sync costs prevent attributing the full-stream difference to the filename allocation change. These results establish a native pass under the original workload, not a repeatable optimization speedup or MVM/WAN/energy/fleet qualification.

## Source-scoped coverage

The separate [native Server 2025 coverage job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43415998823/job/131854102412) covered **225 of 237 mapped production lines in `quota.rs` (94.94%)**, above its 80% gate. Three selected tests passed serially with zero failures, filters or ignores. The ten pinned test-only source lines contained seven mapped LCOV lines; only those seven mapped lines were removed from the denominator. All 64 artifact members and six storage-source pins verified. [Coverage inputs, denominator and limitations](quota-coverage.json).

The coverage executable was instrumented and separately hashed. Its timings do not enter the transfer result above. Controller, harness and repository-wide coverage percentages remain unmeasured.
