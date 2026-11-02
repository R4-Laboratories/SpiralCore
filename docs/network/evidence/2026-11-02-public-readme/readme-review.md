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

Review corrections requested before final publication:

1. Identify the packet table as the primary IPv4 shaped-link subset; the full
   32-session campaign includes unshaped runs and IPv6 smokes.
2. Identify the object table as the shaped-link subset; the 288-delivery campaign
   spans both local and shaped links.
3. Label the quickstart demo as fixed-policy loopback with no peer authentication.
4. Render the long LR Mermaid diagrams before merge; compact grouping may be
   needed for legible default-width presentation. Include the phase-memory update
   alongside Kuramoto in the conceptual runtime node.

Public-download contents and credentials are independently reviewed in
`artifact-review.md` and the release packaging records. Publication URLs and
rendered diagram checks require the parent execution's final verification.
