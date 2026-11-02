# Independent scoped measurement and infrastructure review

Reviewed `run-combined.sh`, `analyze-combined.sh`, `combined-session.jq`,
`combined-analysis.jq`, their synthetic test, and the measurement contract before
the frozen-source native run. No blocking finding remains in this scope.

The runner binds the current committed source and clean implementation/scripts,
registered signed host binary, guest binary and kernel hashes before launch. It
requires a fresh output, records the 36-session Latin-rotated arm schedule before
execution, runs sequentially at the same explicit bandwidth profile, and checks
a 4 GiB free-space reserve. Failure stops the run with owned data retained. Only
the newly created session's cache or ext4 disk is removed after independent
receipt acceptance; public receipts and cleanup identities remain. No bootstrap
credentials are serialized to evidence by these scripts.

Before cleanup the analyzer requires matching plan, successful worker receipts,
the fixed mailbox allowance/burst, synchronized receiver/recovery disks for the
two-guest stage, all exact read-only readback events, and every operation's
identity, length, position and reverse-direction counters. Session totals use
actual encrypted envelope and framing counters, with the explicitly identified
two-hop multiplier. All eight cross-arm payload identities must agree.

The comparison groups pair by stage, requested-size tier, operation kind where
applicable, and trial. They require exactly one observation of each arm in each
trial and compute the median of the three trial ratios. They do not divide
independently reported medians. Setup publications and setup CPU/wall remain in
paid session totals. Logical bytes, operation CPU, elapsed and full lifecycle
remain separate fields. Nested guest/worker CPU is not combined.

The initial session validator lacked explicit worker-profile, synchronized-disk
and read-only-readback checks. These were added during review before any native
campaign launch; no production transfer code change was needed. A small known
ratio assertion was also requested and added to make the synthetic arithmetic
regression explicit rather than checking only comparison row counts. Counter
validation now requires all six numeric fields on both sides, so two missing
reverse-direction fields cannot accidentally compare equal as null.

Independent execution of the synthetic harness passed 36 synthetic sessions,
known operation/session ratio assertions, and six rejecting mutations: failed
reboot exactness, wrong object identity, directional-counter mismatch,
unsynchronized recovery disk, wrong worker rate, and missing reverse-direction
counters. See `review-analyzer-test.txt`; these synthetic results are not transfer
measurements. Bash syntax checking also passed. Final native results and their
arithmetic require separate review after execution.

Limitations remain explicit: three descriptive trials, shared-host scheduling,
common fixture-generation CPU within setup, receiver framed-receive CPU outside
the processing counter, no physical energy measurement, and local HVF/vsock
rather than a new Internet WAN or production mailbox qualification.
