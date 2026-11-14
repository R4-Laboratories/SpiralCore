# Signed installed FlowMux fixture — original Linux guest

The normal signed fixture producer, retained private handoff and original installed runner now complete a one-guest Linux x86-64/Firecracker transaction. The sealed guest sends and receives **5,804 exact bytes**, including embedded and trailing zeros, and an unauthorized destination is refused before any application bytes. The original SDK and tenant owners retire the guest and workers; four identified persistent reservations remain charged deliberately.

This is a single-host functional and lifecycle witness. It supplies neither a WAN result nor a matched compression or zstd speed comparison. [Machine-readable measurements and source identities](native-signed-fixture.json).

## Inputs and execution

MVM source is `31f8a562f19d9eac89cb0d078b507770bb592262`; all 4,645 tracked source blobs and modes were checked before building. The matched three host helpers were built from that MVM source. The controller, installer and peer were built from SpiralCore `0669c8f4db439f804cf26119a6d971b5015e0d94`. The guest and repaired export/handoff test use `757b41913f1b2e46ddf18392551fcf27020432ae`. The latter delta adds the opt-in private handoff and diagnostic observer; this run does not enable the stall feature. Binary hashes are in the JSON rather than inferred from filenames.

The genuinely signed 62,385,152-byte package has SHA-256 `28b7c51b34933bf51f271f167b8240edae23e79d228bcd497b55197094f26b95`. The guest ELF has SHA-256 `7cd3aedb078148902adfda4d7950839316df68f018ea18dc80888603db2c606d`. The canonical runtime is **0.28.0-rc.2**; Firecracker is verified **1.17.0**. This disposable development publisher gains no official publisher authority.

Before the native run, the original five handoff controls passed, strict fixture/handoff Clippy passed, and the real signed-export test passed 1/1 in 15.59 seconds. The successful export process retained its private workspace. Provisioning reopened its original current-owner 0700 root, matched device/inode/UID, checked the archive against both the ready record and successful public export result, checked the literal compiled IPv4 against an actually assigned host address, and placed distinct issuer and peer private keys in separate 0700 role directories. Only public key material crossed between roles. Private paths, addresses, keys and signed operator packets are excluded from the published report.

Execution used the existing `tools/installed-flowmux-witness/run-linux.sh`, with one CPU/256 MiB guest, no guest NIC, a new finite 512 MiB filesystem in a private mount namespace, 768 MiB controller and 192 MiB peer address-space ceilings. The enclosing user scope had a 2 GiB aggregate memory cap, 200% CPU quota and 900-second deadline. KVM access was supplied only to the non-root test process's supplemental groups; no account, global KVM permission or GPU setting changed. Builds were complete before the native transaction.

## Recorded results

| Boundary | Actual result |
| --- | ---: |
| Authorized guest application bytes sent / received | 5,804 / 5,804, exact comparison |
| Unauthorized destination application bytes | 0 sent / 0 received |
| Cooperating connection TLS receive / transmit | 3,664 / 3,234 B; **6,898 B total**, both directions |
| Guest authorized proxy operation | 120.258685 ms wall; 1.099387 ms guest process CPU |
| Guest refused proxy operation | 3.058270 ms wall; 0.754944 ms guest process CPU |
| Peer accept through verified HalfClose | 113.005782 ms |
| Controller entrypoint dispatch | 153.992795 ms |
| Controller prepared phase marker | 944.875902 ms |
| Controller launch phase marker | 6,178.563302 ms |
| Controller start through SDK retirement | 6,496.494804 ms |
| Complete controller command | 6.62 s wall; 5.12 s user + 0.26 s system = **5.38 s process CPU** |
| Controller maximum RSS | 275,088 KiB |
| Peer command | 6.41 s wall; user/system CPU each rounded to 0.00 s; maximum RSS 10,144 KiB |
| Native test / peer / mount namespace | 1/1 test passed; all three original exit statuses 0 |

The phase markers have the boundaries defined by the controller; they must not be added together as disjoint phases. CPU clocks belong to different processes. The controller measurement includes setup and children reported by GNU time; the guest's process CPU is separate. Maximum RSS is not simultaneous aggregate host memory. TLS counts omit lower-layer headers and retransmissions, so they are not full physical-network traffic. This small run does not establish a throughput ceiling or statistical latency distribution.

## Independent evidence and cleanup

The original owner verified the original audit registration/terminal inventories and signed chain before archiving, with device/inode custody. A separate existing canonical copied-chain verifier then verified the retained 2,336-byte audit: **two signed entries, one matching allowed destination and one matching denied destination**, bound to the original tenant, VM, session and host verification key. Raw audit SHA-256 is `5f193a98c2a8ae2cc04e8e27e24d6b6908cc85d124ebdcfdb3530e635a957c59`. The archive trailer, adopted raw digest and equal registered/terminal/original-host keys were checked independently. This copied-content check does not replace the original owner's descriptor and lifecycle evidence. The verifier executable and its older matched library closure are identified in the JSON; they do not qualify the new runtime independently.

After the original runner exited, independent kernel/process checks found no live matched guest/controller/helper executable, no backing loop attachment and no original mount. Only then was the owned 536,870,912-byte disposable filesystem removed. The private prepared package remains available for subsequent bounded tests. No raw VM disk or key material is committed.

The shared ledger reports zero guests, workers, guest memory, carrier, codec and plaintext allocations after retirement. Cache, installation metadata and staging remain conservatively charged; `all_resources_released=false` is expected and was required by the test. All four original retained reservations were verified. This is not a claim that persistent installation storage has been erased or released.

Still required: the controlled current-upstream join and separately authenticated rc.3 successor, paired guests and failure/recovery on that source, sustained mixed control/data load, canonical cross-host carrier acceptance, platform cells and capacity/security qualification. No new WAN, production, FIPS, CUI or energy qualification follows from this witness.
