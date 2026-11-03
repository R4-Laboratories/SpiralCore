# Firecracker host-lease helper: security source review

**HOLD at MVM 570cb8f pending the marker-inspection correction below.**
Scope is the isolated helper only, before driver/daemon integration. No source
edits, builds, tests, remote operations or unrelated audit by this reviewer.
Required MVM ponytail/rust-skills catalog was unavailable; neither was invoked.

Isolated source: `/Users/casey.allard/spiralcore/local/mvm-host-lifetime-lease/source`,
based on `ae12e9deb41cc9a3264196ce22ebc72d4042883d`.

- `crates/mvm-backends/src/driver/fc/host_lease.rs` SHA-256:
  `b50e72efe3bf1e60f1262d9d41b606e250cd1e962f50b23d5153ab170d2f1c91`
- `host_lease/tests.rs` SHA-256:
  `ca45602497c59d4060c72e3d1f1867ec5a2bccb7de81a2726546b538bc5a29d4`

## Finding and required correction

**MEDIUM: a failed forbidden-marker inspection can be treated as absence.**
The rendered `[[ ! -e path && ! -L path ]]` checks cannot distinguish ENOENT
from stat/lstat EIO or access failure. They can therefore accept an unreadable
intent/temp marker while old expected Active bytes still match. This is weaker
than the owner's rule that only confirmed absence permits progress, notably
for a delayed launcher after a failed retirement. No demonstrated overlapping
resource admission or end-to-end exploit is claimed for this unintegrated code.

Use a checked directory scan through the held directory: fixed `/usr/bin/find
-H`, depth1, the two validated names, bounded first-match output (`-print -quit`).
Require command exit0 and empty output; inspection failure must refuse. Keep it
inside the existing timeout and add find to the fixed-tool contract. Dangling
markers must still refuse. Lead accepted this correction; source/test rerun
and final exact hash are pending. Do not describe an unexecuted EIO injection
as covered by the existing marker fixtures.

## Other reviewed helper boundaries

The trusted expectation admits only bounded distinct ASCII basenames, bounded
state length and an opaque expected digest; no executable/tool path, shell
fragment or callback is accepted. Private descriptor/type/UID/mode validation,
CLOEXEC and nonblocking SH operate on the original supplied descriptions.
The fixed script verifies actual reopened object IDs, independently acquires
SH, checks bounded current state, and closes its directory descriptor afterward.
The timeout wraps preflight only. The launcher retains the holder description
shared with timeout/bash/flock; no explicit unlock or target-wide timer exists.
Integer/hex literals and restricted names do not introduce shell interpolation.

Read retained absence RED and `first-linux.txt`: 5 serial PASS (4 behavior tests
plus one child entry), no ignored/filtered cases. The real fake-exec process
retains kernel SH after parent lease drop, and final target exit releases it.
Those records precede the correction and establish no unreadable-marker EIO
qualification. Missing tools/timeout fault execution, actual sudo/Firecracker
handoff and native process custody remain separate gates.

This helper supplies no canonical launch authority and changes no MVM wire,
NIC path, worker ceiling or signed grant. A helper PASS after correction will
not approve driver/daemon composition, idle-pool exclusion, diskless/verifier
coverage, actual VMM lease lifetime, full B6a or host-scope accounting.

## Final correction delta — af9662b

**PASS for the helper only; resolves the marker finding above.** Exact helper
SHA-256 is `b4137f464212c6c5e49124cb7a741ccbc951dd6ce25d0359e80719dfa1940ec5`;
test source remains `ca45602497c59d4060c72e3d1f1867ec5a2bccb7de81a2726546b538bc5a29d4`.
The original HOLD remains retained as the pre-correction verdict.

Source now captures checked `/usr/bin/find -H` output through the held directory,
depth1, matching the two validated basenames with `-print -quit`. A failed
command exits; any returned marker exits, including a dangling marker. A failed
inspection is no longer interpreted as confirmed absence. The fixed-tool
contract includes find. Reopen, metadata, flock, state, scan, hash, timeout and
descriptor-close security guards explicitly exit on failure; the fixed block
does not depend solely on ambient errexit context. The timeout remains confined
to preflight and successful SH custody remains shared with the launcher.

Read `corrected-source.sha256`, `corrected-linux.txt` and exit0: the exact current
five serial entries passed (four behaviors and one child helper), with no
ignored/filtered entries. These are existing semantic fixtures; actual marker
EIO execution, missing-tool and timeout fault injection remain unrun. No tests,
builds or source edits were performed by this reviewer. This delta approves no
driver/daemon connection or actual Firecracker/native recovery claim.
