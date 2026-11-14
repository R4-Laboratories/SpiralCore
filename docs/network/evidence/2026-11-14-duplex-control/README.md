# Default-feature simultaneous and alternating direction control

This development control investigates the Windows deadline failures holding
PR #260. The retained ARM failure at source `515b87bb` records a sender waiting
for a kind-3 reply while both scopes remain valid. Its receiver thread also
handles incoming storage work synchronously, so delayed local reply routing is
a hypothesis; the log does not prove that the remote endpoint withheld a reply.

The same ordered-stream fixture now accepts the test-only environment selector
`SPIRALCORE_ORDERED_LAYOUT=simultaneous` or `alternating`. Absence retains the
simultaneous workload. Unknown values refuse before endpoint setup. Production
library code, encoding, authority and wire meanings are unchanged.

Both modes retain 4,096 distinct 16-KiB blocks in each direction (64 MiB each),
the same exact byte comparisons, two-second operation deadlines, 240-second
workload deadline, 270-second enrollment and retention bounds. Alternating mode
uses two capacity-one integer-token channels to allow one direction's block at
a time. Exact token ordinals are checked. No extra worker is spawned, token
receive uses the original workload deadline, and token publication refuses a
full or disconnected channel instead of waiting. Dropping a failed writer drops
its original channel ends so its peer cannot wait forever. This control state
belongs to the fixture, outside the product's SmallV1 accounting.

The manually requested `duplex_control` timing workflow builds one default-
feature executable per native Windows runner, executes both modes with separate
fresh endpoint stores, records both outcomes even when one fails, and checks the
same executable digest before and after. The existing instrumented timing jobs
are skipped for this explicit control request. The paired order is determined
by run identity and matrix row and recorded in the results. Source, tree, lock,
OS/architecture, raw result lines and checksums are retained in compact artifacts.

This is a diagnostic comparison, not an optimization or acceptance claim. If
only simultaneous traffic fails, that discriminates concurrency dependence but
does not by itself prove a specific storage stage or reply-routing cause. No
root cause, speedup, WAN result, native MVM backend or supported capacity is
claimed before execution and analysis. PR #260 remains held.
