# Canonical MVM host conformance measurement

Registered before implementation/timing. This is a local host interoperability
comparison, not a WAN run, live guest service, accepted performance qualification
or proof that the original HTTP cost model minimizes MVM envelope bytes.

Use current pinned MVM `3104a12c5b071a6851323fa8115d2afaf3f0bb8b` and unchanged
SpiralCore transfer source `b2dacc6f22d36454c1b8d6ce9bab654e71e68d66`. Freeze the
adapter/example source, crate lockfile and executable hash before one run.
No builds overlap timing. Use shared Cargo targets, incremental off, and stop
before work if free storage falls below 4 GiB.

Dose: three paired trials at each of 1 MiB and 16 MiB, alternating arm order.
Each arm retains one canonical receiver and its prepared sender for an initial
Raw base plus five distinct targets: 32-byte midpoint overwrite, independent
pseudorandom replacement, another 32-byte overwrite, 31-byte truncation, then
17-byte append. The append follows truncation to remain within the 16 MiB cap.
Warm additionally fits and actually publishes its schema artifact once before
targets; Whole retains one equally configured zstd context. Both arms pay the
same base seed although Whole does not require it. Total: 72 object operations
plus six schema publications, 78 exact stopped-store readbacks across 12 sessions.
One constructor scope-probe per session is actual canonical traffic, charged to
setup separately. No automatic retry, discarded warmup or pooling with WAN runs.

Use deterministic synthetic object bytes; never retain signing/encryption keys
or plaintext object payloads in JSON records. Public fixture keys are explicitly
conformance-only. Actual MVM seal/authorize/channel/open occurs in both directions.
Count sent application frames and encoded canonical envelopes once in each
direction; received counters must agree. Do not add sent and received counts of
the same envelopes. Independently report the original HTTP-body-equivalent size
from actual batch frame lengths, purely as a model comparison, not wire traffic.

Time each complete transfer including selection, hashing, canonical signing,
encryption, opening, decoding, disk verification and durable ACK. Process CPU
is the combined sequential host workload; do not manufacture separate endpoint
CPU or network-send time from this in-memory transport. Record setup CPU/time
and byte counts separately, fixture generation separately, and charged lifecycle
sums. Reopen the receiver after all operations and compare every original byte
and length before removing its uniquely owned marked test directory. Retain any
failed session's evidence and store; do not retry until it appears successful.

The example's tiny 4 KiB smoke test validates its recording and cleanup behavior;
it is qualification only and excluded from the registered measurements. Record
each operation, verification, cleanup and final completion marker. Compare warm
and Whole distributions including negative cases; no CPU/time threshold or WAN
claim is established by this bounded first adapter demonstration.
