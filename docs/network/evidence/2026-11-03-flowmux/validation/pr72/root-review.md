# Native Mac TCP timeout correction: independent source review

Root reviewed the retained reviewer's two-file delta against immutable b24a5d0c.
CODE/security PASS for that delta. tcp.rs SHA256
c51849d231ab343fb037f579fcf830c18bffe2f53b62fb3f9a0149856a9ad7c9;
paired_tcp.rs SHA256
35e6ec3fe280b8a39e40331cb26614e14a434a0c0fcfc412279eada2f00be7ed.

The carrier sets an actual bounded20ms-or-smaller timeout before traffic and
cloning. Only Darwin raw EINVAL22 can preserve an operation after failed timeout
setting, and only with observed nonzero existing timeout no longer than the new
original-deadline slice. None, zero, excessive timeout, failed readback and all
other errors remain refusals. No deadline is renewed. The TLS carrier uses bounded
nonblocking internal deadlines rather than this setsockopt path, so its timeout
setter has no matching OS error to suppress. Authority, encoding, frames, key
pins and cache partitions have no change.

The public original reverse-half-close test and an independently stronger joined
peer/buffered exact response test both reproduced EINVAL before correction.
The isolated immutable31-test source plus exactly these two overlays passes33
native Mac component tests and strict all-target Clippy. Hosted Mac originals,
semantic REDs and prerequisite failures retain distinct labels. These are
ordinary-process TCP components, not native MVM or WAN acceptance. Fresh hosted
native matrix, source freeze and the remaining canonical/resource gates are
required before release.
