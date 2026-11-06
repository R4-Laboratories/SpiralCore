# TLS carrier component contract

This narrows the current API draft's authenticated and bounded profile setup.
It changes no MVM envelope or object publication. A selected initiator and
acceptor use TLS1.3 only, mutual RFC7250 raw public keys, full SPKI SHA-256 pins,
and ALPN `spiralcore-flowmux/object-stream/1`, with no tickets, early data or
resumption. Pins are inputs from the trusted signed selection, not permission
created by this carrier API. Credentials load through the same rustls ring
provider. Full TLS1.3 signatures are verified using rustls's standard RPK API.

After mutual handshake the acceptor sends exactly eight TLS-protected bytes
`SCREADY1`; the initiator consumes them under the original setup deadline before
returning. This confirms that the server finished validating the client; it is
profile readiness, not a new authentication primitive or an object ACK. Wrong
pin, signature, ALPN, readiness or missing peer fails with the same socket
closed. A successful constructor returns the TLS exporter bound to this profile
for subsequent full-scope namespace binding. No codec is allocated here.

Cloned I/O handles share one TLS connection plus one nonblocking original TCP
socket under a short mutex. No blocking syscall/sleep occurs while holding it.
Read/write polls advance the same session in both directions. A stalled reader
therefore cannot lock out the writer. Reads/writes have bounded default budgets
and independent handle timeouts; setup uses its single original Instant. Empty
I/O has ordinary Rust semantics. Ciphertext bytes count actual read_tls/write_tls
results in each direction, separately from object-record counts. Close-notify
is sent before an orderly physical write half-close; cancellation closes both.
