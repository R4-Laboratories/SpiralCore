# Preserve the resident mailbox service for sustained campaigns

The canonical campaign Session can use either its existing owned service
or a mutable borrow of the service supplied by its resident. The same
session logic handles both. Borrowed construction checks the shared budget
before creating its receipt tracker, and the resident can shut down the
same service after the session ends. The check establishes budget identity.
The installed caller must still supply the original broker context with
its shared dispatch lock and clock.

The focused Linux test sent a real canonical Begin Call and compared the
encoded Reply on both endpoints. The original service retained the accepted
identity and FIFO scope after the queue entry drained, then its owner shut
it down. A service from a foreign budget was refused before the exchange.

| Check | Result |
|---|---|
| Focused borrowed-service tests | 2 passed |
| Canonical suite on exact candidate rerun | 37 passed |
| Canonical suite on protected baseline rerun | 35 passed |
| Isolated tail timing control, baseline / candidate | 2 of 3 / 3 of 3 passed |

The first broad candidate run passed 36 of 37. Its failing tail fixture
expected a write error on its first poll but got a normal idle result.
The same failure reproduced with the protected baseline. Both broader
reruns passed. The retained evidence does not establish that the timing
race was fixed.

The installed witness currently provisions 300-second capabilities and
funds a 64-identity broker tier with a 16-identity control reserve. This
patch keeps those limits. The next native run needs to bind the campaign
to the existing authenticated carrier, explicitly fund its qualification
tier on the original resident, and account for the signed lifetime,
paced load, object store, and real retention windows. No sustained
native run or capacity claim follows from this component.

[Native installed baseline](../2026-11-05-two-installed-agent-v2/README.md)
