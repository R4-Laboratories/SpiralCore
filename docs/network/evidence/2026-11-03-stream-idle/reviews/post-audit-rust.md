# Final CODE/RUST and publication confirmation

2026-11-03. **PASS — no blocking CODE/RUST or publication-consistency finding.**
This is the bounded quality-gate Step 2 confirmation after the retained
[Step 1 PROCEDURAL PASS](../coverage/procedural-gate.md), reusing the approved
[source/test review](idle-source-rust.md), [hosted custody verification](../qualification/portable/verification.md)
and [independent measurement review](../measurement-review.md). Source PR #62
and the compact result packet are approved from this review's scope. Independent
security approval, protected merge and remaining roadmap acceptance are separate.
No new source audit, builds, tests, measurements or CI runs were introduced.

## Frozen source and qualification

Fresh checks match reviewed source `960e73c59d7eab62f15a16667f95e594e1cabc8b`:

```text
29010dded67185207d677dca8bcbf10a447336c96b689e0e5308aac214ec9eac  crates/spiralcore-mvm-adapter/src/live.rs
4644f962641862ba700402b1e58b4d2fbffd7afef285177ba4d6bfbd71f24ee7  crates/spiralcore-mvm-adapter/tests/service_tcp/idle.rs
cfa21562f617d07aa8d14ce63de7285b43d062d7dd1a116742470bd41bf939c4  docs/platforms/mvm-mailbox-portable.json
```

The additive local outcome preserves strict framing, authority, original partial-
frame deadlines, fatal fences and counters. No consumer, timer, retry or resume
is enabled. The previous source/test conclusions stand without reopening them.
Hosted custody establishes 616 native client executions (88 per row, TCP11 and
receipt21), eight separate Unix storage executions, 92 Linux live release and
131 shared executions (8 object-v1 + 123 adapter). Counts overlap; zeros and
absent Windows storage remain explicit. Tested merge
`1da75b1aa4a6aacda5f47706caeeeaa213f0e99a` and reviewed head have identical tree
`7d4d1568fd6f2143992b145abf68a790b1b1d4ae`. Strict Clippy/default guest compile
and synthetic analyzers passed; these are not booted-guest or WAN results.

Coverage records agree: 88 instrumented tests, changed executable lines 96/97
(98.97%), broader mapped live.rs production lines 673/733 (91.81%), one changed
and 60 broader unhit lines retained, branch NA. Both 285-entry source verification
records match and contain only OK; before/after protected normal binary manifests
are identical and agree with the qualified normal host/guest/example identities.
These are scoped line denominators, not whole-project or semantic-path coverage.

## Publication and measurement consistency

The final report agrees with summary.json and the measurement review: twelve
host cells/six pairs, 108 publications and independent readbacks, target length
1,047,202 bytes, peak process RSS 36,823,040 bytes. The primary statistic is the
median of paired changes: Whole CPU/wall +1.51%/+0.38%, Combined -1.07%/-0.97%.
Whole's increases and Combined trial-zero +4.79%/+4.62% loss remain visible;
the linked review/JSON retain every RSS, wire, setup, cleanup and lifecycle delta.

The two current native cells have 18 publications and 18 extra exact keyless
readbacks, eight 14,851,384-byte positions and paid Combined setup. Native table
bytes/CPU/lifecycle values match exactly. Displayed current-arm reductions
41.75%/32.38%/26.38% and schema-fit, drift and unrelated losses match rounded
JSON values. There is explicitly no fresh native before/after pair. Timed endpoint,
host process/thread, relay/worker and lifecycle domains remain distinct; two-hop
physical bytes, excluded control traffic, unenforced experimental64 rate and
unqualified native RSS/plaintext budgets remain disclosed. The 14 host plus 27
native custody entries account for all 41 archive members; owned disk cleanup
is correctly described as 536,870,912 apparent bytes, not freed blocks.

All transfer measurements use strict receive; no Idle-consumer performance,
causal/statistical improvement, generic secured raw/TLS, WAN, fleet, energy or
send-only benefit is claimed. README labels the preceding table as its older
checkpoint and now links “latest recorded mailbox-client checkpoint” to the idle
report. That stale-link finding was corrected by root before these final pins.
The plan checks only the transport seam; recovery, uncertain-send reconciliation,
C9/C10 retention, installed ownership, pipeline and full C8/goal gates stay open.
The packet README maintains those distinctions and the retained compiler RED
versus fixture-count failure history.

## Final publication identities

Read the final content before hashing; only the new review file was written by
this reviewer. Root's requested README link correction changed no source or metric.

```text
2e896de0dfd63ed305544a7b8a82e8f4d166b4a47bd790ee3d9131abc582b81e  docs/network/stream-idle-results-2026-11-03.md
c27ea35637207f6bf8e89465d3b00e1582ae3c67ffee4b545220e9e1acb9ae4e  README.md
1dfecf5372204eb5cc3e5b0120084b9d1e9f65b368519a8b436d9f051aadd95f  docs/plans/2026-11-03-persistent-mvm-service.md
d1ca83df6386a4e8a4cd1b3270a2307266c214be96a70ac5ef43d8f39c810cc0  docs/network/evidence/2026-11-03-stream-idle/README.md
2075e8921128a856d82da2113e463938e1878e6f91e213b260beaf5a3c6191f8  docs/network/evidence/2026-11-03-stream-idle/measurement-review.md
ba560d04b6e1a139b727b264a7e236469d063bbf63597b4153192272e8fdc878  docs/network/evidence/2026-11-03-stream-idle/summary.json
```
