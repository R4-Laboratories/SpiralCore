# First cooperating endpoint scope enrollment

Parent reviewed the authority boundary before implementation: mutual TLS proves
peer key possession, not foreign plan/tenant permission. The first profile uses
an explicit trusted local controller API, not wire-minted admission or general
cross-organization delegated discovery. Its PinnedScopePolicy starts empty and
contains at most32 exact enrollments. Each binds the full ScopeFacts commitment,
full peer SPKI pin, local initiator/acceptor direction, original expiry Instant
and a unique enrollment identity. Policy revocation advances a local generation
and invalidates existing ScopeAuthority handles. Replacement never revives an
older handle. Expired entries can release bounded enrollment capacity.

The initiating trusted MVM controller derives ScopeFacts from its actual admitted
plan and authenticated flow. It explicitly provisions both cooperating ends,
including the fresh TLS session exporter after that session authenticates. The
receiver does not boot/re-admit the foreign plan. It approves only the exact
provisioned facts through the existing shared endpoint::Authority interface.
Unknown scope, wrong peer/direction/tenant/plan/nonce/exporter, expired or revoked
enrollment refuses before object codec allocation. A constructor checks actual
TLS identity/role/exporter, and every mapped I/O entry checks the retained current
enrollment. Shared core object receipt and commit checks remain unchanged.

ScopeFacts contains full policy SHA, plan ID/version/nonce, bundle SHA, tenant,
instance, authenticated FlowMux session/guest key/stream, original requested host
and TCP port, actual admitted upstream address/port/route, ordered initiator and
acceptor SPKI pins, and fresh TLS exporter. Strings are length-prefixed and
bounded256 bytes; byte identities are full arrays and numeric facts use their
actual canonical widths. Their domain-separated commitment binds Context's
application namespace; reserved numeric Context epoch slots are0 with no MVM
epoch authority. No field received from the remote supplies permission by itself.

Native canonical factory activation, authenticated exchange/materialization of
these scoped facts, production controller enrollment/revocation, general remote
peer discovery/delegation, performance and real-WAN qualification remain open.
