# Runtime and warm gateway evidence

Start with the [results report](../../runtime-gateway-results-2026-11-02.md) and
[runnable service guide](../../runtime-object-gateway.md).

- [Registration](network-registration.json): workload, setup costs, three paired
  trials, capture/calibration gates and the diagnosed capture correction.
- [Compact original Linux receipts](linux-run): 24 sessions, 312 operations,
  288 exact deliveries, calibrated local/shaped links and complete captures.
- [Analysis](analysis.json), [method](analysis-method.md), and
  [independent arithmetic review](review-analysis-parent.md): all trials,
  logical operation bytes, captured session bytes, scoped CPU, measured elapsed
  time and paid setup/cumulative comparisons. The standalone Rust analyzer is
  under `tools/gateway-network/runtime-analysis`.
- [Successful full archive](runtime-run-41373931230-archive.json): immutable
  private Hugging Face reference, original ZIP hash, fresh redownload check,
  and source/executable custody. Full PCAPs and executables are in that ZIP;
  they are intentionally omitted from the compact Git copy.
- [Failed first run](failed-run-41373268385/QUALIFICATION.md) and its
  [complete archive](failed-run-41373268385-archive.json): exact Raw transfers
  followed by a seven-packet capture mismatch; excluded from performance
  comparisons, never discarded or relabeled as passed.
- [TUN regression archive](quick-regression-41373931002-archive.json): 12 paired
  gateway sessions and 48 exact application flows on the same corrected source.
- [Implementation gate](implementation-gate.md), focused RED/GREEN receipts,
  [core/service review](review-core-service.md),
  [transport/repair closure](review-service-transport.md), and
  [infrastructure review](review-infra-parent.md).

Measured source: `d5f35a34c3c2c63c18deaf84739d10c43da7d2b4`.
Successful run: `41373931230`; artifact: `13110193702`.
Original ZIP SHA-256:
`92e83030cf057f718ec6e605148f2b020f052fd0bfabd1f3af4fb2622c6c6f81`.
All 1,021 artifact entries, 156 source entries and three frozen executables
verified. The executed workflow blob matches the measured source. Formal raw
receipts retain their original formatting, including trailing blank lines.

These results use one Linux host with real QUIC traffic and calibrated network
impairment. Runtime and Warm are separate cooperating-object profiles. They do
not establish a real Internet WAN result, automatic TUN compression by these
mechanisms, or physical-energy savings.
