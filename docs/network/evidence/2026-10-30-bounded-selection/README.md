# SC-OPT-016 bounded first-block selection evidence

This packet records a source-qualified rejection probe that avoids fully encoding
a losing Whole-zstd candidate while preserving the existing byte-minimum choice.
See the [result report](../../bounded-selection-results-2026-10-30.md) and the
unchanged [registered measurement contract](measurement-contract.md).

- Design/plan commit: `e90d9eadd46496c48fce4a2490b2449e9d2fe0ab`.
- Qualified implementation/native measurement: `eccb104d8d5dbdf56ad2fdb37a920b0424edbceb`.
- WAN source: `8e4b87329ea2a72ad5a408e39869bb36285befe8`; compiled source/workflow
  hashes match the native qualification.
- Baseline production overlay: exactly two source files from
  `4234591e25b4ca87db6a4670bdf407f48a581575`, common current harness, probe disabled.
- `baseline-local/`: preimplementation 36-transfer context, not the paired gate.
- `qualification/`: actual RED/GREEN and failed qualification receipts, release
  tests, Clippy, coverage, frozen builds and source restoration, procedural and
  harness/workflow reviews. `content-review.json` covers production independently.
- `local-comparison/`: two five-trial stores, 120 seed/update transfers, complete
  raw timings/receipts and exact stopped-store verification before cache removal.
- `compressible-control/`: twenty prepare-only samples, actual separately charged
  setup/seed, exact sending/readback outside preparation timing, forty exact objects.
- `local-review.json`: independent raw arithmetic and local advancement review.
- `wan/`: six registered plans, receiver observations, hosted sender evidence,
  stopped-store exact verification and artifact custody/cleanup records.

The local ten-pair small-update sender-CPU median ratio is 0.6000621615489989,
passing the registered 0.80 gate. The compressible fallback control is slower:
median paired preparation-CPU ratio 1.1014885623189048. Neither result is omitted.
Local controls do not substitute for WAN measurements.

The exact executable command interfaces, environment variables, scope limits and
timing definitions are in the contract. `qualification/freeze.sh` records native
baseline/candidate construction; the pinned workflow independently builds both
Linux senders. The receiver uses the native hardware-SHA feature and the sender
uses Linux default features, matched across every arm on each host. Summaries can
be regenerated with the frozen Rust example; output paths must be fresh.

Bytes are bidirectional HTTP bodies, not full-path packet bytes. CPU is sender
process plus disjoint receiver-handler CPU, excluding relay and receiver socket/
logging work. Exchange elapsed is nested, not pure wire time. Fitting/context,
seeding and update costs are reported individually and as actual paid lifecycles.
The corpus is two localized-edit object sizes, not a general compression result.

Formal raw evidence is intentionally retained in Git for later review. Generated
object stores are removed only after exact readback and receipt custody. Frozen
executables remain outside Git and in the hosted artifact for its retention period.
`SHA256SUMS` covers this packet excluding that checksum index itself.

First-attempt [run 41153084110](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41153084110)
completed all 360 transfers with 360 exact stopped-store readbacks, 120 actual
fits, sixty old/new winning-frame pairs and no discarded observations.
[Summary](wan/summary.json) retains all distributions. Small-target median paired
new/old sender CPU is 0.720097314628596 (30/30 wins); completion is
0.9103506845896778 (20/30 wins). Small paid-lifecycle completion versus old and
CPU versus Whole remain negative by their paired medians.

[Custody](wan/custody.json) verifies the downloaded ZIP against GitHub's digest,
59 artifact members, 91 frozen source hashes and six identical cross-platform
plans. [Cleanup](wan/cleanup.json) records twelve stopped owned processes, six
removed verified stores (3,060 MiB payload), and removal of the temporary
capability, one GitHub secret and nine variables. [Final review](final-review.json)
checks the raw arithmetic and report without rerunning the experiment.
