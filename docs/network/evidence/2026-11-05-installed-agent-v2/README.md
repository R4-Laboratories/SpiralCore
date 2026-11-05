# Installed mailbox recovery with the booted guest agent

One Linux x86-64 Firecracker v1.17.0 guest ran the canonical installed mailbox
Recovery V2 witness using the original signed portable bundle. The booted
guest agent applied the Vsock filter before directly starting the workload.
The workload checks for seccomp mode 2 and IPv4 socket refusal before opening
credentials or dialing. It contains no filter installation. The normal
carrier and its authenticated replacement used the same VM and mailbox
authority.

The guest sent one exact 65,023-byte Call and received its exact 65,023-byte
Reply. After closing the first carrier before its final acknowledgement, it
reconnected and compared the replayed encoded Reply byte for byte, then sent
the original acknowledgement. The host verified receipt and release of the
cached reply. A subsequent 23-byte Cast and receipt completed. Port 5257 was
the dedicated mailbox binding. There was no guest NIC.

| Measurement | Observed |
|---|---:|
| Guest → host, both carriers | 131,251 B |
| Host → guest, both carriers | 252,151 B |
| Bidirectional total | 383,402 B |
| Guest / host exchange thread CPU | 9.870 / 8.181 ms |
| Guest / host exchange elapsed | 18.157 / 17.589 ms |
| Single guest through original retirement | 2.582 s |
| Complete supervised run | 2.829 s |
| Retained runtime allocation | 107,339,776 B |

The wire counts cover both directions of both mailbox carriers, including
their authentication and framing. They are application transport counts.
This run did not include WAN traffic or a matched compression baseline.

The native harness and the outer supervisor exited zero. The original
owner retired the guest, its mailbox socket and vsock bridge, and released
staging reservations. The runtime stayed charged under the separate retained
cache reservation. Final observation found no Firecracker process or
listening socket for the test. The run retained the original 1 GiB guest,
133 MiB managed startup reservation, 224 MiB staging ceiling, and 112 MiB
retained runtime limit.

The preceding no-boot preflight verified the official base image, sealed
the exact contributor x86-64 runtime in a private publisher-signed schema-4
bundle, rejected that bundle under empty trust, then verified and installed
it under the isolated trust store. The resolved initramfs, overlay and
verity tree matched the direct-agent test bytes. The manifest retained
authenticated Linux image declarations and removed unrelated retired
runtime and SDK rows. The native run generated a fresh bundle for its own
private enrollment and verified its digest after retirement. Fixture
preparation precedes the managed startup reservation. The no-boot image
tool peaked at 170,672 KiB RSS, and the outer native host command reported
189,752 KiB peak RSS. Those process measurements do not establish a
production aggregate memory limit.

This is one privately signed Dev-lane installed witness. Production
publisher qualification, multiple guests, sustained operation, WAN
acceptance, and FIPS qualification remain separate gates.

[Portable input evidence](../2026-11-05-portable-archive-inputs/README.md)
