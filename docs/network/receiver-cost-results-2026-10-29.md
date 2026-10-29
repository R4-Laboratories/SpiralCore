# SC-OPT-002 paired receiver result

**The pre-registered joint target was missed.** All 256 observations recovered the exact target and base after reopening; no process or observation failed, and no pair was removed.

Candidate `b4a18602c04863bac2ba404450c401109dbee3ea` changes only the repeated namespace formatting and directory comparison in quota scans. The old code is pinned by revision `74ee59df7b7a55476fa573b17f5d947fc28fb030`. Helper, dependency lock, compiler and flags are identical. Both immutable executables and complete per-file manifests are retained.

Primary metric is whole-process local operation CPU, from recipe/frame construction through validated Published and request-buffer destruction. Handler CPU is a subset. Each cell has eight paired observations, four old-first and four new-first. Ratios are the geometric mean of matched old/new measurements; values above 1 favor the candidate. Median CPU columns use the arithmetic midpoint of the central two observations.

| Background objects | Arm / max nodes | Old median CPU ms | New median CPU ms | Paired CPU ratio | New CPU wins | Paired wall ratio |
|---:|---|---:|---:|---:|---:|---:|
| 0 | raw / 262144 | 57.476 | 58.062 | 0.934× | 2/8 | 0.853× |
| 0 | sparse / 262144 | 16.640 | 16.875 | 0.964× | 3/8 | 0.876× |
| 0 | raw / 1 | 12.329 | 12.546 | 0.985× | 5/8 | 0.914× |
| 0 | sparse / 1 | 16.525 | 16.681 | 1.036× | 4/8 | 0.849× |
| 64 | raw / 262144 | 78.453 | 74.061 | 0.989× | 4/8 | 0.983× |
| 64 | sparse / 262144 | 31.301 | 24.505 | 1.125× | 5/8 | 1.055× |
| 64 | raw / 1 | 16.379 | 15.404 | 0.963× | 5/8 | 1.025× |
| 64 | sparse / 1 | 19.268 | 19.168 | 0.935× | 4/8 | 0.893× |
| 256 | raw / 262144 | 77.459 | 73.951 | 1.020× | 5/8 | 0.904× |
| 256 | sparse / 262144 | 26.386 | 34.106 | 0.907× | 3/8 | 0.949× |
| 256 | raw / 1 | 20.505 | 23.408 | 0.949× | 2/8 | 0.986× |
| 256 | sparse / 1 | 25.288 | 25.983 | 1.014× | 4/8 | 1.038× |
| 768 | raw / 262144 | 78.929 | 81.725 | 0.915× | 2/8 | 0.885× |
| 768 | sparse / 262144 | 41.157 | 39.902 | 1.080× | 6/8 | 1.057× |
| 768 | raw / 1 | 41.669 | 35.245 | 1.130× | 6/8 | 0.965× |
| 768 | sparse / 1 | 39.561 | 38.286 | 1.038× | 4/8 | 1.027× |

The primary N768 sparse/262144 cell reached **1.079686×, 6/8 wins**, below the required 1.10×. The supporting sparse/1 cell reached **1.037647×, 4/8 wins**, missing both requirements. The joint target requires both to pass; the stronger raw/1 cell is secondary and cannot replace them. A 1.10× old/new ratio corresponds to a 9.09% CPU reduction.

At the primary cell, paired CPU ratios range **0.862452–1.655413**, with paired median **1.044939×**. Its marginal wall medians are **50.599 ms old and 50.655 ms new**, while the paired wall geometric mean is **1.056883×**. These different summaries should not be interchanged or presented as a stable latency gain.

At N768, the normal raw arm has a negative total CPU result (**0.914641×, 2/8 wins**). Its Finish stage is slower in every pair (**0.873786×, 0/8 wins**). All of those observations remain in the report; the cause is not isolated by this diagnostic.

| N768 arm | Begin CPU ratio | Finish CPU ratio | Begin + Finish CPU ratio | Unchanged frame-preparation CPU ratio |
|---|---:|---:|---:|---:|
| raw / 262144 | 1.103× | 0.874× | 0.908× | 1.015× |
| sparse / 262144 | 1.027× | 1.122× | 1.079× | 1.089× |
| raw / 1 | 1.177× | 1.143× | 1.165× | 1.011× |
| sparse / 1 | 1.054× | 1.027× | 1.040× | 1.031× |

Host variation is material. One-minute load snapshots ranged from **5.99 to 23.63**. In primary block7, unchanged frame preparation was 16.296 ms old versus 9.053 ms new, alongside Finish costs of 37.501 versus 17.032 ms. No row was deleted and no after-the-fact normalization was applied. This is a mixed local measurement, not evidence of a large or reliable improvement.

Both graph-retention knobs are zero. The one-node policy changes admission/reservations and optional graph readiness; it is not a pure warming switch. Normal raw Finish prepares a canonical graph; sparse skips it. Within-arm old/new comparisons preserve the same behavior. The patch retains full filesystem scans, so cost still scales with cached publication count.

Execution ran from 2026-10-29T15:10:57Z to 2026-10-29T15:21:53Z (656 seconds), including synchronized clones and separate verification outside primary timing. All 16 processes exited zero; 256 target observations and their bases were exact. All identities, inventories, manifest readiness, reply bodies, frame/stage sums and modeled-byte sums were checked. There were 2,944 frame samples. The earlier single-block characterization is excluded from this comparison.

Raw receipts, start/end records, per-process host snapshots and hashes are in [the evidence directory](evidence/2026-10-29-receiver-cost/README.md). [The paired analysis](evidence/2026-10-29-receiver-cost/paired-analysis.json) retains all paired values and summary calculations. Reported p95 uses nearest rank; with eight observations it equals the sample maximum and is not a stable population-tail estimate. This experiment has no WAN, packet-transfer, thermal, power or energy measurement.

Recommendation: do not promote this candidate as an accepted performance win or use it to justify WAN escalation. Preserve the valid source simplification and measured misses separately; the next optimization requires its own pre-registered mechanism and acceptance test.

## Recorded disposition

The candidate passed all 124 transfer tests, fmt/clippy, a release-binary build and independent
Rust/security review. It still missed the registered performance criterion. Commit
`cb376b80dd5f54decdac124611c3ff24417e7fd1` reverted the candidate from the active branch, restoring
the previous production source exactly. The candidate commit, both executables and all evidence
remain available; no observation was deleted and no favorable secondary cell substituted for the
registered gates. The completed WAN results at `b552017` remain a separate, unchanged record.

The next proposed experiment is an explicitly versioned build using the pinned SHA-256 library's
hardware backend, with known vectors and complete-operation measurements. Earlier primitive-only
results motivate that test but do not establish its outcome. See the [optimization ledger](optimization-ledger.md)
for the current preregistration and status.
