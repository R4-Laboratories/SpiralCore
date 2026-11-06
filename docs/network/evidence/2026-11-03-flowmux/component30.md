# Authenticated component30 checkpoint

The pure `spiralcore-mvm-flowmux` component is frozen pending independent review,
not a completed issue44 contribution or canonical MVM/native/WAN acceptance.
The API draft is [mvm-flowmux-v1](../../../api/mvm-flowmux-v1.md). The
[carrier refinement](tls-carrier-contract.md) and
[explicit enrollment contract](scope-enrollment-contract.md) record the reviewed
trust boundaries. The earlier [component20](component20.md) and all failures
remain historical evidence with their original dependency/source identities.

Actual current-source Wick qualification is30 release tests PASS, zero failed,
ignored or filtered; one library test plus29 integration tests across8 binaries,
and zero doc tests. Strict all-target Clippy passes without warnings. The raw
current-source receipts are validation/current-dependencies-complete-release.txt
and current-dependencies-clippy.txt, with adjacent actual exit0 files. Commands
are recorded in validation/commands.md. No VM, host-network change, Mac build,
performance experiment or WAN operation was run by this component task.

The18-file own source inventory is source-component30-current-sha256.txt.
The140-file complete Rust/manifest dependency inventory is
source-component30-current-all-sha256.txt. Every remote inventory member was
checked against current local bytes (component30-current-local-check.txt).
The initial30-test source used an older shared dependency copy: three compared
existing files differed and four new storage files were absent. The locked check
refused; the first incomplete three-file sync then failed missing storage module.
Both failures are retained. The complete current shared sources were copied,
the standalone lock was regenerated offline, then the complete30-test release
and strict Clippy ran again. No dependency drift is hidden by the earlier PASS.
Only Cargo.lock changed in the frozen pure crate during this reconciliation.

Current wire header is120 bytes: role-bound direction identity, full policy
SHA-256, full admitted-scope commitment, exact sequence and bounded length.
Context's numeric epoch slots are reserved0; actual full admitted facts bind
its namespace. Canonical core Warm/Runtime/Combined and Published verification
remain shared. Full plan/session/nonce/exporter partitioning intentionally means
verified warm state is reused within this flow, not across connections. An
explicit stable cache-scope capability is a separate future design/qualification.

Mutual TLS1.3 uses rustls's ring provider and standard RFC7250 RPK signature
verification, full SPKI pins, mandatory ALPN and bounded readiness under the
original setup Instant. One original TCP socket and TLS state serve both cloned
I/O handles. No second dial, plaintext fallback, wire-minted grant, copied codec
or handwritten crypto/DER parser exists. The bounded local PinnedScopePolicy
requires exact enrollment; unknown/changed/foreign/expired/revoked scope rejects.
It is the trusted operator/controller enrollment API, not a general signed
cross-organization discovery/delegation system. Core and mapped I/O recheck it.

Actual semantic REDs retained in this phase: a trusted TLS role could be
replaced at duplex startup, and revocation still released a previously verified
buffered suffix. Both are fixed with focused GREEN and the full30-test result.
The120-byte conversion initially failed two older raw-peer/direction fixtures;
fixtures now supply the exact real role/direction header before testing the
original Published/event-bound assertions. The same-role test now observes the
new earlier Protocol refusal rather than the old transfer Sequence refusal.
No wrong-direction Published proof relies on a malformed role header.

One functional authenticated fixture recovered20,000 application bytes forward
and4,096 reverse after half-close; actual ciphertext counters were2,617 forward
and2,557 reverse, equal to the opposite endpoint's received/sent counters. They
include TLS handshake/profile setup/close and mapping/core replies, exclude
TCP/IP and canonical MVM carrier overhead, and establish no CPU/time/WAN gain.
Other test workloads/counters are deliberately not matched performance arms.

The separate canonical seam overlay has focused signed-policy/default-None/hash,
admitted-plan projection and real authenticated NetworkFlow refusal tests. Its
five selected MVM library/bin packages pass strict Clippy. Selected terminated
and typed HTTP paths produced actual bypass REDs and now refuse unsupported
combinations; the ordinary None path retains its canonical transforms. This is
not yet the production TLS factory, a full integrated access-agent source run,
a native guest/backend/platform test, UDP/ingress mapping, scale or WAN proof.
Full canonical factory/controller integration and independent code/security review
remain required. Full issue44 and the parent Ari/MVM goal remain OPEN.
