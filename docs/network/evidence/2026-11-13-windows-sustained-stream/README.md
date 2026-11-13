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

Rustfmt, whitespace checks and a locked offline GNU Windows cross-check of the storage crate and its tests passed. Independent pipeline, code/Rust and security/infrastructure source reviews passed. These establish extraction and source properties only. Native correctness, coverage and the unchanged sustained fixture remain the acceptance gates. **Performance improvement is unproven until the candidate's native results are recorded.**
