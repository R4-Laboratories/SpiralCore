# Bounded zero-prefix idle reads

SpiralCore now provides `StreamTransport::receive_or_idle`: a ready mailbox
transport can return a local Idle result after a positive interval with no next-
frame prefix bytes consumed, preserving the connection for later authorized
sends and exact frame reads. Partial frames retain their original deadline and
fail closed on timeout, cancellation or truncation. The existing strict receive
API, wire format, cryptography, error enum and frame limits are unchanged.

This is the next C8 recovery seam after the
[bounded endpoint checkpoint](mailbox-session-events-results-2026-11-03.md).
It **does not enable a retry timer, consumer, reconnect/resume or long-idle
service lifecycle**. An Idle result is not evidence of peer loss or permission
to resend. Both the persistent consumer and opaque relay still have their
existing lifecycle rules. [Source PR #62](https://github.com/R4-Laboratories/SpiralCore/pull/62).

[API contract](../api/stream-receive-idle-v1.md) ·
[Implementation plan](../plans/2026-11-03-stream-receive-idle.md) ·
[Compact evidence](evidence/2026-11-03-stream-idle/README.md) ·
[Independent measurements and all losses](evidence/2026-11-03-stream-idle/measurement-review.md) ·
[Source and executable pins](evidence/2026-11-03-stream-idle/execution-pins.json).

## Framing and ownership

The local idle window is **1 ms through 1 s**. Invalid arguments return the
existing typed `Io(InvalidInput)` before locking or I/O, without fencing. The
method fixes its effective frame deadline at entry, capped by the caller's
absolute deadline, configured transport timeout and existing 30-second maximum.
Consumers must pass the same original exchange deadline across repeated polls.

The idle clock starts only after acquiring and validating the ready transport.
Lock contention is never Idle. The existing distinction between failure while
owning a guard and failure while waiting for one remains explicit: a contended
lock error cannot promise physical closure without ownership.

After any positive prefix read, Idle is impossible for that invocation. Remaining
prefix and payload bytes use the original frame deadline, without a per-byte
refresh or partial-prefix discard. Cancellation/absolute expiry take precedence.
EOF, invalid lengths, partial errors and timeout-configuration failure retain
fatal closure and actual partial counters. Payload allocation follows the
unchanged length check; Idle allocates no frame buffer and changes no counters.
It releases the mutex before returning. Returned Frame bytes still require
canonical authentication by the existing consumer.

## Executed qualification

The written contract/plan preceded the missing-API compiler RED. Seven portable
cases plus the four existing TCP cases pass: healthy Idle followed by canonical
write and exact read; early retryable errors; partial prefix/body timing; fatal
error/counter/fence matrix; local input/state precedence; lock contention; and a
bounded real TCP exchange. The first expanded run's six-versus-seven-byte fixture
count error is retained separately. Its payload-equality check had passed; the
correction changed only the expected count. It is not a product-fix claim.

Reviewed source is `960e73c59d7eab62f15a16667f95e594e1cabc8b` with a verified
285-entry crate manifest. The recorded local live release passed **92 checks**;
the shared stage passed **131 = eight object-v1 plus 123 adapter checks**.
Strict Clippy, protected host/example builds and the static GNU guest build pass.
The seven new TCP checks increase the shared stage, not the live-stage count.

[Hosted artifact verification](evidence/2026-11-03-stream-idle/qualification/portable/verification.md)
records **88 checks on each of seven native client rows (616 executions)** plus
eight separate Unix storage executions. All seven original ZIP digests and 236
inner checksums pass. Native OS/architecture/Rust-host identities were checked;
the tested PR merge tree equals the reviewed head tree. Windows Server 2022/2025
x64, Windows 11 ARM64, macOS 26 ARM64/Intel and Ubuntu 24.04 ARM64/x64 are native
client/component and TCP-loopback evidence, not native Windows MVM or WAN proof.

The isolated [88-test coverage run](evidence/2026-11-03-stream-idle/coverage/README.md)
reports **96/97 changed executable lines (98.97%)** and **673/733 mapped production
lines across `live.rs` (91.81%)**. Its one uncovered changed defensive line and
all 60 uncovered mapped production lines remain counted. Branch mapping is
unavailable. Normal benchmark binaries and all 285 source hashes were unchanged;
executed instrumented binaries/profiles remain protected on Wick. Coverage counts
overlap other suites and establish no semantic-path or whole-project percentage.

## Strict-transfer costs after the shared-helper refactor

These unchanged Rust engines use **strict receive, not the Idle API**. Twelve
local TCP host cells/six matched pairs passed **108 publications and independent
exact readbacks**, with identical target IDs/lengths and successful owned-store
cleanup. Requested 1 MiB targets are 1,047,202 actual bytes. Median paired process
CPU/verified-wall changes were **+1.51%/+0.38% for Whole zstd** and
**-1.07%/-0.97% for Combined**. Whole increased in all three trials; Combined
trial zero increased **4.79%/4.62%**. Maximum process RSS was **36,823,040 B**
in this small host profile; it is not comparable to a larger-tier memory run.

Two current-source Linux/Firecracker cells passed **18 publications and 18
additional exact keyless reboot readbacks**. Each uses the same eight target
positions of **14,851,384 bytes**; Combined pays checkpoint/schema setup.
There is no fresh native before/after pair.

| Current arm | Logical traffic, both directions | Timed sender CPU | Timed receiver CPU | Full lifecycle |
|---|---:|---:|---:|---:|
| Equally tuned whole zstd/raw | 107,356,654 B | 4.368591 s | 3.619492 s | 8.644921 s |
| Combined | 62,537,586 B | 2.954190 s | 2.509574 s | 6.364583 s |

This mixed sequence uses **41.75% fewer logical bytes, 32.38% less timed sender
CPU and 26.38% less lifecycle time** under Combined. Schema-fit CPU/wall remain
**12.92%/6.99% higher** than Whole; schema-drift **7.56%/5.65% higher**. Unrelated
sender CPU is **0.61% higher**, and its cold 14.85 MB object still requires about
31.35 MB of canonical logical traffic. These are canonical-mailbox comparisons,
not a generic secured raw/TLS baseline or universal compression claim.

Native runs stayed within two NIC-less guests, two total vCPUs and 2 GiB RAM.
Transport use peaked at two links/two slots/524,288 reserved bytes and returned
to zero without refusal/fencing. Actual native RSS and aggregate guest plaintext/
cache ownership are not qualified here. The requested Linux experimental64 rate
remains unenforced. Physical stream accounting crosses two vsock hops and is
twice logical traffic; lower-level control traffic is excluded.

Host process CPU includes both actors/setup; thread CPU is separately recorded.
Native endpoint CPU covers timed setup/operation intervals and excludes untimed
gaps, JSON and final independent verification. Lifecycle starts after journal/
link admission and includes boot, transfer, keyless verification and teardown.
Worker/VMM/relay CPU domains remain separate. No instrumented, energy or send-only
measurement is substituted. Shared-host variability and the small exploratory
sample prevent causal optimization or statistical noninferiority claims.

Both measurement archives and all **41 members** verified before removal of only
two successful owned native disks totaling **536,870,912 apparent file bytes**.
Host cleanup follows the Rust engine's verified record; native cleanup follows
archive custody. No payload, VM disk, raw coverage profile or new build target
was copied to the Mac.

The next consumer step is original-identity retransmission and uncertain
Call/Reply/ACK reconciliation, followed by C9/C10 retention/long-session and
installed-owner/shared-budget gates. Pipeline one and the 2,048-Call guard remain.
[#43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49), parent
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42) and the final WAN/
upstream gate [#47](https://github.com/R4-Laboratories/SpiralCore/issues/47) stay open.
