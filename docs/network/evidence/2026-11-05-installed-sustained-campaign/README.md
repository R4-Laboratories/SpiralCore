# Sustained installed Firecracker mailbox qualification

This is the source and preflight record for the paced Linux x86-64
Firecracker witness. The short installed object campaign is the
[preceding source and evidence](../2026-11-05-installed-object-campaign/README.md).
Apply this patch to that composed source. The final source and
executable pins cover the successor after the pre-authentication
resource refusal.

The sustained-only signed fixture schedules three distinct synthetic
3 MiB Raw objects in 4 KiB chunks. One Status and three groups of
Begin, 768 Data Calls and Finish yield 2,311 Calls. Four Calls per
second with no catch-up gives a minimum 577.5-second Call schedule.
The original V2 carrier, service, broker and ordering scope are
reused. All tests use synthetic data.

The fixture signs a 1,200-second execution-plan window and
1,800-second grants and checks remaining authority before admission
and carrier authentication. The campaign uses a 900-second
Call-boundary guard. The observation path uses the original audited
guest-agent process RPC with a bounded longer read. Ordinary profiles
retain their existing windows and Call limits.

The final fixture reserves 22 MiB of post-boot codec workspace,
including a 12 MiB endpoint limit, 9 MiB of fixed inputs and 1 MiB
for campaign overhead. The reservation and process RSS cover
different scopes. The owner and tenant ceilings are unchanged. The
host and guest builds exited zero and the nine focused mailbox
tests passed. Both earlier native attempts stopped before
authentication completed. The second isolated the 24 MiB codec
reservation refusal.

The third attempt completed and exited zero. It ran one original
Linux Firecracker x86-64 guest with 1 GiB RAM and no guest NIC.
Both peers completed all 2,311 Calls on the original session.
After the original owner retired the guest, the host reopened and
byte-compared all three publications (9,437,184 bytes), removed
the private store and verified its absence. Guest and staging
charges returned to zero and no Firecracker process remained.

| Measurement | Guest | Host |
|---|---:|---:|
| Bytes sent, including authentication and framing | 41,074,041 | 22,420,642 |
| Application frames sent | 4,622 | 4,622 |
| Campaign CPU time | 12.397 s | 9.794 s |
| Session elapsed time | 577.728 s | 577.763 s |
| Peak sampled tracker entries | 1,300 | 1,294 |
| Lower bound on expired tracker entries | 1,011 | 1,017 |
| Peak sampled broker identities | 4,382 | 3,070 |

Both directions reconcile between endpoints. The full supervised
run took 580.580 s. The outer host command reported a peak RSS of
187,064 KiB. The fixture samples after pruning every 64 Calls and
at completion. The sampled high-water figures do not establish a
continuous maximum. Each end had zero queued FIFO entries at the
final sample. Guest wall time includes pacing, while the host
campaign processing wall counter excludes idle receives.

This dose used 63,494,683 carrier bytes for 9 MiB of logical
data. The Raw mode, small chunks and two encrypted receipts per
Call deliberately stress the state lifecycle. This is a paced
retention and exactness result. WAN performance, warm savings,
fairness and sustained recovery with a carrier replacement each
need separate evidence.

The guest build reused the validated guest binary after the
reservation and diagnostic change to the test-only host module.
Its exact source manifest differs only at that host module.
