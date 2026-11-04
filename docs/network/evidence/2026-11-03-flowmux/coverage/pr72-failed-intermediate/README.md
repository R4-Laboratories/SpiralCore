# Failed intermediate instrumented attempt — retained, not accepted

Candidate7024f2fa / duplexa5b6f851 had passed34 ordinary release tests, but this
selective instrumented execution FAILED Combined half-close at duplex_tcp387:
right_tx.half_close returned Cancelled. The raw failed scope/log/exit,159 input
hashes, nine fresh compiled/protected-object identities and preceding executed
scope receipts remain here. A full34-test accepted coverage total was never
produced. Its partial profiles/LCOV stay remote and are excluded from final line
union and threshold claims.

Phase1/source comparison found the refactor removed the original Tx mutex before
checking local_closed following peer EOF. The peer could observe the FIN while
the writer still held Tx to flush and publish its local flag. That was an author
mistake and a missed effect in the initial independent review, not a reason to
raise a timeout, ignore the error or retry this unchanged candidate as proof.
The original ordering barrier was restored, with explicit guard release before
complete() relocks Tx. Both review correction and fresh restored-source approval
are retained in the cancellation packet. See ../pr72-restored for the separately
executed corrected-source34 success and91.10% coverage.
