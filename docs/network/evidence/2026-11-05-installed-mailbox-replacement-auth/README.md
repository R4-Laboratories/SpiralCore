# Authenticate the installed mailbox replacement socket

The replacement transport now uses MVM's existing pinned host and guest
session handshake on the same socket that will carry mailbox frames. Both
ends supply independently enrolled peer keys and the original recovery
session identifier. Short I/O remains under the original deadline and
cancellation token. Success opens mailbox framing. Refusal fences and shuts
down the transport. Handshake bytes count as framing overhead. The proof
does not change the canonical encryption or authentication of mailbox
envelopes.

The locked Linux release run passed 5 dedicated authentication tests and 12
live adapter tests. The dedicated tests cover exact pins, an unenrolled guest,
a wrong session with zero guest proof bytes, an idle peer at the original
deadline, and cancellation after the handshake begins. A Unix socket fixture
then sends one canonical encrypted Call and verifies its Reply on the same
authenticated socket. Its guest sent 731 authentication bytes and received
799. The Call and Reply envelopes were 3,923 and 3,425 bytes respectively.
The single local fixture sample took 1,921 microseconds from setup through
verified Reply. It is a functional sample. See [validation](validation.txt).

Apply [this adapter patch](replacement-auth.patch) to the previous composed
candidate. The existing [MVM pinned-session patch](../2026-11-05-installed-mailbox-pinned-carrier/)
provides the two SDK APIs used here. For the Linux run, a local Cargo overlay
substituted the composed MVM Core and contract paths for the git dependencies.
It changed no package versions. The source and composed lock digests are
recorded in the validation file. Exact Code, Rust, and Security review passed.

The original resident job still has to take its replacement from Runtime,
supply independently verified enrollment and session pins, and use this
adapter before the cached Reply callback returns. The fixture uses local
Unix sockets. Live V2 admission, same-VM Firecracker recovery, and the upstream
submission remain pending.
