# Independent FlowMux guest readback from the sealed package

On 2026-11-14, read-only inspection recovered the actual guest executable from
the ext4 root filesystem inside the exact signed package used by the
[rc.3 installed FlowMux witness](native-rc3-fixture.md). Its **1,328,904 bytes**
matched the original staged executable byte for byte, SHA-256
`fe6d14c0168c3b7e613a73be8789e8f459f8f844211a0ace75bdd41b6e0e4648`.
This supplies the previously missing independent sealed-ELF observation in
[#261](https://github.com/R4-Laboratories/SpiralCore/issues/261).

The complete package remained SHA-256
`498bc217f54b394821394afcc6bc10ae5c836d0cb276658a25b5ffef02f44d8d`,
63,859,200 bytes. This is the earlier canonical MVM-authenticated package,
not a newly admitted workload or a new official publisher. The out-of-band
public key matched recorded development publisher ID
`51c09c23ddf4710f4196ee4987efc3f3`.

## Checks and identities

| Read-only check | Result |
| --- | --- |
| Original archive hash against the native report | Exact match |
| Independent Ed25519 verification of archived manifest with OpenSSL | Passed |
| One-bit change in the detached signature | Refused |
| Rootfs member size/hash against the signed manifest | Exact match |
| ELF read through debugfs from `/usr/local/bin/spiralcore-flowmux-proxy-guest` | Exact original bytes |
| Operator environment against signed environment member | Equal |
| `/etc/mvm/image-runtime.json` argv against signed entrypoint argv | Equal |
| `/etc/mvm/entrypoint` marker | Canonical OCI wrapper path |

The rootfs contains 2,064,384 bytes, SHA-256
`6c2f32290d2267500ac0a5f74ef7f1125b03c36351d211936ba5d7bc0b0a83fe`.
The check copied only that member to an automatically removed private temporary
directory. It did not mount or change the image, execute guest bytes, read
private keys, boot a guest, or change the original package. OpenSSL 3.5.5 and
debugfs/EXT2FS 1.47.2 performed the independent checks. The original canonical
MVM signature, policy, revocation and admission checks remain the authority;
this auxiliary inspection does not replace them.

For the core readback, with `SC_FLOWMUX_PACKET` set to the retained operator
packet directory, the executed tool operations are reproducible as follows:

```bash
set -euo pipefail
packet=${SC_FLOWMUX_PACKET:?set the retained verified packet directory}
inspection=$(mktemp -d)
trap 'rm -rf "$inspection"' EXIT
sha256sum "$packet/workload.mvmpkg"
tar -xOf "$packet/workload.mvmpkg" artifacts/rootfs.ext4 > "$inspection/rootfs.ext4"
sha256sum "$inspection/rootfs.ext4"
debugfs -R 'cat /usr/local/bin/spiralcore-flowmux-proxy-guest' "$inspection/rootfs.ext4" > "$inspection/guest.elf"
sha256sum "$inspection/guest.elf"
debugfs -R 'cat /etc/mvm/image-runtime.json' "$inspection/rootfs.ext4"
```

Compare the hashes to this report and the signed manifest, and compare all guest
bytes to the independently pinned staged executable. The structured
[results](sealed-guest-readback.json) retain the actual comparisons and
inspection-helper identity. Only synthetic public facts are published; the
operator packet and its local addresses remain private.

## Scope and measurements

Inspection took 56.728455 ms wall and 32.192425 ms of the inspection process's
CPU. Child-tool CPU is not included in that CPU field. These are artifact
inspection costs, **not transfer time or transfer CPU**. The earlier native
witness's 5,804 exact bytes each way and its measured timing remain unchanged.
This adds no WAN, sustained-capacity, FIPS or fresh admission claim. Delivery of
the underlying implementation PR #260 and the broader MVM acceptance remain
separate gates.
