# Completed mailbox Reply tail on a surviving connection

This development slice repairs a final acknowledgment after a caller has
returned its result. The original receiver explicitly replays its cached Reply.
The caller matches the exact retained wire, authenticates it as a canonical
duplicate, and writes the canonical receipt. The receiver releases the cache
only after verifying that receipt. The caller returns its application result
once. The ordinary strict API still creates no completed tail.

The adapter reserves the retained allocation's actual ciphertext capacity
against the same signed and aggregate memory budget before moving its ownership
and releasing the old Reply charge. The capacity controls preserve the original
unconsumed result on reservation failure. The replay path retains both original
clock bounds, authorization and attempt counters.

On Wick, the main baseline passed 30 canonical tests. The port onto main passed
10 adapter lease and authority tests, plus 101 entries across the live crate's
default all-target test command. The public direct TCP fault fixture withheld a
complete authenticated final receipt after local write, checked the byte-identical
cached Reply, then verified subsequent publications by reopening their exact
bytes. The fixture checks actual framed traffic counts and does not publish a
timed comparison. This packet contains test summaries and source hashes.

The archived candidate had pre-port REDs and corrections. This packet records
the fresh main baseline and the candidate run on the same host. The reviewed
source preserves PR #90 idle maintenance, including the ordinary Reply ACK wait.
The Linux actual-relay witness and hosted matrix are separate review gates.

The existing connection survives the deliberate frame withholding. A closed
connection, crash recovery, an installed-native session and cross-host WAN are
separate acceptance steps. See the linked [replacement plan](../../../plans/2026-11-05-closed-mailbox-replacement.md).
