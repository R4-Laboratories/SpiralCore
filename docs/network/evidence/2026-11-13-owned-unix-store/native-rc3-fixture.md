# Installed FlowMux: current contributor host and rc.3 guest

The installed Linux x86-64/Firecracker runner passed on MVM contributor host `468661f47e01d0f59c6f1da4e35a968ceccb3ad2`, tree `ceea5a0d51e37394f256f365e7abb66ef66566cb`, and SpiralCore `0941abea5e1122bd594477931288eda9d7c556c7`. The workload sent and recovered **5,804 exact application bytes each way**, including zero bytes. The unauthorized destination received zero application bytes. The original SDK retired the guest and the tenant owner stopped its workers.

This is **one single-host functional and lifecycle trial**, without a matched zstd control. It is not WAN, sustained-session, capacity, FIPS/CUI or energy qualification. The [earlier rc.2 trial](https://github.com/R4-Laboratories/SpiralCore/blob/515b87bb9d26b8e4dca1036bef6437a191cf7aaa/docs/network/evidence/2026-11-13-owned-unix-store/native-signed-fixture.md) retains its separate source and measurements. [Current host workspace](mvm-workspace-qualification.md) · [Authenticated rc.3 assets](https://github.com/R4-Laboratories/SpiralCore/blob/515b87bb9d26b8e4dca1036bef6437a191cf7aaa/docs/network/evidence/2026-11-13-owned-unix-store/runtime-rc3-preflight.md)

## Signed export and execution

Five handoff controls passed, followed by the actual signed-export test. A fresh consumer checked the preselected publisher key, original package and signed revocation publication 2. The occupied-output case refused replacement and preserved existing bytes. The package is 63,859,200 bytes, SHA-256 `498bc217f54b394821394afcc6bc10ae5c836d0cb276658a25b5ffef02f44d8d`; development publisher `51c09c23ddf4710f4196ee4987efc3f3` gains no official publisher authority. The runtime is the authenticated **prepublication rc.3 candidate** from upstream tag commit `ffd90f895d7080629eceb16095d1ca81c0a49b95`, not a newly signed contributor runtime.

The export took 14.49 s wall, 11.10 s user and 0.42 s system CPU, with 70,064 KiB maximum RSS. Public export-result SHA-256: `49e4ccc89759598790e67abca07a1ae4fda61e77298ce2beda158615cb5bdd6c`; export-log SHA-256: `de939045b7a52a6872c9518049c6f6e702f8f69abd57637eb4c317535fbebb50`.

| Executed artifact | SHA-256 |
| --- | --- |
| Fixture producer | `3a156805e683da417cba15e79ee394687022f2c4aaaec25fa836f1db9ab680a3` |
| Export test | `86bef1a4094c337b347d498b04e42e7d1c1c35507decb78c40b9a6611eac259d` |
| Installed controller | `06b92547af169802a2a441f6e772b519c4eddac621f9b784b85f70c5a70b2c75` |
| Static x86-64 guest workload | `fe6d14c0168c3b7e613a73be8789e8f459f8f844211a0ace75bdd41b6e0e4648` |
| Installer | `70654d99146679fe562c13fd2fdc8affa6f9d255b398fa61e72fdd37c0d447d7` |
| Cooperating peer | `4abb3299ef090dcee726ef2331f9652eb51a7e115f567e5ed24636f24dc938f9` |
| MVM host agent | `c8cd9c06959458663e5c8a5d44b896f40ce63ff77298407e7b9486e8414bcb99` |
| MVM network endpoint | `06e571f14db348726142d5f0853a5c480ce5c896de86453c1b6da54146a11f5e` |
| MVM signer helper | `bfffbd6ccd360c9c771298ffabddff56cc5f62d53eae96bfbaccd553b465f51e` |
| Firecracker 1.17.0 | `99ad0f5cd0514a88aad0e9ae8cfdb3cc3b4ab9d190e1194602406c786b5de7a5` |

Before and after qualification, the MVM checkout matched 4,655 tracked entries and 64,659,163 bytes; the staged SpiralCore subset matched 556 entries. The controller uses its current path-patched core; the installer and peer retain the explicit codec Git dependency at `4ea98` in their standalone lockfile. Their wire format is unchanged, but these binaries do not all use the same core source revision. The host helpers were rebuilt from the current contributor host.

Provisioning reopened the original successful producer's private handoff with its recorded device, inode and owner, rechecked the archive identity, and used distinct issuer and peer private directories. Only public keys crossed between roles. Private addresses, operator packets and keys remain outside Git.

The existing `tools/installed-flowmux-witness/run-linux.sh` used one vCPU, 256 MiB guest memory, no guest NIC, a private mount namespace and a finite 512 MiB filesystem. Controller and peer address-space limits were 768 MiB and 192 MiB. The enclosing user scope capped memory at 2 GiB, CPU at two cores and runtime at 900 seconds. KVM access used process-only supplemental groups; no account, global device permission or GPU setting changed.

## Measured boundaries

| Boundary | Result |
| --- | ---: |
| Authorized application sent / recovered | 5,804 / 5,804 B, exact |
| Unauthorized application sent / recovered | 0 / 0 B |
| Peer TLS receive / transmit | 3,655 / 3,234 B; **6,889 B total** |
| Guest authorized operation | 117.517559 ms wall; 1.206312 ms process CPU |
| Guest refused operation | 3.600436 ms wall; 0.776305 ms process CPU |
| Peer accept through verified HalfClose | 110.540298 ms |
| Controller dispatch / prepared / launch markers | 154.829601 / 1,040.528482 / 6,440.327054 ms |
| Controller start through SDK retirement | 6,737.335161 ms |
| Complete controller command | 6.87 s wall; 5.40 s user + 0.30 s system = **5.70 s CPU** |
| Controller maximum RSS / virtual peak | 277,212 / 766,124 KiB |
| Original test / peer / namespace | One selected test passed; all three exits 0 |

Prepared, launch and through-retirement markers are cumulative from controller start; dispatch measures its own interval. These overlapping boundaries must not be added as disjoint phases. Guest process CPU is separate from controller GNU-time accounting. Peak RSS is not simultaneous aggregate host memory. TLS counts omit lower-layer headers, retransmissions and other host traffic. One trial supplies no latency distribution, throughput ceiling or compression speedup.

## Audit and original-owner retirement

Before archiving, the original owner verified audit registration, terminal inventory and signed-chain custody. A separate checker linked to the current canonical `mvm_hostd` verifier accepted the copied 2,336-byte chain: two signed entries, one matching allowed target and one matching denied target, with the original tenant, workload, session and registered/terminal key. Mutating one signature made the same verifier refuse it with `SignatureInvalid`. Archive trailer and adopted raw digest matched. Raw-chain SHA-256: `aad9fb3bb5c58ed9257cbad993251e423ad2cbc5f1896ce61980feb6eb3f773d`; checker SHA-256: `0dcceff0e1ea8247b2937eb4998f17c1cc6a864f5a87dcb99bd5d28d3500450e`.

The existing audit events' `fields.plan_id` contains the nil UUID, while the peer uses a SHA-256 plan identifier. **This field does not establish peer-plan binding.** Target, tenant, workload, session and original descriptor custody remain independently verified observations. The copied-content check does not replace original-owner lifecycle evidence.

After retirement, the ledger reported zero guests, workers, guest memory, codec-store descriptors, carrier, codec, plaintext and stack allocations. Four original persistent reservations were verified; charges remained 806,371,328 B host metadata, 9,494,528 B cache and 536,875,008 B staging. `all_resources_released=false` was expected. Persistent installation storage was not declared erased or freed.

Independent executable-inode checks found no original guest, controller or helper process. The backing loop mapping and original mount were absent before removing only the owned 536,870,912-byte filesystem. Private source packets, keys and audit evidence remain retained.

## Evidence custody and remaining acceptance

Raw evidence remains on the Linux test host in `sc-0941-native.zcFLz3t9`, run `native.x2zvhpJs`; the genuine export is `sc-0941-genuine-export.4IcPuQdW`. These identify retained private directories, not public download links. No VM disk or credentials are committed.

| Retained record | SHA-256 |
| --- | --- |
| Controller log | `3372de3f48060cf88c3477ce2cd0a4eebb2e3dd1a0ff33e40d41a382f44fe8eb` |
| Controller time | `45e54a1bffd0c835aac71b24f62e48999e79f4a5ebed15a225230a8fe48c13dd` |
| Peer log | `15b66860f83cdbad6a2036e3222a907a421f1fe9452e204674dcfb347fae2135` |
| Peer time | `aa9bcb75135f13ef807dc84e6fa19f26ffb23c74ccb00dd087dccf2fb25cd306` |
| Exit-status record | `d3ef8a56b4af5972564748666a59be39ad24d3cea7dcdf59a33e44bdf8554fc1` |
| Executable inventory | `e546e921abe07879c3d29367a77018ada79011449499a2bb27512ad1309d2fbc` |
| Drained-file inventory | `3292d3ac3d728c193251ba2c38101b2ad61d0af892876ac364ac555ad9551005` |

Remaining acceptance includes paired current-source guests and failure/recovery, sustained sessions beyond the earlier witness bound, large data alongside controls, canonical cross-host direct/relay transport, platform acceptance and a supported capacity/security envelope. Historical Windows deadline failures and upstream restrictive-umask producer defects remain open. The final upstream MVM deliverable is incomplete.
