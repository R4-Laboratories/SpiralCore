# First 2,049-transfer Firecracker trial

Two original Linux x86_64 Firecracker guests completed 1,480 contiguous transfers before the reader in guest B reached its deadline on ordinal 1,481. A and B used the same installed executables, sealed guest images, grant limits and sender profile as the 65-transfer trial. Each completed row contains a distinct original message ID, a signed B receipt verified by A, independent B readback and ACK, and A's settlement of that receipt. Each B Empty control ran after A staged the ordinal and before A submitted it. [Source and installed identities](native-65.json) · [Measurements and original digests](native-2049-initial.json)

| Observation | A | B |
|---|---:|---:|
| Guest application CPU on the 1,480 completed rows | 643.777 ms | 901.196 ms |
| Original host port 5257, completed exchanges over process lifetime | 2,962 | 4,442 |
| Port 5257 bytes read from guest / bytes accepted for write to guest | 1,969,125 / 2,481,267 | 4,436,983 / 17,536,819 |
| Port 5257 read / write errors, lifetime | 0 / 1 | 0 / 0 |
| Original helper exit on first stop, unassisted | 0 | 1 |

The completed prefix delivered 380,360 application bytes (257 per transfer). It moved 8,664,790 sealed bytes toward B and 1,790,525 signed response bytes toward A. Median and p95 time to settle were 0.784 and 0.858 seconds. The full process took 1,165.72 seconds, including startup, the failing operation and teardown. GNU time recorded 224.86 user and 17.74 system CPU seconds with 83,908 KiB maximum RSS. The driver’s own CPU counter recorded 43.619 seconds. These accounting scopes overlap. Both port observers closed. Both original scopes were inactive with absent cgroups, and the audit verified 17 entries across two sealed sessions.

### Last durable operation

B's original anchor ended at cursor 4,442. Record 4,441 accepted ordinal 1,481 and 4,442 leased it. The preceding three records completed ordinal 1,480. No later signed ACK or checkpoint appeared. The application reported `guest reader socket: MVM reader deadline`. The available trace cannot distinguish waiting for the lease response from waiting for the ACK response. The B helper exited with code 1. Its stderr was discarded by the launcher, and the capture does not establish when it exited relative to the reader timeout.

| B cursor | Record bytes | Total journal bytes |
|---:|---:|---:|
| 4,438 | 11,109 | 33,533,277 |
| 4,439 | 7,334 | 33,540,611 |
| 4,440 | 4,192 | 33,544,803 |
| 4,441 | 11,077 | 33,555,880 |
| 4,442 | 7,338 | 33,563,218 |

At cursor 4,441 the journal crossed its 33,554,432-byte maintenance threshold. The pinned implementation tries to encode all protected records in one checkpoint, subject to a 16 MiB record maximum and a 64 MiB journal quota. The signed retention window still protected these records. A subsequent deterministic component test reproduced a fatal automatic checkpoint refusal under space pressure. The native run did not capture the helper's exact error. This trial covers one guest pair on one Linux host. [Sender retention](https://github.com/R4-Laboratories/SpiralCore/issues/195) · [Sustained capacity](https://github.com/R4-Laboratories/SpiralCore/issues/49)

### Per-transfer records

The downloadable gzip contains every completed original JSONL row (1,480 rows, 2,927,846 bytes after decompression). Each row exposes measurement counters, elapsed times, a synthetic message ID, and a digest of the verified B receipt. The JSON manifest gives SHA-256 for both representations. [Download original rows](native-2049-initial-rows.jsonl.gz) · [Verify hashes](native-2049-initial.json)
