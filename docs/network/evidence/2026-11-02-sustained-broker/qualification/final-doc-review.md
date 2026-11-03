# Final documentation and measurement review

Verdict: **APPROVE**, after the native CPU/session interval labels were clarified.
Scope: the documentation-only results branch after product PR #52 merged as
`111ceca`; no product change, build, benchmark or external publication was
performed by this review.

The reviewer independently rederived the five Linux completed-row totals:
**13,684 operations, 20,537 exact comparisons, 324,300,612 actual framed-envelope
bytes, two incomplete admissions**, all five recorded qualification gates and
counter agreements passing. Table CPU, wall, RSS and per-worker p95 values match
the original rows. The two incomplete admissions belong to the hot workers;
healthy-worker completions, the burst stopping rule and narrow interference
scope remain explicit. Sampled FDs reach 37. Final budget counters after worker
drop are not presented as live retained-state occupancy.

Object table figures match each successful original `broker_objects_verified`
row. All arms share the same eight target byte strings and exact-object cache;
Combined pays its two setup publications. The original large raw failure has no
complete transfer result. The later global post-join Tenant/Control/Identities
refusal is kept separate and is not attributed retrospectively to that original
failure. Aggregate process CPU and role thread CPU are not pooled or summed as
independent endpoint totals.

The seven native rows sum to **62 exact publications and 44 keyless reboot
checks**. The large two-guest raw/zstd/Combined byte, CPU, wall/lifecycle and
1,467/843/505-Call figures match `native/summary.jsonl`. The two-hop stream-byte
scope and separate per-process native budgets remain clear. Native success does
not erase the shared-budget raw failure or establish aggregate host accounting.
The smaller Combined lifecycle loss remains reported.

The only requested correction was measurement labeling: native CPU/session
values are recorded timed-interval sums, not whole-process lifetime CPU/time.
The final table and prose now identify exclusions for bookkeeping/copies/hash
work, JSON output and independent readback outside those intervals; the full
boot/transfer/keyless-recovery wall measurement includes those activities. The
README calls the headline CPU timed and the byte count logical mailbox traffic.

The Mac deadline failure remains a failed gate. Expiry is source-supported for
the three consumption refusals; the fourth Unauthorized error and aggregate host
observations do not uniquely establish a scheduling cause. The recovery mode's
unrecorded provisioning CPU/time remains unavailable. Short probe retention,
single fixed-order pilots, no booted-fleet inference, Windows durable-gateway,
canonical WAN and production supervision gaps are retained explicitly.

Reviewed file SHA-256 identities:

| File | SHA-256 |
|---|---|
| `README.md` | `e8d59ab8e3d5cf7850cb8aedf3886be9b73cbed6859c77644e432c43922fbfd4` |
| `docs/network/sustained-broker-results-2026-11-02.md` | `7776f52535cbc5d7596a325112eb94e4c7f3de4178ddd6ca98941d69b9a8be35` |
| Evidence `README.md` | `2cbf82328f33dcc3282d12fad68ce4f3ed665bbd48bdfb63ebfcdec15821fe56` |
| `linux-summary.json` | `d71b37a9fdb44e218137c0514757090215b6994e72df431548f338ec6f3977cd` |
| `native/summary.jsonl` | `dc08b663a7e79c5dab32b1db0d0f4389d60805868b56b0b02dd7852f5dcdf979` |
| Final-platform `README.md` | `0fd0eeef8701e1d1732c95e0b55cd88e2bf1ec64233d50e0bded352ebc857094` |

No remaining must-fix documentation or quantitative interpretation issue was
found in this reviewed scope. This review does not convert unexecuted broader
release gates into completed work.
