# Pin the authenticated mailbox replacement carrier

The host entry requires the enrolled guest key before confirming the
handshake. The guest entry requires the exact expected session identity and
pinned host key before writing its challenge proof. It verifies host key
possession with the existing signed confirmation before exposing a Session.
These methods reuse MVM's authenticated session and sealed-frame code. Existing
constructors retain their existing behavior and wire format.

The focused locked suite passed 21 tests on Wick. The new TCP fixture checked
the exact sealed Reply with matching peers, refusal for a foreign guest key,
and refusal for the wrong session identity. A separate counting fixture
observed zero guest challenge-proof bytes for the wrong session. The RED
compiled against the same dependency graph and failed at the missing APIs.
Independent Code, Rust, and Security source review passed.

The host's original worker must still derive and enforce the signed,
launch-specific session scope and original I/O deadline, and prove that the
accepted socket came from the original live Firecracker process and bridge.
This is a component checkpoint. The tests do not boot a VM or establish
cross-host performance or deployment compliance.
