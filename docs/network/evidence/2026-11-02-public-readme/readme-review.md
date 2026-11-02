# Scoped README, mechanism and license-scope review

Reviewed the rewritten main README, LICENSE, LICENSING.md, CONTRIBUTING.md,
PROVENANCE addition and packet-gateway README correction against the current
source and the linked packet, Combined object, MVM and historical WAN reports.
Roster verification passed for `code-reviewer`.

The numerical tables and stated paired reductions match their source reports.
The README distinguishes captured IP bytes, logical encrypted mailbox bytes and
HTTP-body bytes; sender CPU, endpoint process CPU and gateway CPU are not pooled.
MVM setup and separate lifecycle/reboot scope are explicit. Reported schema and
fast-local-link losses remain visible. Current gateway shaping is distinguished
from the historical two-host HTTP WAN and from local native guest/vsock runs.

The three architecture paths agree with the code: reliable authenticated object
QUIC; independent packet framing through TUN/QUIC datagrams; and plaintext object
encoding before canonical MVM envelope encryption with a keyless host relay.
Experimental mailbox port5257 remains distinct from FlowMux5253. The diagrams
do not promise transparent CA/warm reuse for arbitrary encrypted Internet traffic.
The deterministic runtime description retains logical steps and bounded built-in
transitions; whole-object verification, not ternary averaging, enforces exactness.

The root MIT text and scope implement the owner's explicit original-contribution
license choice. Existing UOR/MVM notices and supplied/historical third-party
research are explicitly preserved/excluded. No blanket rights in those imported
materials are asserted. This review checks consistency with the requested scope;
it is not an independent adjudication of third-party ownership.

Review corrections resolved in the final source-level pass:

1. The packet table explicitly identifies the primary IPv4 shaped-link subset;
   the full 32-session campaign includes unshaped runs and IPv6 smokes.
2. The object table explicitly identifies the shaped-link subset; the
   288-delivery campaign spans both local and shaped links.
3. The quickstart explicitly identifies its synthetic loopback harness as
   unauthenticated and distinguishes the documented gateway transports.
4. All four Mermaid diagrams have compacted paths. The conceptual runtime node
   includes memory updates alongside Kuramoto and cellular automata. Their
   source-level architecture remains consistent; final GitHub rendering and
   default-width legibility are reserved for the publication check.

The public artifact catalog agrees with the publication manifest and
redistribution note on all four package names and measured source revisions.
It distinguishes new public archive hashes from original preservation hashes,
states that nine executables are omitted, and explains historical manifests
that still refer to those files. The packages are described as evidence subsets,
not complete executable distributions. Private Hugging Face archival references
are labeled private; the older 7.7 GB WAN preservation archive is explicitly
outside this publication. Public test certificates and possible short encrypted
QUIC prefixes are described without claiming payload-free packet captures.

No remaining source-level documentation or license-scope corrections were found.
This review does not independently repeat archive credential/extraction checks
or assert that release assets have already been downloaded successfully.

Public-download contents and credentials are independently reviewed in
`artifact-review.md` and the release packaging records. Publication URLs and
rendered diagram checks require the parent execution's final verification.

## Illustration follow-up

Visually inspected all three generated PNGs in `docs/figures/overview/` and
reviewed their README captions and generation provenance. No blocking claim
corrections found. The object illustration shows three children per ternary
symbol and reusable state plus residual transmission; the caption explicitly
identifies the instruments as conceptual software, not hardware or runtime
screenshots. Illustrated tile, phase and CA patterns are visual metaphors, not
byte-level test fixtures or measurement evidence.

The normal-network cutaway depicts application/TUN/gateway/NIC ordering and
authenticated QUIC between cooperating gateways. Its caption expressly excludes
an add-in hardware requirement and current cross-packet warm-cache/CA behavior.
The MVM cutaway keeps encryption/decryption within separate guest boundaries and
sealed envelopes at the host relay. Its caption scopes the picture to the local
two-guest witness, with no two-host WAN qualification claim. None of the images
asserts a performance ratio, physical clock synchronization, a cryptographic
certification or production readiness. Existing measured results and their
qualifications remain unchanged; collapsed Mermaid sections retain precise
technical paths beside the conceptual artwork.

## OSI illustration follow-up

Visually inspected `docs/figures/overview/osi-model.png` and its README caption.
The diagram correctly distinguishes functional OSI terminology from implemented
protocol stacks: codecs/reconstruction and transfer state/acknowledgments remain
application-level functions, not independent Layer 5/6 protocols. The object
service uses reliable QUIC streams; the packet tunnel encapsulates inner IP
packets, obtained through Layer 3 TUN, in raw/zstd framing and QUIC datagrams over
outer UDP/IP. Ordinary NIC/link layers remain underneath those network paths.
The MVM panel separately places mailbox/object codecs inside guests and
virtio-vsock between guests and the keyless host relay; it does not imply that
vsock runs over the illustrated IP stack or that a mailbox is a standardized
OSI Layer 5 interface. The caption makes those boundaries explicit. No blocking
claim corrections found; this was a focused illustration review, not a new
implementation audit or runtime qualification.
