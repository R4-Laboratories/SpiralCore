# Authenticated FlowMux terminal EOF classification

Status: proposed narrow shared-core correction after an observed existing
terminal-join failure. Ten matched executions of the unchanged authenticated
fixture failed nine times on original ca5 source and nine times with the
additive reader API. Original logs, sources and protected binaries are retained.
The reader API did not introduce this failure.

The existing successful application-completion barrier is unchanged: an
authenticated remote application FIN has been processed with no active object;
the local application FIN is fully written, observed while holding the original
TX barrier; and no required reply remains pending. Canonical object publication,
sequence and exact application queues have already completed. A fresh scope and
fatal-delivery check must still pass before completion.

Only at that barrier, a transport `io::ErrorKind::UnexpectedEof` surfaced while
polling for the NEXT frame has the same terminal classification as the existing
`Error::Truncated` path. No error text is matched and no TLS parser, authentication,
wire format, object codec, receipt or data check changes. The generic carrier
continues reporting missing TLS close_notify as truncation. Partial frame/body,
premature EOF without both authenticated application FINs, pending replies,
cancellation/revocation and other TLS errors remain fatal.

Qualification retains the original failing positive fixture and exact-counter
checks. Before the correction, add actual pinned TLS negative cases: missing
remote FIN after the local FIN, and transport loss after canonical Need with the
next request physically observable but its publication reply withheld. Both must
remain failures with fenced application delivery. Then re-run both negatives,
the positive fixture and existing duplex/mapped/cancellation tests on the exact
successor. This is neither a TLS close_notify implementation nor a general EOF
exception, and provides no new native, WAN or performance claim.
