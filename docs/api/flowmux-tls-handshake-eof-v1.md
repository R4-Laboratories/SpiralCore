# FlowMux TLS handshake EOF failure

Status: additive failure correction contract before focused public tests and
implementation, succeeding immutable reader checkpoint
`6c885fe5a1a03d65a6f9093edc407c98feff31be`.

Both TlsCarrier constructors retain the same already-admitted TCP socket, TLS1.3
mutual raw-public-key/SPKI verification, ALPN and original absolute deadline.
While rustls is still handshaking, `read_tls` returning zero means physical EOF
before authentication completed. Return typed `io::ErrorKind::UnexpectedEof`
immediately and close the original socket. Do not spin until the remaining
deadline, construct a ready carrier, allocate an object engine or fall back.

This condition is fatal; it changes no TLS parser, cipher, certificate/pin
verification or application terminal-EOF rule. Outside the handshaking state,
existing TLS read/close semantics remain intact. Partial input retains its
original deadline, and all existing handshake and transport errors still fail.

Public real-socket tests cover initiator and acceptor roles. The initiator's
peer consumes the actual first TLS record before an orderly write-half close;
the acceptor's peer half-closes without sending a ClientHello. Both preserve the
same original connection and require prompt UnexpectedEof rather than a later
timeout. The installed consumer separately retains and shuts down an original
pre-codec TCP wake while that handshake is pending, then joins its actual
session before group/pool/Drained barriers. Neither fixture establishes a
native VM, successful TLS authentication, WAN or performance claim.
