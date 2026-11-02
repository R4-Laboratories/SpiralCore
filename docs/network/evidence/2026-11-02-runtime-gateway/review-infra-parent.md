# Independent runtime gateway infrastructure review

Reviewer: `/root`, independently reviewing `/root/selector_finish`'s fixture,
workflow and namespace harness. Active infra-reviewer eligibility checked against
roster `d1927d9e7ce51354`. Source review passes; execution remains a separate gate.

The complete harness and workflow were inspected before launch. Same-repository
PR source, read-only permissions, pinned actions, frozen executable/source hashes,
single attempt, 40-minute job/20-minute campaign bounds and a 4 GiB disk floor
constrain the run. The services run as the unprivileged runner after entering
owned namespaces; only namespace routes and router forwarding are changed. Fresh
private keys, large synthetic inputs and stores stay outside the artifact tree.
Cleanup signals only owned namespace processes and removes only its fresh private
directory. Public failure evidence is retained.

Both router egresses are impaired. Calibration measures RTT and receiver goodput
in both directions before capture; qdiscs reset before each arm. The retained
Ubuntu qdisc JSON from the previous campaign confirms the new checks' field
names and units (0.04 seconds, 1,250,000 bytes/second). Capture starts before
authentication and ends after service finish and close/drain. Zero capture loss,
queue drain, unchanged host routes and frozen binaries are checked. Finite-queue
drops remain measured costs. This is real QUIC traffic on one shaped Linux host,
not an Internet WAN or physical-energy measurement.

Fixture generation delegates to the existing canonical runtime, with exact
registered sizes, deterministic 512-byte edits and matching identities across
arms. Repeated paths reuse bytes instead of duplicating files. The amended
opaque-edit case isolates rolling-cache behavior independently of trace
compressibility. Setup, checkpoint/schema/base publication and prediction replay
are charged. All arms receive the same target-cache probe opportunity.
Qualification requires actual RuntimeV1 and SparseV1 execution on their intended
non-cache cases, exact reopened object bytes, and matching channel counters.
Offered modes on cache hits are not attributed to decoder execution.

One pre-launch finding was corrected: the new workflow initially copied the old
packet workflow's path filters. It now watches object/core/transport/fixture and
relevant dependency changes. Narrowing the old long packet campaign avoids a
duplicate research run; the existing quick gateway regression still covers
shared transport changes. Static positive/negative registration checks are
retained, and the new fixture amendment still requires its final integrated
GREEN receipt before launch.

Limits: three descriptive matched trials with rotated orders, no tail-latency
qualification; per-object logical bytes differ from full-session captured bytes;
receiver handler CPU is a subset of whole-process CPU. Preparation and seed
costs must appear in cumulative results. No performance victory is required for
acceptance, and no measured result is asserted in this source review.
