# 2,049 durable transfers on one original Firecracker pair

One Linux x86_64 guest pair completed 2,049 contiguous 257-byte transfers with 2,049 distinct message IDs. For each transfer A staged a 313-byte frame, B made a separately authorized empty Receive, A verified B’s signed V2 receipt, B independently read back the exact payload and acknowledged its original lease, and A settled the receipt digest. No status or settlement retry was needed. The final row file was updated at 23:42:15.499 UTC, about 5.08 seconds before the original one-hour grant expired. Both original helpers exited with code 0 on their first unassisted stop. [Original measurements, artifact identities and hashes](native-2049-checkpoint.json)

| Measured interval | Wall time |
|---|---:|
| Setup | 6.396 s |
| Sequential transfer series | 3,588.550 s |
| Stage to host-verified V2, median / p95 | 0.312 / 0.383 s |
| Stage to verified B readback, median / p95 | 1.648 / 1.758 s |
| Stage to A settlement, median / p95 / maximum | 1.764 / 1.878 / 3.203 s |
| Whole timed driver including teardown | 3,595.43 s |

| Accounting boundary | A | B |
|---|---:|---:|
| Release guest workload CPU over completed operations | 2.495 s | 3.389 s |
| Original port 5257, completed exchanges over lifetime | 4,099 | 6,148 |
| Port 5257, bytes read from guest / accepted for write to guest | 2,701,645 / 3,418,032 | 6,154,130 / 24,252,706 |
| Port 5257, read / write errors over lifetime | 0 / 1 | 0 / 1 |
| Original VMM cgroup cumulative CPU at last sample | 97.264 s | 106.675 s |
| Original VMM cgroup peak memory through last sample | 88,875,008 B | 91,045,888 B |

The application delivered **526,593 bytes**. The local relay carried **12,052,072 bytes toward B** and **2,487,009 bytes in signed returns toward A**. Those bytes and the port-5257 bytes describe different links. Both port observers wrote their final totals. The write errors cannot be assigned to particular operations. B’s workload CPU includes 0.502 s for the empty Receive controls. The debug driver used 92.391 s of process CPU. GNU time reported 455.83 s user CPU, 96.27 s system CPU and 83,616 KiB maximum RSS for the measured process tree. The cgroup observations were sampled while the VMMs lived. These scopes overlap. The nominal load average was 9.20 at launch on the 32-logical-CPU host.

## Journal behavior across the earlier failure

The first trial reached the reader deadline after B leased ordinal 1,481. The successor holds the original journal prefix when its signed checkpoint cannot fit in the checkpoint record or available staging space. Automatic maintenance retries when additional terminal records pass their signed retention horizons. Explicit checkpoint requests still report full, and a whole journal still refuses new admissions. The cause and precise timing of the first B helper’s exit remain unobserved. [First trial](native-2049-initial.md) · [Journal change and regression](https://github.com/Casey-allard/mvm/commit/eff4467eaf48749ddeb4c2e8a06121a26169ba95)

At a live sample with 1,498 settled transfers, B had 34,070,886 bytes of retained entries and its anchor was at 4,496. After both guests stopped, B retained 6,147 original journal entries totaling 46,579,868 bytes, an anchor at 6,147 and zero checkpoint files. The byte sum excludes filesystem allocation, ancillary files and the anchor. The configured limits remained 64 MiB per journal, 10,000 records and 16 MiB per checkpoint record. During the trial, 178 samples of A’s active ordinal snapshot ranged from 165 to 13,357 bytes. These samples do not bound the temporary storage required for an atomic replacement. The final audit verified 17 entries, two sessions and two seals. Both original scope units were inactive with absent cgroups, and zero VMMs or helpers remained.

The canonical automatic-maintenance regression first returned `Full`. The successor passed 22 journal tests, including protected exact retry across reopen, hard quota refusal without anchor movement, anchor mismatch, and the transition one millisecond beyond an acknowledged record’s retention horizon. The Linux host service passed 33 tests with one further test ignored. The driver passed 19 tests. The full Linux workspace and the separately feature-gated conformance BDD targets compiled. The native run used the new debug host binaries and the same original release guest binaries, driver source, sealed images, resource limits, cadence and grant lifetime as the first trial. [Host source](https://github.com/Casey-allard/mvm/commit/eff4467eaf48749ddeb4c2e8a06121a26169ba95) · [Original guest runtime](https://github.com/Casey-allard/mvm/commit/4dcf544bed90bfc2cb90c065998a025b859ddab4)

The trial qualifies 2,049 serial durable ordinals with one transfer outstanding and an interleaved sequential control, on one zero-NIC pair with one vCPU and 256 MiB per guest. Multi-tenant contention, concurrent transfers, live helper replacement on the original guests, higher capacity and WAN delivery remain open. [Sender retention](https://github.com/R4-Laboratories/SpiralCore/issues/195) · [Capacity and fairness](https://github.com/R4-Laboratories/SpiralCore/issues/49)
