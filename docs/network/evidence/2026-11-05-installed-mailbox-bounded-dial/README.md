# Bound the Firecracker mailbox dial to the original exchange deadline

Recovery V2 now uses a nonblocking AF_VSOCK connect. It starts one connect,
waits on the original descriptor with the remaining absolute deadline, and
rejects socket errors and hangup. The guest starts its deadline before the
initial dial and passes the same deadline to the one redial, authentication,
Reply ACK, and Cast. A writable descriptor only permits the pinned-peer
handshake to begin. Mailbox framing still opens after authentication.
The existing Vsock filter covers these operations unchanged.

The legacy development witness again uses per-operation timing and keeps
its original blocking dial and readiness exchange. The SDK test suite
checked expiry before socket creation, timeout during a pending wait, and
a closed peer. The focused tests use Unix sockets for deterministic wait
and hangup coverage. The native Firecracker run separately exercises the
AF_VSOCK path under the signed guest policy.

Apply [the SDK patch](mvm-vsock-dial.patch) and
[the witness timing patch](witness-timing.patch) over the preceding
[same-VM candidate](../2026-11-05-installed-mailbox-native-recovery-v2/).
The Linux source and executable pins are in [validation](validation.txt).
The [Linux AF_VSOCK poll implementation](https://github.com/torvalds/linux/blob/v6.12/net/vmw_vsock/af_vsock.c)
documents why writable readiness and verified peer identity are separate.

The locked Linux SDK and composed witness suites each passed seven tests.
The signed one-guest Firecracker rerun exited zero. The guest redialed
the original mailbox port, received the exact 65,023-byte Reply, sent the
held ACK, and completed the subsequent 23-byte Cast. No Call was sent on
the replacement. The original guest and staging charges returned to
zero, and the mailbox and bridge sockets were absent after retirement.

| Carrier | Guest to host | Host to guest |
| --- | ---: | ---: |
| First | 120,974 B | 126,135 B |
| Replacement | 10,406 B | 126,160 B |
| Both | 131,380 B | 252,295 B |

The counts include canonical envelopes, framing and pinned authentication.
The guest and host worker used 9.510 ms and 7.981 ms of thread CPU.
Exchange elapsed times were 16.596 ms and 16.096 ms. The guest lifecycle
through retirement took 2.653 s. The full supervised run took 2.845 s.
See [native result](native-verified.json) and [input pins](input-pins.json).

The static development guest still applies the canonical filter through
its private witness entrypoint. Production agent enforcement and enrollment
remain open. The current signed image set carries its enforcing agent
inside the authenticated boot runtime, so replacing the workload rootfs
does not qualify a newer agent. The next separate enforcement witness
uses a checked source-runtime generation and a direct workload, while
installed V2 retains the existing signed portable-runtime admission gate.
