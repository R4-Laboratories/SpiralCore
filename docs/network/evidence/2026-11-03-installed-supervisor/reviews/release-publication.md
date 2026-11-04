# PR71 compact qualification publication review

**PASS for the corrected docs/evidence publication.** Source is merged PR71
`7e7479835a3150125b903bacc55b28bf765fbd28`, tree
`368679d28580e460ed69ede5d43506e702a4802e`, equal to reviewed head922414 and the
recorded tested synthetic merge. Later canonical lifetime/parent/helper work is
excluded. This confirmation reuses final-922414.md and its independence boundaries;
it performs no build, VM, measurement or original-artifact redownload.

Checked the release README, parent README qualification link, custody.json,
portable artifact metadata, archived storage count records, Linux log, coverage
summary and archive inventory structure. All74 SHA256SUMS entries verify locally
after the count correction. Final release README SHA-256:
`f9c817b37af1f373ab47fd9e1d5330723e8d36491ae63257706f8018d5f81ac5`;
manifest SHA-256:
`5ffc3479edcb1a1b4b5cfec0687aa50e1884a06269322d7fc1995a54a93edbc6`.

The initial table conflated storage scopes. Corrected wording now matches actual
records: each Unix row has3 private storage and21 endpoint tests; each Windows
row has4 outer private storage,20 endpoint and2 isolated primitive tests. All
have nonzero enumeration; the two manually gated workflows remain SKIPPED.
The archived Linux output independently sums to142 live checks in12 test-result
entries and138 shared checks in23 entries. Zero-target entries are not additional
coverage, and these scopes are not added into an independent result total.

Custody records seven original ZIP identities and42 inner hashes per row =294;
Windows CRLF versus Unix LF identity is explicit. The initial wrong-manifest and
newline-verifier failures remain disclosed; no source/test changed to pass them.
This publication retains compressed derivative portable records and original JSON
selections, complete original member inventories, and the Linux log, while making
clear that these are not the original artifact ZIPs. The lead's original ZIP/hash
verification is reused rather than silently re-described as this reviewer's work.
Relative links to independent review, object-finish packet and results report
resolve within the published documentation layout.

Linux coverage remains exactly1002/1179=84.99%, broader2302/2703, frozen f1 source
scope, host.rs UNMAPPED. Later Windows/macOS/QUIC deltas retain separate source
and execution evidence. The65 LLVM mismatch warnings are disclosed and retained;
no branch or cross-platform percentage is invented. Instrumented native runs
remain coverage evidence, separate from optimized timing observations.

Historical capture, terminal-ACK, parallel-fork and prerequisite failures remain
retained. Later successful checks do not explain away the earlier capture gap.
The publication does not expand native Windows storage/client CI into Windows
MVM hosting, HVF transfer into death/recovery, local pairs into WAN, transport
limits into aggregate plaintext/cache ownership, or component success into full
B6a/B6b/C8/scale completion. Pending canonical endpoint/parent ownership, retained
tail/long-lived session, fairness/saturation and WAN gates stay explicit. No new
performance, energy, power-loss or certification claim is introduced.
