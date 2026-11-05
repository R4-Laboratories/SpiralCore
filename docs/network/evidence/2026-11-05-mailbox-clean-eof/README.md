# Distinguish a closed mailbox carrier from a truncated frame

The opt-in receive path reports clean EOF only after the peer closes its write
half before any byte of the next prefix arrives. Cancellation and the original
absolute receive deadline take precedence. A partial prefix or payload remains
a framing failure. An existing send view observes the same closed transport
state, and the socket is shut down using the existing full-close fallback.

The strict receiver still reports UnexpectedEof on clean closure. The separate
relay reader retains its reverse-half behavior. No protocol bytes change. Clean
EOF describes the old socket and supplies no authorization for a replacement.

The test-first compile failed with the missing API. The first implementation
passed all 15 real and scripted TCP cases. Expanding the test-only writer
helper surfaced a Rust Arc dereference error, corrected by borrowing the
original implementing transport. The final suite passed 17/17 on Wick with
the completed-tail draft in the surrounding source. The test uses real TCP
FIN, observes the exact byte counters for partial frames, and checks that a
stale authorized write view refuses. It also checks strict compatibility,
cancellation, and expiry. No VM or WAN test is implied.

Independent Rust, code and security source review passed. The proof of final
ACK recovery over a replacement still requires the original application and
mailbox state on both sides and a separately admitted carrier.
