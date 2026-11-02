# Independent capture correction review

Root reviewed the measured failure and the infrastructure owner's surgical fix.
The actual first run passed exact object/readback checks and failed capture
completion. The change enables libpcap immediate delivery and replaces a fixed
short shutdown delay with at most ten seconds of counter polling. Two identical
complete snapshots are required before stopping; the final parsed PCAP must
still equal both captured and filter-received packet counts, with zero kernel
drops. Original acceptance thresholds are not weakened. Intermediate snapshots
and the failed first run remain retained.

The parser supports the observed one-line SIGUSR1 form and the final three-line
form. Its focused checks reject the actual 49123/49130 mismatch, nonzero drops,
empty captures and missing counters. No Rust codec or measured input changed.
Fresh execution is required to qualify the fix on Linux; this source review
does not turn the incomplete first run into a successful measurement.
