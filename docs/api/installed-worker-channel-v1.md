# Installed worker channel v1

Status: private Rust implementation and focused native process qualification;
actual worker/VM integration remains lead-owned and unqualified here. This is
an anonymous host-local grant exchange, not a canonical MVM network envelope,
alternate guest authority or payload protocol. See
[installed supervisor v1](installed-supervisor-v1.md).

The authoritative WorkerBinding lives in installed_supervisor.rs: exact original
root, installation/profile IDs, generation, role (1Sender/2Receiver/3Verify),
slot0..1 and nonzero serial. The channel imports it; it cannot mint a ledger
grant. Fixed profile is independently checked against InstalledConfig. Root is
an absolute non-NUL Unix pathname, at most896 bytes, with no parent components.
State/lock identity, key and Active epoch remain canonical HolderLease checks.
IDs and nonce facts do not replace canonical signed-plan/guest authorization.

## Parent API and flow

```rust
ParentControl::pair() -> Result<(ParentControl, UnixStream)>;
parent.receive_challenge()?;
parent.send_binding(&controller.worker_binding(&grant)?)?;
parent.wait_ready()?;
parent.write_initrd(&mut parent_owned_stdin_writer, &initramfs)?;
parent.receive_boot_challenge()?;
parent.send_go(&controller.worker_binding(&grant)?)?;
```

Return only the child endpoint across exec at fixed descriptor198. The parent
endpoint is private/non-Clone and must never be inherited or delegated. Parent
Child/slot/grant validation and both fresh binding queries are controller-owned,
outside all owner/accounting/worker locks. The kernel peer PID seen by the parent
of a socketpair remains its creator's identity, not the forked child's PID;
associate that endpoint with the exact owned Child and slot instead.

Parent phases force that order. Original binding is saved; Go requires exact
equality of every field and a distinct second child challenge. Invalid phase or
record fences the invocation and closes control. No retry, rebind or reset.
The existing initramfs bytes go unchanged through a parent-created UnixStream
stdin pair. Parent writes them nonblocking under the retained deadline and
half-closes Write. Empty or >64MiB refuses; the payload is not a control record.

## Child API and custody

```rust
let mut ctx = InstalledWorkerContext::from_inherited_fd(expected_role)?;
let initramfs = ctx.read_initrd()?;
// Host preparation may consume these exact bytes; do not reread stdin.
ctx.authorize_boot()?;
ctx.check_before_backend()?; // immediately before native backend creation
let held = ctx.holder_handoff()?; // Linux typed FC handoff only
```

The entry is called once before worker threads. It atomically duplicates and
validates only fd198 as AF_UNIX/SOCK_STREAM, closes inherited198, and retains a
CLOEXEC/nonblocking stream. Existing MVM connected_peer_pid must equal the live
kernel parent; arm existing ProcessExitObserver before issuing a fresh32-byte
challenge. Ambiguous/unavailable peer identity refuses, with no argv PID fallback.
On the fresh matching answer, validate fixed profile/expected role and acquire
the original HolderLease before Ready. Socketpair creator identity is qualified
by a real fork/exec fixture on native macOS; it is not assumed to become child
identity. Child context/control/observer/holder live through host cleanup.

read_initrd duplicates fd0 only as AF_UNIX/SOCK_STREAM and consumes EOF-bounded
input under the same deadline. It owns one Vec of at most64MiB with bounded
geometric capacity growth, no persistent copy/log. A failed read shuts down its
stdin endpoint and control; no backend may run. authorize_boot uses a fresh
different challenge and accepts only matching Go with the entire original
binding. A post-Go peer/deadline check precedes holder export/native creation.
Export uses canonical prepare_handoff, not a new state parser or lock reopen.

## Bounds and failure behavior

Exactly five records: Hello, Binding, Ready, BootChallenge, Go. Private encoding
is version1/kind/32-byte nonce, with bounded canonical-length binding fields for
Binding/Go. A four-byte length prefix is included in the1024-byte total bound;
length is checked before allocating a received body. Unknown version/kind,
trailing/truncated bytes, invalid fields, nonce reuse/mismatch and unexpected
phase refuse. Partial I/O retains one original absolute monotonic deadline.

Parent deadline starts once at pair creation; child deadline starts once at
entry, each bounded40s. No step refreshes its local cap, including raw initramfs
read/write or a completed frame. Deadlines are not serialized Rust Instants or
new signed guest validity. Sender failure can have an uncertain already-written
Go; retain ledger/holder ownership and use the existing shutdown path. Go can
race later controller death: no atomic death-to-boot cancellation is claimed.
An already charged worker/VMM SH prevents retirement while that lifetime lives.

EOF, observed controller exit, I/O or authorization failure closes the exchange.
Errors contain fixed categories/OS errors, never nonce, binding or payload
contents. No fallback/automatic traffic, new socket path/port, listener, reaper
thread, PID signaling or independent resource quota is added.

## Focused qualification

Before implementation the exact-product-file wrapper retained a missing-module
exit101 RED. Native Mac libc-only peer qualification loads the original pinned
MVM uds_peer.rs by path and executes a real child with inherited198. The channel
wrapper likewise loads actual MVM peer/observer/liveness code, plus exact
SpiralCore product files; it is a component witness, not a full MVM crate build.

Real process checks cover five-record exact input, context SH after parent
finish refusal and final release, all seven Go-binding field mutations plus a
root spelling alias (Go compares the original Unix path bytes), forged
nonce/epoch/role, invalid profile/role, EOF, oversized/truncated prefix, empty/
over-limit input, partial writes with the original deadline, and actual owned
controller death before Go. The death test observes the original worker exit
with a kernel observer and requires a fixed refusal marker; it signals only
its exact owned controller Child. No VM or payload store is involved.

Root owns actual public entry/default-path closure, Linux full-graph checks,
native FC/HVF lifetime, installed shutdown/recovery and release acceptance.
Component passes do not complete B6a/B6b, scope=host, WAN, Windows native MVM,
retention or the five-outcome delivery goal.
