# Sustained installed Firecracker mailbox recovery after tracker expiry

One signed Linux x86-64 Firecracker Dev guest ran three distinct 3 MiB
Raw objects over the original installed mailbox service. After 1,540
completed Calls, the second object's Finish Reply was admitted by the
guest and its receipt was withheld before sending. The guest closed the
first carrier at a frame boundary and reauthenticated through the
original enrolled V2 acceptor. The host sent one exact cached encrypted
Reply, and the guest sent the original prepared receipt. No Call was
resent. Both sides completed 2,311 Calls in the same session and ordering
scope.

The guest sampled its original tracker before issuing the selected
Finish. The host sampled after sending the original Reply and before
accepting the replacement. Prior completed Calls minus retained entries
is a conservative lower bound on tracker expiry. The samples do not
measure deletion in other service or identity stores.

| Measurement | Guest | Host |
|---|---:|---:|
| Elapsed at pre-redial sample | 385.139 s | 385.184 s |
| Retained tracker entries at sample | 1,299 | 1,294 |
| Expired tracker entries before redial, at least | 241 | 246 |
| Original carrier bytes sent | 27,449,664 | 15,039,121 |
| Replacement carrier bytes sent | 13,729,495 | 7,520,032 |
| Total carrier bytes sent | 41,179,159 | 22,559,153 |
| Original carrier application frames sent | 3,081 | 3,082 |
| Replacement carrier application frames sent | 1,541 | 1,541 |
| Campaign CPU counter | 11.416 s | 9.452 s |

The directional envelope and framing counts reconcile, including both
authentications. The run used 63,738,312 carrier bytes for 9 MiB of
synthetic logical payload. The guest CPU counter covers campaign setup
and operations after local authentication. The host campaign counter
covers setup and post-frame work, excluding idle receive waits and the
explicit redial-cut sample. The host witness process separately used
10.699 s of CPU. The supervised lifecycle took 580.558 s, including
four-Calls-per-second pacing, and outer peak RSS was 186,736 KiB.

After the original owner retired the guest, the host reopened and
byte-compared all 9,437,184 bytes, removed the private store and verified
its absence. Guest and staging reservations returned to zero, and no
Firecracker process remained. The existing signed lifetimes, quotas
and eight-second replacement cap covered this qualification dose.
The pinned host and guest builds and ten focused checks passed. The
prior protected binary refused the combined mode before launch. The
first successor build's borrow error and the successful successor
preflight are recorded in validation.txt.

Apply the delta to the source of the
[short installed Finish redial](../2026-11-05-installed-object-finish-redial/README.md).
The [uninterrupted sustained run](../2026-11-05-installed-sustained-campaign/README.md)
remains a separate witness. These are paced local Dev results. Partial
receipt writes, multi-tenant concurrency, production capacity,
cross-platform WAN, and a validated security profile remain open.
