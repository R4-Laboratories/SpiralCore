# Verified role images for the addressed two-guest test

A real SDK launch of the earlier unpacked directories removed their image-authored launch controls during normal OCI injection. This qualification produced one sealed ext4 file per role on Linux x86_64. In both images, the MVM-owned OCI wrapper starts the selected workload, the runtime grants one serial worker with a one-call queue, a 64 MiB RSS recycling threshold and a 100-call recycling limit, and the guest requires a launch-provisioned verb grant. [Original launch record](../addressed-native-launch/README.md) · [Rust builder and reproduction commands](../../../../../tools/installed-mailbox-witness/operator-images/README.md)

| Role | Workload bytes | Sealed ext4 bytes | Initial build wall | Initial user + system CPU | Reviewed rebuild wall | Reviewed user + system CPU |
|---|---:|---:|---:|---:|---:|---:|
| Source A, mailbox producer | 717,576 | 1,470,464 | 0.05 s | 0.08 + 0.00 s | 0.04 s | 0.06 + 0.01 s |
| Destination B, Cast recipient | 708,576 | 1,458,176 | 0.05 s | 0.08 + 0.00 s | 0.04 s | 0.07 + 0.00 s |

Times are single host image-preparation observations at the resolution of GNU time. The reviewed build produced the same eight files byte for byte. Independent ext4 and full dm-verity readback of that pair took 0.05 s wall, 0.02 s user and 0.01 s system. The images use a runtime wrapper with SHA-256 `64e8b2577db974fbf716ca344a50c39bac5e989beeeff68aa6682effd76de5e9`. Both prepared role directories had mode 0700. The reviewed builder sets that mode atomically. [Image, tree and sidecar digests](images.sha256) · [Extracted guest contents](verified.json) · [Compiler, source and artifact identities](build.json)

The injector discarded planted input copies of the entrypoint, image runtime, worker policy and a sentinel. Only the operator generated the worker policy, using the guest's own Rust type before sealing. Independent readback verified that the source image contained only the source executable of the pair and that the recipient contained only the recipient executable. The verifier also refused an inherited `PYTHONOPTIMIZE=1` before producing output.

This packet establishes the contents and integrity of the images handed to the launcher. It contains no guest execution, application readback, network byte counts, transfer latency or WAN comparison. [Mailbox acceptance tracker](https://github.com/R4-Laboratories/SpiralCore/issues/43)
