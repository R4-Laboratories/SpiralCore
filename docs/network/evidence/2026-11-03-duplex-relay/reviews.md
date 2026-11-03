# Source, review and qualification identity

Source PR #58 merged at `d997bb06e535b54209f5c2031264aa9573fed426`.
Original native comparison source is `f964c034`; Darwin error-path repair and
final native confirmation use `62cb6042`. Final qualification head is
`fdfbd2f72276589e1c1c8ad8d4908b97f94e9647`: its only crate difference from
`62cb6042` is the TCP test's timeout placement. No production file changes.
The original, final-native and final-qualification manifests each bind 278
tracked crate files; they are separate observations, not substituted originals.

| Final production file | SHA-256 |
|---|---|
| Adapter `src/live.rs` | `53c29a1724776c2cd60a004b59f46cd16a28600eca9e9345f55f03725b39e500` |
| Live `src/orchestrator.rs` | `42b25a95b48a1560db26f6a9122794a23cccc80208b90bd9ff77f8d8338ab57e` |
| Live `src/orchestrator/duplex.rs` | `90c2cc67e7fc40b6a8310cd0511dbaf3cdd1d792f8bb261013edeb6664656e92` |

Independent code/Rust and security reviews approved the scoped EOF and duplex
boundary, then the separate-half shutdown fallback. The fourth TCP transport
fence test and final fixture-only timeout moves were independently reviewed.
The original raw EOF/closure assertions were preserved. Reviewer source checks
covered credit ownership, bounded cloning, start/error/panic cleanup, joins,
half-close, fixed deadlines, counters, route authority and error-text exposure.
Actual thread-start/panic injection and a real 30-second blocked-write timeout
remain unexecuted; the blocked-write test establishes wakeup after partial output
fails on peer closure. No reviewer claimed the old owning transport lacked its
logical fence.

The bounded native runner received an independent infrastructure review against
its registration and existing Rust resource limits. The final runner keeps that
mechanism and reduces the registered dose to two confirmation cells with separate
paths/source/binaries. Independent numeric/evidence review recomputed the original
four-cell table, every ratio and 36/36 publication/readback observations, all
directional/two-hop counters, 49-member custody and cleanup scope. The report
retains increased relay CPU and one-trial/shared-host limitations. The two final
confirmation cells remain separate measurements with 18/18 observations and a
27-member archive, rather than a relabeled paired comparison.
A final independent docs/evidence review recomputed the two final rows,
41.73%/34.75%/28.25% reductions, all 18 final readbacks and hosted CI totals.
It approved README/report/plan consistency and open-gate/custody wording; the
root's archive digest/member verification remains a separately recorded check.

Coverage was independently collected in isolated per-binary exports. Initial
shared-target executable paths were overwritten by normal reuse; original
observed exports/profiles/hash receipts remain, with that custody limit explicit.
Final binaries were hash-verified against their execution/export identities,
copied/byte-compared and protected remotely before further target reuse. Normal
host/guest hashes stayed fixed. Branch mapping is unavailable; changed-production
and broader three-file line denominators are distinguished.

| Normal native source | Host SHA-256 | Static guest SHA-256 |
|---|---|---|
| Initial `f964c034` | `1b3e8b00166ca0470b79ddbcb07d30be8fa4234efaffc8d88500dc7554834d50` | `27604d73ad1b7a4ae570d78404efef994ad59990eeab747e1bf14d7631e2204f` |
| Final `62cb6042` | `0b851e9df95cdfb1f7f999035b4e551707fd29ca3744d532041cb2bcadc991e6` | `c9fba260cbbbb16fb18d9b9bf7e0f87df26ceacdb8d5cab037699094c70b5431` |

Final hosted runs 41611048227/41611048527 passed on executed merge
`f997c34897ce444830efd12326526caf309dd573`, with tree
`d8295443e985e6759d184f89d83a640c58327957`, identical to reviewed head `fdfbd2f`.
Seven original archive digests and 236 inner-file hashes verified. Windows CRLF
was normalized only in the checksum parsing stream; original bytes are retained.
There are 77 client checks per row (539), plus eight Unix storage checks; Linux
has 73 release and 120 shared conformance checks. Scopes overlap. Native client
CI is not a native MVM backend, WAN, fleet or security certification.

The literal `/quality-gate` command was unavailable; independent scoped code and
security reviews were used and recorded. The full C8 peer state machine and the
larger active goal remain incomplete. No payload/VM disk was copied to the Mac;
only custody-verified successful owned Wick disks were removed.
