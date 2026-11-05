# Native Firecracker mailbox development exchange

One synthetic Linux x86_64 guest on Wick completed Call, Reply and Cast
through the original installed mailbox and Firecracker's native AF_VSOCK
route on port 5257. The host and guest compared all payload bytes, including
trailing zero bytes, and verified their authenticated encrypted receipts.

| Measurement | Result |
| --- | ---: |
| Call / Reply / Cast payload | 65,023 / 65,023 / 23 B |
| Guest to host / host to guest | 129,676 / 130,981 B |
| Bidirectional carrier payload including framing | 260,657 B |
| Guest / host exchange thread CPU | 14.420 / 9.529 ms |
| Guest / host exchange elapsed | 22.321 / 21.677 ms |
| Host lifecycle elapsed including retirement | 3.782 s |

The 260,657 bytes are the sum of Firecracker port-5257 data-packet payloads
in both directions and match the endpoint counters. This excludes virtio
and link headers and credit/control packets. The endpoint timings overlap.
The run does not establish a WAN, codec comparison, sustained rate, or
customer capacity. The fixture configured no guest NIC. The retirement
oracle observed the original owner retired, both mailbox bridge paths
absent, no staging charge and zero live guests. Accounted runtime storage
was retained under its existing quota.

## Clock regression

The pre-fix development run reached the mailbox and the guest received
the first host data frame, then exited 27 during exchange. The old guest
agent's boot clock used integer seconds. The focused regression reproduced
an authenticated receipt rejection with the sender 1,000 ms ahead.
The witness now uses the existing 5,000 ms bounded sender-side issuance for
Call, Reply, Cast and receipts. The same signed receipt still fails at a
6,000 ms offset, leaves the tracker pending, and verifies at 1,000 ms.
This does not relax canonical receiver checks or extend expiry.

## Runtime authority

The legacy published guest agent failed the strict effective-confinement
check in the previous [checkpoint](../2026-11-05-installed-mailbox-vsock-confinement).
For this developmental carrier experiment only, the signed private workload
image includes an entrypoint that requires the signed plan's boot tier,
applies the exact canonical MVM Vsock filter, and calls the ordinary
effective-confinement check before opening bootstrap or dialing. The
result explicitly records that guest-agent-enforced release policy remains
unqualified.

The upstream v0.27.0 CI guest runtime
[run](https://github.com/tinylabscom/mvm/actions/runs/41966531575) was
verified by its artifact and producer checksums and contains the newer
agent. Its producer context identifies it as an unsigned branch artifact.
The next strict gate requires an authenticated newer runtime accepted by
MVM's original installed launch, followed by the same native exchange and
retirement oracles. Two guests, recovery, scale and WAN are tracked
separately under [SpiralCore #43](https://github.com/R4-Laboratories/SpiralCore/issues/43)
and the [delivery roadmap](https://github.com/R4-Laboratories/SpiralCore/issues/42).
