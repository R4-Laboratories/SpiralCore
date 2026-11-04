# Canonical factory component contract

The canonical hostd module adapts its private AdmittedTcpPayload to the pure
FlowMux mapping. It depends on one immutable SpiralCore revision for both the
pure adapter and existing shared transfer core. A remote-only path override is
allowed for intermediate tests and is excluded from product/merge qualification.
No core copy, second route/dial or cyclic MVM dependency is permitted.

SpiralCorePayloadFactory::from_host_key takes the actual admitted endpoint host
SigningKey and an explicit Arc<dyn PayloadScopeController>. Standard Ed25519
EncodePrivateKey/PKCS8 material feeds the existing rustls ring key loader; no DER
or signature implementation is copied. The factory consumes the already admitted
TCP socket, authenticates the signed selected peer pin/profile under the original
setup Instant, then materializes full ScopeFacts from the private token and actual
TLS identities/exporter. It never reads payload-supplied facts as permission.

PayloadScopeController::prepare receives the actual AdmittedTcpPayload, complete
ScopeFacts, fixed authenticated v1 NegotiatedProfile and original deadline. It
must authorize and explicitly provision the exact scope before constructing its
PreparedPayloadSession, containing ScopeAuthority plus the existing shared
ObjectSender/ObjectEndpoint<ScopeAuthority>. It owns the bounded storage/resource
configuration. Receiver enrollment remains explicit: mTLS alone never approves a
foreign plan. Unknown/expired/revoked/mismatched controller enrollment refuses.
The factory compares the prepared authority's complete binding, verifies actual
TLS role/key/exporter, and freshly checks the original deadline before returning
mapped I/O. No callback, TLS I/O, object publication or join holds a stream ledger.

accept_enrolled_payload takes an already authenticated TlsCarrier, an explicitly
prepared/enrolled session and original deadline. It is a cooperating application
endpoint on that same TCP, not a new proxy or canonical MVM guest transport. Both
ends use shared Duplex/MappedStream and core Published verification. The fixed v1
core profile has maximum object size65536 and existing core capability bits; no
peer claim creates a capability. Every active endpoint retains current scope
checks. Clones share the same mapped worker/TLS state and last drop joins it.

The initial real canonical test refuses a fully authenticated but unenrolled
peer before application/object bytes. A positive test must cross actual encrypted
NetworkFlow OpenTcp/Data/HalfClose and a real paired TLS object stream, preserving
all canonical admission/rate/window/audit/reset behavior. Whole is an intermediate
setup mode, not completion of the required shared Warm/Runtime/Combined mapping.
Those paths require explicit authenticated artifact publication under the same
original setup Instant; existing public seed methods cannot be called serially
with refreshed budgets to claim this gate. Production daemon provider/controlplane
materialization, aggregate resource qualification and native/WAN gates remain open.
