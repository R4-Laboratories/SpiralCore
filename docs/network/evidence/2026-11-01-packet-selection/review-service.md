# Scoped service and persistent-probe review

Reviewer: `/root/mvm_boundary`, independent of the original service/probe
changes. Scope is the root-owned service profile/counter plumbing, persistent TCP
application probe and their tests. The packet codec authored by this reviewer is
excluded. Review date: 2026-11-01.

Roster `d1927d9e7ce51354` was verified in the active worktree: 13 agents and 14
skills. `code-reviewer`, `security-reviewer`, `rust-reviewer` and subsequently
`debugger` each returned active. Their role files were read. The file-claim table
has no conflicting in-progress claim for this scope. No new build, dependency
audit or coverage campaign was started for the review; the focused regression
below was authorized when the review uncovered a concrete timing defect.

## Finding and repair

**MEDIUM, measurement integrity — fixed, independent repair review requested.**
The original new `tcp_sequence` loop called `report` after every exchange.
Although `report` stopped that object's timer first, it then hashed the entire
fixture and wrote JSON before starting the next object. The opposite peer could
already be timing its next exchange, so report CPU/I/O could spill into the next
measured transfer. The original probe source at finding time was
`0116fc6827f8d8f9f80694d83826688799afecb167b056a5461f55d6cd59a380`.

The authorized narrow repair captures wall/CPU values immediately after each
successful exchange, retains only three timing/data-reference tuples, and emits
all reports after the sequence. No fixture bytes are copied by the deferral.
The existing exact byte comparison and returned digest checks remain inside
each timed exchange. The root also corrected the general timing description to
follow `detail.connect_included`, since only the first client object includes
connection establishment.

`probe-deferral-red.txt` / `.exit` retain the observed behavioral failure
(`report I/O ran between timed exchanges`, exit 101). After the repair,
`probe-deferral-green.txt` / `.exit` retain all three passing probe tests (exit 0):
ordinary TCP/UDP exactness, three exact objects over one TCP connection, and no
report files before the third exchange completes. The repair author does not
claim an independent review of this repair; `/root/mvm_tests` owns that closure.

## Other reviewed behavior

- Profile validation and dispatch agree on exactly `raw`, `zstd`, `fast-zstd`
  and `adaptive`; unknown profiles fail. Existing configuration is not silently
  changed to the experimental selector.
- New encoder statistics are numeric observations. They do not bypass peer
  authentication or either directional IP authorization check. Encoded-packet
  counters are distinct from successfully sent framed bytes and actual UDP
  counters; a partial transport shutdown is not represented as delivered data.
- The persistent probe opens one socket, exchanges an explicit expected length
  for each ordered object, compares every received byte with its fixture, and
  checks a SHA-256 receipt before client success. One listener accept and one
  client connect establish actual connection reuse. The regression checks stable
  endpoint addresses and opposing client/server identities across all objects.
- First-client timing begins before connection; server timing begins after
  accept. Subsequent timers use the same established connection and stop after
  each exactness exchange. Fixture loading and final reporting remain explicitly
  excluded; these application intervals are not complete process lifecycle or
  isolated network-send time.
- No new unsafe block, secret logging, routing policy change or authentication
  change was introduced in this scope. The plain TCP probe is synthetic test
  traffic inside the separately authenticated gateway experiment, not a new
  production security boundary.

No other actionable source issue was found. Root-owned service/source scope is
**PASS**; the new repair requires the separate cross-author closure. Final native
Clippy/build and the Linux constrained-network runtime/results remain parent-owned
gates, not claims of this review.

## Reviewed file identities

| File | SHA-256 after repair |
|---|---|
| `crates/spiralcore-gateway/src/service.rs` | `fe49db73721e0e311c9c7556d946ca416873154fec8e97247eabb3e5bb6bbb83` |
| `crates/spiralcore-gateway/src/bin/spiralcore-gateway-probe.rs` | `18bdc36a479f798a11631df28d3049c797aa3dad8656301be5e798a690f472f0` |
| `crates/spiralcore-gateway/tests/service.rs` | `f60c28c32d1fe219cd0a5a455f92ff9116da2e276c48202f324d3af02047e503` |
| `crates/spiralcore-gateway/tests/probe.rs` | `95955d8eab689b2bb3eafbdcbf36342d7edb9f972d90f88b1fd81ff1b33f243b` |
