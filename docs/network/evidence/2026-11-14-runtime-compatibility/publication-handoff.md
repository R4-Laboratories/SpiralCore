# Guest-runtime publication handoff

The project owner confirmed on 2026-11-14 that there is no approved contributor
publication process for an accepted signed MVM guest runtime; **Ari must publish
it**. The complete installed MVM acceptance remains open until that authoritative
artifact is available. Independent host/endpoint/recovery work continues.

## Reproducible input for review

The clean unsigned producer witness uses contributor MVM
`9394d74f26d13424afeaade1ace9ece0f7a29dd8`, with upstream base
`50b1a239b75855150fa7fc2fd185b202c733ccdd`. The later host-only unused-preparation
fix is published separately at `0909b7ec11942abbc48c47229e27533ba170bbcb`.
The [build report](guest-build.md) and [manifest/declaration receipt](guest-build.json)
identify both architectures, 76 members, tool pins and all artifact hashes.
Both architectures declare format 1, an 8,192-byte boot bound, directed journal
v1, object pair v1 and sender ordinal v1. This is not proof of live guest behavior.

The unsigned archive is 22,536,289 bytes, SHA-256
`d596b947bf9e91f50ea73a1e366b8b0226b157fed7580cbeaec41df3ceb33759`.
It is retained on Wick with its checksum, manifest and original canonical reader.
The first dirty-source artifact is retained separately. The build is reproducible
from the public source with MVM's existing `guest-bins` producer; no private key
or operator packet is needed to review or rebuild it.

## Maintainer-owned publication

Ari must select the approved upstream source and a fresh candidate version, and
use MVM's existing guest-bins/release publication and authentication process.
The old signed rc.3 artifact and its version remain immutable. The unsigned Wick
archive must not be patched, given invented official provenance or admitted by
weakening the runtime verifier. A maintainer-selected source/version can change
the resulting artifact hashes and must be recorded independently.

A fresh candidate version also requires a consumer built for that expected
runtime version; rebase the reviewed changes onto Ari's selected source and rerun
the required checks. The rc.3 host build is not automatically a consumer of a
new-version artifact.

The consumer needs host code that understands the authenticated compatibility
metadata and enforces its semantic and exact-length checks. Older closed readers
may refuse the new declaration; absence retains the legacy 2,048-byte/no-new-
extension profile. Matching a version label or valid signature alone is not
host/guest compatibility.

After publication, acquire the actual archive/checksum and accepted signature
bundles through the canonical verifier. Recreate the signed synthetic workload
packages through the approved producer because their boot closure pins the exact
guest-runtime artifacts. Replacing a cache file does not update an immutable
signed package.

Then execute feature-specific guest controls and the original two-guest
transfer → destination admission → application ACK/readback → original-owner
retirement path. Record both directions' actual bytes, CPU, elapsed time,
source/artifact identities and retained resource state. Continue through lost-
receipt recovery and sustained sessions before broader platform/WAN acceptance.
No signing keys or tokens are requested. Publication does not by itself establish
CUI/FIPS or the final networking submission's qualification.
