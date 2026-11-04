# TLS startup EOF successor

Linux x86_64, Rust 1.97.1; successor to immutable
`6c885fe5a1a03d65a6f9093edc407c98feff31be`. The earlier 39-case checkpoint
keeps its original source/result identity. No native MVM, installed-profile,
platform/WAN, FIPS or performance acceptance is asserted here.

The actual installed stalled-setup fixture first observes a ClientHello on the
admitted TCP socket with zero active codec charge. Its original group join
deadline expires, the peer's read times out, and the parent's five-second
watchdog kills and reaps its original test child. That retained MVM failure is
at `/var/tmp/spiralcore-installed-factory-20261104/validation/stalled-tls-red`;
it is not replaced by these pure transport checks.

Source investigation finds a second setup problem: rustls read_tls returning
zero during handshaking is processed without terminating the constructor, so
the loop waits until its original deadline. Two public tests reproduce this
before the product change, exit 101: both initiator and acceptor return TimedOut
instead of prompt UnexpectedEof. The initiator's peer consumes the complete
first actual TLS record before an orderly write-half close; the acceptor's
peer half-closes before ClientHello. Each retains the same original connection.
The original four-second/two-failure result and its protected binary remain.

The production delta adds only a fatal typed UnexpectedEof when read_tls
returns zero AND rustls is still handshaking. It changes no TLS parser, cipher,
pin/signature verification, ALPN, application framing or original absolute
deadline. Other TLS errors still fail, and post-handshake EOF continues through
the existing strict transport/application completion paths. No ready carrier,
object engine or fallback is fabricated for the unavailable peer.

The exact successor passes 43 focused entries across eight pre-protected
objects: library 9, duplex TCP 9, mapped stream 1, paired TCP 8, authenticated
stream 10, reader completion 2, TLS carrier 2 and handshake EOF 2. There are
zero ignored/filtered entries in these full target executions. The earlier
terminal-close positive fixture also passes ten repeated executions; those are
repetitions of one case, each with its nine filtered siblings. All eight scoped
release Clippy targets pass `-D warnings` with no warnings. All 385 source files
and eight protected executables match before/after. The actual TCP/TLS
fidelity/counter checks remain in their original test output; no new transfer
timing or CPU result is inferred from test duration.

Product tls.rs SHA-256:
`60b0eae5a83fe09ff902a0c896b8cb0805b7d943b08217b74175b9218d55a44d`.
New test SHA-256:
`5345b0c4c63ebbb95499f24035917146e0c586f3760429bb2e25b8dd473a002b`.
Dependency lock and the rest of the 6c source remain unchanged.

The transferred compact archive SHA-256 is
`9c9c6c5a7f798c73de52d7cd1d4643eeff91cbd7877b6c58e4d31c1859651941`;
all 73 receipt members and the original index (74 archive files) were checked.
Root publication initially compared the unstripped archive directory prefix to
the index paths and refused before creating a Git index/commit. The corrected
check removes only the known `compact-handshake/` prefix, verifies the original
index bytes and checks every receipt digest; no evidence bytes changed. The pre-product
intent hash file is retained separately. Original manifests, compiler JSON and
protected executables remain on Wick at
`/var/tmp/spiralcore-flowmux-reader-20261104`; none were copied to the Mac.

The owned MVM consumer must separately execute its original TCP setup wake,
prepared-reader custody, session/relay/reader joins and supplied pool/store
drainage. This pure result does not establish that combined lifecycle. The
complete installed resource model, native retirement and WAN/platform gates
remain open.
