# Verify sealed Cast images under the guest service identity

Both original Firecracker guests booted during run 7. The protected SDK diagnostic for each role recorded `Permission denied` while reading `/etc/mvm/runtime.json`. The native report has SHA-256 `cd15be76099ac1bb81aed846bd170c62ec97025938f557b4e51dfddf95a7ecc9`; the permission failure occurred before transfer of the 257-byte fixture. The two guests used MVM host source `c492c52c`, guest runtime producer `9d7224cf` and SpiralCore driver `8f01c576`. Only the observed failure category is retained here. [Original sealed inputs](../operator-images/README.md)

An independent Linux check reproduced the failure on both original ext4 files. `runtime.json` was root-owned and readable (0444), but its parent `/etc` was root-owned with mode 0700. A privileged image reader could extract the file; the guest service could not traverse the directory. The same private staging umask had set mode 0700 on `/usr`, `/usr/lib`, `/usr/local` and `/usr/local/bin`.

The corrected Rust builder sets those five **guest directories** to 0755 before materialization. The OCI wrapper and selected role executable remain root-owned 0555. Host output directories are 0700 and the sealed images, verity files and metadata sidecars are 0400. Both roles retain the original ELF and wrapper hashes, the same single-worker policy and the original launch grant requirement. [Builder and reproduction steps](../../../../../tools/installed-mailbox-witness/operator-images/README.md)

| Role | Ext4 bytes | SHA-256 of verified ext4 | Reviewed image build, wall | Build user + system CPU | Service-identity verification, wall |
|---|---:|---|---:|---:|---:|
| Source A | 1,470,464 | `b61cd509acc27f3b091d01b4c5a3c0156665c0c59a5ff945abdd7491f7c251d6` | 0.05 s | 0.07 + 0.00 s | 0.02 s |
| Destination B | 1,458,176 | `a0b27ae7a19c4c1d0bb086c76c8e014bf32284791005ca0ce904ab392138ecdb` | 0.04 s | 0.06 + 0.00 s | 0.02 s |

These are single GNU time observations on Wick. Two builds produced identical ext4, verity trees, root hashes and sidecars. The service probe mounted each finished filesystem read-only in a private Linux mount namespace. At real/effective/saved UID/GID 1000, with no supplementary groups, no capabilities and no-new-privileges, it traversed the parent directories, opened every required configuration file, compared the ELF contents with the expected hashes, and checked effective execute permission. The independent verifier recomputed the entire dm-verity tree from each ext4 file. [Source credential and inode checks](a.service.json) · [Destination credential and inode checks](b.service.json) · [Eight artifact digests](images.sha256) · [Filesystem and verity readback](verified.json) · [Tool and receipt digests](v2-evidence.sha256)

The read-only mount test started no guest process or workload executable. The timing covers preparation and validation. Verified delivery, guest stream byte counts and end-to-end latency require an installed transfer run. [Tracked mailbox acceptance](https://github.com/R4-Laboratories/SpiralCore/issues/43)
