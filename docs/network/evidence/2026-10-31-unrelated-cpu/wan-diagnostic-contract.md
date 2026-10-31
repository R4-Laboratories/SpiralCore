# SC-OPT-019: prospective WAN diagnostic of an unqualified candidate

Registered before launch on 2026-10-31. The user requested before/after WAN
results, including CPU and exact recovery. The original local acceptance pilot
remains blocked: both 700-operation local campaigns were exact, but the 16 MiB
replacement sender-CPU paired median missed the unchanged `<=0.95` gate:

| Campaign | Ratio | Summary SHA-256 |
|---|---:|---|
| Phase 1 | 0.9599936918364103 | `6453a9b9d8d10726a0353589ee20c365ad54e4eabf7a6847970519cd8acaaf12` |
| Phase 2 | 0.9602631904881127 | `e4a5390a95c25863eb64f221ae6d2eea2bf781d036078a59a19d23c9cfbe4ce7` |

Keep both summaries' `advance_to_wan:false`, all observations and every failed
gate. No further production optimization or same-candidate local rerun is part
of this amendment. This is a separately registered diagnostic fulfilling the
user's WAN-measurement request, **not acceptance advancement** or a lowered gate.

Run once: six paired trials per size, all six arm orders, 36 sessions and
**420 transfers** across six stores, using the already fixed mixed sequence,
four real artifact publications per store, and baseline/candidate persistent
selectors plus prepared Whole. Use the phase-2 production implementation at
`f662f5a54886365de20be17e6ef8219d94157362`; freeze the launch revision and exact
executables separately. The baseline remains the four-file `56e15db` overlay.
Source/attempt/capability guards, HTTPS origin restrictions, 768 MiB store bound,
shared memory admission, 32-request/500 ms connection policy, authentication,
durability and exact stopped-store comparison are unchanged. No timeout tuning.

The workflow registration and artifact designation must say:

```json
{"purpose":"unqualified-candidate-wan-diagnostic","local_advancement":false,"performance_acceptance":false,"designation":"one six-pair WAN diagnostic; local gates failed; not accepted"}
```

The unchanged harness's internal six-pair WAN dose label describes its sampling
shape; this diagnostic registration defines the run's purpose. Retain source,
plan and binary identities and the registration with raw results. Do not edit
the harness to manufacture a pass. No pooling with either local campaign or
earlier WAN results, no discarded warmups, no automatic retry and no repeat
until success: stop at the first failed transfer and retain diagnostics.

Report actual body bytes, sender/receiver CPU, durable completion, paid setup,
seed costs, p95 and losing pairs against baseline and Whole. Body counts are
not full-path packet counts; exchange is nested and energy is unmeasured.
Favorable WAN observations cannot retroactively satisfy local acceptance,
establish general superiority, or justify publishing this candidate as an
accepted optimization. Preserve the unqualified status in the final report
and delivery decision, regardless of the diagnostic's outcome.
