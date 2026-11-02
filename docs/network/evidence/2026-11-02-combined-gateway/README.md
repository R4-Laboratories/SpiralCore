# Combined gateway evidence

Start with the [results report](../../combined-gateway-results-2026-11-02.md)
and [service guide](../../runtime-object-gateway.md).

- [Registration](network-registration.json): exact arms, dose, setup, capture and
  calibration gates. Measured source `7e698aefe88f3729bc1b6e4f7ca72d9f036c110d`.
- [Original compact Linux receipts](linux-run): 24 sessions, 336 operations,
  288 deliveries, exact reopened bytes and complete captures.
- [Analysis](analysis.json), [method](analysis-method.md) and
  [independent review](review-analysis-independent.md): every trial, operation,
  paid cumulative prefix and matched comparison, including losses.
- [Full archive](combined-run-41394672925-archive.json): original ZIP with PCAPs
  and executables, immutable private HF revision, fresh-download verification.
  [Artifact](verified-manifest.txt), [source](verified-source.txt) and
  [binary](verified-binaries.txt) identities checked independently.
- [TUN compatibility archive](quick-regression-41394672864-archive.json):
  12 sessions and 48 exact application flows on the same source.
- [Implementation gate](implementation-gate.md), formal RED/GREEN outputs,
  [core review](review-core-gateway.md), [gateway review](review-gateway.md)
  and [infrastructure review](review-infra-parent.md).

Main run: `41394672925`; artifact: `13119520976`. Original ZIP SHA-256:
`258ffa5d58fd8b9957285e020a5d6128179725a996bbd11348c2508adfc065a3`.
All 1,029 artifact entries, 162 source entries and three frozen executables
verified. The workflow blob matches the measured source. The historical analysis
was also reproduced byte-for-byte before launch. Formal raw outputs retain their
original formatting; full PCAPs/frozen binaries are omitted from the Git copy
and retained in the archived original ZIP.

This is one Linux runner with real QUIC and calibrated local impairment, not a
new Internet WAN run. CPU is measured; energy is not. Combined is an explicit
cooperating-object profile and does not transparently transform the TUN path.
