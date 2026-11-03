# Hosted qualification custody

Final portable run **41611048527** and Linux run **41611048227** succeeded on
reviewed head `fdfbd2f`. Client artifacts identify actual executed PR merge
`f997c34897ce444830efd12326526caf309dd573`; its tree matches the head:
`d8295443e985e6759d184f89d83a640c58327957`.

`artifact-api.json`, `EXPECTED-SHA256SUMS` and `original-zips/` retain all seven
original archives and their API digests. All seven digests verified. Extracted
archives retain their original files/manifests; only the checksum parsing stream
normalized Windows CRLF. All **236 inner-file checks** passed. Each derived
`verified-members.txt` records those checks and is not an original ZIP member.

All seven `identity.json` source fields match the executed merge and classify
native process/TCP-loopback/ephemeral execution. Every `test-counts.json` has
13 targets, **77 passes**, zero failed/ignored/filtered and listed counts equal
to passes. Four Unix rows each have two additional storage passes. Linux's
retained full log contains **73 release and 120 shared-conformance passes**,
with successful Clippy/default guest/analyzer steps in `linux-status.json`.
Scopes overlap and are not unique pooled experiments.

`initial-failure/` preserves the original macOS closure failures and Darwin
witness. `final-attempt-failure/` preserves the later fixture setup failures,
source/run/job/executed-merge metadata and a verified original Intel identity.
They remain failed attempts, not final acceptance. `fixture-timeout-order.diff`
changes only two test setup positions; final production code is unchanged.

These component/client rows do not establish native MVM backends, a durable
Windows gateway, canonical WAN or fleet capacity. The local macOS 27 developer
TCP run is separately recorded under `validation/`, not substituted for hosted
macOS 26 qualification.
