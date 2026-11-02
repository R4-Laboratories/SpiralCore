# Failed campaign receipt-shape diagnostic

This diagnostic uses the retained `failed-run-41373268385/campaign` receipts.
The first Raw session completed all 12 deliveries and exact reopened readback,
but capture recorded 49,123 packets against 49,130 received by the filter. The
campaign exited nonzero and has no successful session completion receipt.

A temporary copy of analyzer source
`d141302a1f1adca2ea34c7470e65721b7c451a1d090ade40b2553bdb59de2ead`
renamed its private `read` function to `read_original` and appended the retained
`analysis-failed-run-smoke-wrapper.rs`. Only the missing session completion JSON
was supplied as a clearly synthetic test fixture, to reach downstream validation.
No original receipt was altered. No successful capture or terminal status was
supplied. The standalone analyzer's production source remains unchanged.

The test passed all twelve real operation/fixture/receiver batch joins, opposite
channel counters, reopened ID/length sets and fourteen cumulative/prefix rows.
It then **required** rejection of the real incomplete capture, and separately
required full-campaign rejection of the actual nonzero exit. No session summary
or performance report was produced. This is receipt-shape validation only, not a
qualified measurement or rescue of the failed campaign.

The focused command used the same standalone package dependency lock, shared
Cargo target, Rust 1.97.1, two build jobs and incremental compilation disabled.
`analysis-failed-run-smoke.txt` and `.exit` retain the observed one-test PASS.
