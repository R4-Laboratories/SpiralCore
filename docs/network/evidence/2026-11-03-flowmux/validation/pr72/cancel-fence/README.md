# Delivery-fence REDs, repair and source identity

Final source:1e69a99d27691d1db9921568d834a98814defc81, tree
94426f89602dc2714842198f9bc4dd7fa4fa8d8b. The entire selected159-file archive
matches the final executed snapshot byte-for-byte. No later TLS/Mio, SmallV1 or
Ordered-retention WIP is included. No product change was made to improve a
coverage percentage: the frozen baseline already covered91.12% when independent
review found the actual fatal-delivery contract defect.

Observed evidence remains distinct:

- Original57c production plus one real authenticated regression: semantic RED,
  post-ShutdownBoth read returned5 buffered bytes while scope was still valid.
- Initial reason-state repair: focused success; full run then exposed an old
  saturated-queue expectation that drained4 Data events AFTER fatal failure.
  Only those post-fatal expectations changed to repeated typed Cancelled;
  four publication ACKs, fifth refusal, deadline, reentry and joins stayed strict.
- The same regression then caught pre-I/O write-after-FIN incorrectly upgrading
  graceful close into fatal cancellation. Graceful I/O now returns Closed,
  preserving reverse drain; explicit cancellation remains monotonic Cancelled.
- It also caught successful JoinGuard Drop inventing cancellation after joining.
  Successful join now preserves queued data; error/timeout/panic and unjoined
  Drop remain fatal. Positive joined drain retains actual reciprocal counters.
- The ordinary34-test source then FAILED instrumented Combined half-close.
  The removed Tx ordering guard was restored; the initial source review was
  explicitly corrected and a fresh independent approval recorded.
- Final restored source: all34 release tests/9 targets, strict Clippy and fresh
  selective coverage PASS. See [restored coverage](../../../coverage/pr72-restored/README.md); old receipts never inherit that final-source status.

[contract.md](contract.md) defines delivery admission versus graceful drain and
its concurrent-copy limits. Root reviews are independent of the repair author;
the original missed review and correction remain, rather than being rewritten.
The final test still counts as ONE additional test, with several bounded real
socket cases and no arbitrary sleeps or fabricated authority. Fresh seven-row
native CI/custody remains required; old33 artifacts do not qualify final34.
No MVM backend, installed aggregate budget, fleet/sustained/WAN or energy claim.
