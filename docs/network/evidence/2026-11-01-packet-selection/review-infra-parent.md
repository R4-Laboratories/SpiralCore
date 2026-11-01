# Independent campaign infrastructure review

Reviewer: `/root`, independent of `/root/selector_finish`'s workflow and shell
implementation. Active infra-reviewer eligibility verified against roster
`d1927d9e7ce51354`. Reviewed the complete new workflow and campaign harness,
registration/contract, static gate fixtures, and congestion amendment before
launch. Scoped PASS; no remaining launch blocker in this infrastructure review.

The workflow checks out the exact same-repository PR head, uses read-only token
permissions, pinned actions, bounded job/campaign duration, a 4 GiB disk floor,
and retains failures without automatic retry. Private synthetic keys and fixture
data are outside the public evidence tree. It builds and tests before timing and
checks source/executable identity. No paid infrastructure or user host route
changes are involved.

The harness confines routes and forwarding to three owned namespaces. The router
applies impairment independently in both directions. Every shaped session has
measured RTT and bidirectional receiver-rate admission before traffic, with qdisc
counters reset afterward. Capture starts before QUIC authentication and includes
closure; zero capture loss is required. Finite queue drops and reassembly expiry
are retained as congestion costs, while malformed/denied/capacity failures still
fail admission. Queue drain, exact application bytes, fixture hashes, selector
counter identities, host route preservation and executable hashes are checked.

The persistent TCP sequence uses the same flow key for its three objects.
Application times are per object, but gateway CPU and capture bytes are per mixed
session. The deferral repair snapshots each interval before emitting any JSON;
the parent inspected this source and read its focused three-test GREEN receipt
and successful final strict Clippy output. Separate source reviewers provide the
cross-author code/security closure.

Limits: three descriptive primary trials, not latency-tail qualification; one
host running real TUN/QUIC traffic under netem, not a real Internet WAN; capture at
one underlay boundary, not physical Ethernet energy or line-symbol accounting.
Capacity exhaustion remains a resource-limit failure and must not be hidden by
the congestion amendment. Full runtime calibration/exactness and resulting
performance arithmetic are pending the source-pinned execution.
