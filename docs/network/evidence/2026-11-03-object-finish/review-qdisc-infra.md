# Existing bounded qdisc drain: code and infra review

Verdict: **PASS** for carrying the already published measurement correction
as its own commit, separate from the QUIC product fix. Compared exact
`86cae0a509deb44043265c0dd16128fd43509cb1` →
`b0a70391cc18580bd7b7595031044e9cfefcdf15` for only these two scripts:

- `runtime-object-netns.sh`: SHA-256
  `92301af0d2bcb6ba912e15d09fd591056e450f8d5acd2ee6f6eb79caf776dda5`.
- `runtime-object-infra-check.sh`: SHA-256
  `47557aad0881e78e074a321c52bc38964065407ecd906c53fa16189b4c91f72b`.

The current root copies match that published source exactly (46 insertions,
5 deletions). The extracted predicate is unchanged: exactly one netem qdisc,
qlen0, nonnegative recorded drops, limit256, delay0.04seconds and
rate1,250,000bytes/second (10Mbps). Both actual interfaces must satisfy it in
the same bounded observation round. Fifty rounds with0.1-second intervals
record every observation and fail if no qualifying round occurs; the accepted
snapshots are copied as measurement-after records and checked again. The new
drain performs only `tc -s -j qdisc show`; it never flushes, recreates or resets
qdisc counters and never retries the application session/campaign. Existing
initial calibration/measurement setup is unchanged. LAN handling is unchanged.

One pure infra execution on the owned86ca archive plus exactly those two
published script overlays and its published registration passed, exit0.
The qdisc fixtures comprise1 positive and5 negatives: queued, missing,
duplicate, wrong rate and wrong limit. Other existing registration/binding,
capture, invariant and receipt-parity checks also pass; original per-case
JSON/stdout/stderr/exits and `result.json` are in `validation/qdisc-infra/`.
This validates predicates/syntax only, not live queue drainage, native guests
or WAN behavior. No network namespace, sudo, VM, codec, Whole-object control
implementation or lockfile was changed/executed for this review.

The independent PR74 assembly must retain its main-source Whole control pin;
this review does not authorize carrying the later PR71 Windows control-pin
change. The earlier failed candidate remains failed, and the published prior
18-session b0 campaign is separate evidence rather than an execution repeated
by this reviewer.
