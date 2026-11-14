# Current-source paired transfer; protected retirement remains failed

Two Linux x86-64/Firecracker workloads completed the transfer substage on contributor MVM `468661f4`, using SpiralCore guest inputs from main `2ec1d65a`. The original pre-stop recorder reports **257 exact application bytes**, a 313-byte SpiralCore frame, B application ACK and a verified `AcceptedLocalDurable` receipt. The overall witness **failed retirement** and reached its 15-minute outer deadline. It is not a completed service qualification. [Exact source, artifact and record hashes](native-retirement.json).

| Recorded transfer boundary | Result |
| --- | ---: |
| Exact body / SpiralCore frame | 257 / 313 B |
| Counted sealed forward / signed return frames | 5,792 / 1,192 B; 6,984 B combined |
| Original SDK Call to observed V2 receipt | 135.872032 ms |
| Same start through verified destination Call | 1,066.808254 ms |
| Two-guest admission setup | 5,169.336293 ms |
| A application wall / process CPU | 0.682419 / 0.255487 ms |
| B application wall / process CPU | 887.706737 / 0.632639 ms |
| SDK A request / response | 800 / 192 application B |
| SDK B request / response including readback | 81 / 792 application B |

These are counted relay frames and SDK application bytes at separate boundaries, not a sum of all physical/vsock traffic. Guest CPU excludes host relay, cryptography, journal and lifecycle CPU. The receipt interval ends at host verification; the destination interval also includes B's returned readback. Source receipt retirement/recovery and WAN-byte fields are null. There is no matched raw/zstd control, send-only measurement, WAN, sustained-load, capacity or FIPS result.

The source recorder compares the hex-decoded returned bytes directly with the original body before publishing `verified_body_bytes`. The 1,112-byte pre-stop record has SHA-256 `6febc0b146ffa7edb8acc4648e756b5ffa76d00e67ea9330f07f6cba92c168b2` and was synced before shutdown began. The final combined result is empty because retirement did not complete.

## Runtime and image provenance

The guest applications were rebuilt against main's exact library graph with guest lock SHA-256 `3ba5d669084e928a090ef74005476620cfc51cd1a0ab7219624cfd503b4b22ef`. Both pass canonical static-ELF validation. Independent read-only ext4 extraction compared each embedded executable byte-for-byte with its frozen input. Each executable is 1,950,736 B and each sealed rootfs is 2,691,072 B. All ELF, rootfs, verity and root hashes are in the JSON.

MVM's existing source-runtime builder produced archive `b39c95d09c7b3e2aec9bdcc44c052df307e571d8808411953727139587fc2be2`, overlay, initramfs and two SDK sidecars from contributor `468661f4`. This is a **contributor-built runtime**, distinct from the authenticated official rc.3 candidate in the earlier FlowMux report. Its cold preparation cost is retained separately; it is not transfer time. Kernel acquisition used the normal CLI download path and recovered the previously qualified published kernel, SHA-256 `1701ccb2c46b2497066225d6ba05f3f33a3a845ca636fbeedb202835c83b49b6`.

Fresh installations reused immutable public runtime/kernel/cache artifacts only. Keys, grants, machine definitions, audit and revocation authorization state were not copied; canonical preparation and kernel-policy acquisition ran for each fresh home. Each guest requested one vCPU, 256 MiB and no guest NIC. The controller service capped CPU at two cores, memory at 2 GiB and runtime at 900 seconds; MVM also applies its per-guest scopes. These controller limits are not a measured aggregate host-memory result. KVM group access was process-only; global account/device/GPU configuration was unchanged.

## Retained startup failures

The first run refused before guest startup because the privilege wrapper omitted the user-session bus environment required for resource scopes. The next run used the correct session but reported Firecracker exit before readiness. A no-VM probe established that the version-named verified executable has Linux comm `firecracker-v1.`, while the generic driver's legacy liveness check requires `firecracker`. The same bytes under the conventional private basename satisfy that check. The second run's actual Firecracker exit cause is not established; its error is not evidence of a kernel panic or OOM. Both failed installations remain preserved.

The successful transfer substage used the conventional private executable name and the proper user-session environment. This is an installation constraint, not a repair of the legacy name-only liveness check.

## Retirement failure and safe disposition

Both original stop attempts reported `runtime state protection refused`. Both VM state directories retain telemetry records, their key files and collector status. The protection lock and directories satisfy the observed owner/mode/link checks. The generic SDK finish callback attempts runtime-state reaping, which correctly refuses these protected entries; the journal lifecycle lacks the complete original telemetry-owner retention/disposition integration needed before it can finish.

Source control flow places this refusal after the original VM/helper stop-and-collection stages. That is source-supported attribution, **not an exported successful helper-status witness**. No complete unassisted retirement or clean installation is claimed.

The outer service eventually ended with result `timeout` and signal TERM. Its shell EXIT file misleadingly contains 0; the authoritative service/command result overrides that marker. No result is inferred from the zero-byte final JSON. After the deadline, independent executable-device/inode checks found no live owned frozen processes. Late absence does not replace original collection or normal retirement. The records and keys were preserved; no telemetry was deleted, renamed or chmodded to pass cleanup.

The next implementation must carry original collector custody, bounded parent-funded retention and a typed stopped-with-retained-state outcome through the canonical SDK path. [Proposed integration decision](../../proposals/2026-11-14-journal-telemetry-retirement.md) · [Original-owner task #111](https://github.com/R4-Laboratories/SpiralCore/issues/111). Clean/recovery reruns, sustained operation, canonical WAN and final upstream submission remain open.
