# Installed mailbox broker expiry between workers

The installed Firecracker qualification path retains one shared broker after the last mailbox worker retires. Without maintenance, a replay identity remained charged after the existing 60-second replay floor expired. The regression observed two replay entries with zero live Calls, where only the owner entry should have remained.

The candidate funds one resident maintenance thread when the exact original broker is first promoted. It uses the qualified 2 MiB stack class, the original monotonic clock and the broker's original expiry rules. Its 250 ms best-effort sweep skips contended locks and leaves worker-local queues and pending Calls under their existing owner. Teardown joins the maintenance worker before releasing its worker charge. A fatal maintenance fault prevents further jobs on that retained broker.

The old source failed the replay test after 64.05 seconds. The final isolated Linux suite passed 37 tests with three opt-in witnesses ignored. The dedicated adapter test exercised contention. One ignored native witness was then run explicitly on Wick. It used the prior pinned development guest entrypoint, two sequential 1 vCPU / 1 GiB Firecracker guests, and no guest NIC. The first guest stopped after its Call receipt by design. The second performed an exact 65,023-byte Call and Reply and an exact 23-byte Cast. Both had zero guests and zero staging bytes after original retirement.

| Native sample | Framed vsock bytes guest → host / host → guest | Guest / host thread CPU | Guest exchange wall |
| --- | ---: | ---: | ---: |
| First guest, intentionally abandoned after receipt | 120,161 / 5,837 | 5.554 / 2.625 ms | 7.733 ms |
| Second guest, complete exact readback | 129,742 / 131,053 | 8.816 / 6.628 ms | 14.154 ms |

The full native runner took 5.149 seconds including both starts and retirements. Byte counts are observed envelope and framing counts at the vsock endpoints. This run qualifies short native exchange and retirement with the timer present. The 60-second quiet gap was exercised in the focused component test. Production activation, release-agent enforcement, final-ack recovery, sustained capacity, WAN parity and FIPS evidence are separate acceptance gates.

The patch is relative to the installed broker qualification patch and adapter source `9153dfe`. The lockfile difference selects the locally staged adapter for the test composition. The candidate and its evidence are published together for review.
