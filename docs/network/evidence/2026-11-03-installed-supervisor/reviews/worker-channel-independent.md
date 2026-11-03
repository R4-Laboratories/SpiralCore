# Independent installed-worker channel review

The lead reviewed the separate channel author's production code and its actual
installed consumer. PASS for this local provisioning boundary; final release
source/tests and remaining platform acceptance are separate gates.

The channel is an anonymous Unix stream, created by the controller and retained
beside the exact owned Child. The child's kernel peer must match its current
parent; existing MVM process-exit observation is armed before accepting facts.
Two fresh challenges, fixed phase transitions and original startup deadlines
precede input and boot. The controller obtains fresh canonical original-epoch
facts for Binding and Go. Neither argv identities nor returned metadata create
boot authority. No owner EX descriptor or network listener is exported.

Framing checks the 34..1020-byte payload bound before allocation and checks
complete kind-specific length before indexed decoding. Binding roots are bounded
to896 bytes; profile, role, slot, serial and epoch are validated. Root equality
also compares exact Unix bytes, avoiding normalized Path equality for a changed
Go. Input reads are checked before growth and capped at64MiB. Partial reads,
writes, EINTR and WouldBlock retain one original monotonic deadline. Failed
phases close their stream and never fall back to the legacy entry.

The child accepts one fixed inherited descriptor, validates AF_UNIX/SOCK_STREAM,
duplicates with CLOEXEC and closes the original. It holds the canonical SH lease
through Linux/HVF lifetime and only exposes the existing typed holder handoff.
Local same-UID/root provisioning remains trusted; this is not remote peer
authentication or a new MVM envelope. The demonstrated Linux launcher/FC copy
of SH and the actual death/refusal/retirement witness strengthen the consumer
evidence without extending this verdict to native HVF recovery or Windows.

Reviewed SHA-256:

- `src/installed_worker.rs`: cb20bfcb2e56339d561f70c4f5fa0656cd3d2d01e07e7a0823caf2c2d3db3987
- `src/installed_worker/codec.rs`: 66ae27b53f9d1af1bf36b778220070daca8e53cb77537a999ba774dc1f459680
- `src/installed_worker/io_util.rs`: be18bfb83564273b3e71c8eab08668c50b97197ef2aef4b359c7059504f8352c

Paths are relative to `crates/spiralcore-mvm-live`. Bounds/accounting here cover
startup and serialized transport, not the unfinished aggregate plaintext/cache
budget. The channel author separately reviewed the lead's controller/consumer
code and explicitly excluded this channel from their independent verdict.
