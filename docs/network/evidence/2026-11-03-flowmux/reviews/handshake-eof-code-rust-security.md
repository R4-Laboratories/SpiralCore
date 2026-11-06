# TLS startup EOF: independent Code / Rust / Security review

2026-11-04. Root independently reviewed Retained's additive successor to
`6c885fe5a1a03d65a6f9093edc407c98feff31be`, after verifying active Code, Rust and
Security review roles. **PASS for this exact two-file component change.**

Qualified product `src/tls.rs` SHA-256:
`60b0eae5a83fe09ff902a0c896b8cb0805b7d943b08217b74175b9218d55a44d`.
Public `tests/tls_handshake_eof.rs` SHA-256:
`5345b0c4c63ebbb95499f24035917146e0c586f3760429bb2e25b8dd473a002b`.
Both paths are relative to `crates/spiralcore-mvm-flowmux`.

The product diff is six added lines in the existing TLS pump. A zero-byte
`read_tls` while rustls remains handshaking now returns typed UnexpectedEof.
Inspection of the surrounding constructor confirms that this error shuts down
the same original TCP socket before returning and before constructing a ready
carrier/exporter. There is no authentication, pin, signature, TLS parser,
cipher, ALPN, application-frame or original-deadline change. Post-authentication
EOF continues through the existing application-completion rules. The error
does not create an object engine, reset a deadline or invoke fallback.

The two public real-socket tests cover initiator and acceptor roles. The
initiator peer consumes the actual first TLS record before its write-half
close, avoiding an unread-input reset being mislabeled as orderly EOF. The
acceptor peer closes before ClientHello. Both retain the original connection
and require prompt typed failure rather than waiting for the original timeout.
Scoped threads are joined and use finite test waits. The small record-header
read is fixture synchronization only; no replacement TLS parser is introduced
into the product.

The reviewer read the actual two-failure pre-product output, final zero exits
and all eight final target summaries: 43 entries, zero ignored/filtered. The
ten repeated terminal-completion checks and eight strict Clippy target exits
are separate repetitions/qualification, not additional unique test cases.
The evidence records 385 source inputs and eight protected executables unchanged
through qualification. Archive and candidate-tree verification are recorded at
publication. This is independent source/receipt review, not a new test run.

The earlier installed stalled-setup failure is retained. This component alone
does not prove installed raw-socket cancellation, original reader/relay custody,
resource bounds, native VM retirement, WAN, platform or FIPS acceptance. Those
remain the installed consumer's obligations. No blocking finding was identified
for this narrow failure correction.
