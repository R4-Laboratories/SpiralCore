# Installed collector pair: development qualification

This opt-in Linux x86-64 witness carries the existing clean 257-byte journal
transfer through the original installed collector and shared parent. It is a
development tool, not a completed native retirement result or a new protocol.
The original enrollment, canonical mailbox grants, receipt verification,
exact readback and bidirectional byte meters remain the data path.

Use a coherent contributor MVM checkout containing `0835ef0ad` at `../source`.
The optional `spiralcore-mvm-live` dependency must resolve to that same MVM
package family through an explicit Cargo patch configuration. The contributor
host pins SpiralCore `4ea98b9e`; its transfer/FlowMux packages and their path
dependencies must remain one coherent family. Do not combine the newer host's
owned-store APIs with an older codec or create duplicate authority/codec types.
Record the patch configuration, source hashes and metadata package identities
with each execution. The tool does not update product dependency pins.

The three tools are:

```text
collector-install init
collector-fixture SOURCE_ELF SOURCE_SHA256 DESTINATION_ELF DESTINATION_SHA256 NEW_OUTPUT
collector-install install PACKET EXPECTED_PUBLISHER_HEX
paired-journal --installed-collector --backend firecracker A_ROOTFS A_SHA256 B_ROOTFS B_SHA256 FRAME
```

`collector-install init` explicitly initializes the canonical development host
signer under an absolute private `MVM_HOME`; it prints only public identity.
Fixture production requires canonical signed boot assets and current applied
revocation proof. It publishes two distinct signed role bundles with sealed,
deny-all environments, one CPU and 256 MiB per role. The installer requires an
externally selected publisher key. It verifies both archives and their current
revocation binding before installing through the canonical bundle registry.
It never adopts packet-supplied private keys or applies packet-supplied
revocations. A cached manifest cannot replace its authenticated declaration.

The pair requires installed registry rootfs paths and a private bounded
`collector-operator.json` containing the two authenticated role pins and archive
extents. Each executed rootfs must match its signed root declaration's exact
length and SHA-256 before native dispatch. The clean mode refuses mixed
object/recovery/ordinal campaigns; startup failure does not fall back to ordinary
SDK startup.

The declared witness envelope is a shared 2 GiB host resident budget, 1200 MiB
staging, 640 MiB memory and 512 MiB storage per startup quote, 192 MiB per tenant
helper process and 96 MiB metadata + 24 MiB stack per collector partition. Each
partition has four workers, one stream and 8 MiB + 32 KiB retained-record credit,
with zero codec/carrier/plaintext/staging allowance. These are funded ceilings,
not measured memory or fleet capacity. Both canonical preparation calls must
run in the same private finite disk-backed filesystem, at most 512 MiB, with a
hard controller address-space limit no greater than 640 MiB. A directory on the
host's unrestricted filesystem does not satisfy the contract.

Stop attempts both original owners, preserves failed custody, and transfers the
actual endpoint retirement receipt to the original parent before journal
marker clearance. Successful process retirement leaves original encrypted
records, wrapped keys, definitions and retained charges in place. No generic
reaper or deletion exemption is used. The 20-second per-owner journal deadline
does not establish a hard bound for every backend stage or filesystem sync.

Component checks currently include 23 paired-driver tests, six producer tests
and five installer tests. Strict Clippy passed for the three changed binaries
with all feature paths compiled. A clean-feature-only lint attempt retained
three existing unused object/ordinal helper diagnostics; an all-target attempt
also exposed an unused historical `own-mailbox` helper. These were not
suppressed. The existing enrollment assertion was updated to borrow its slice
for the current compiler lint. The individual producer and installer controls
boot no workloads. **Actual signed fixture production, complete native paired
retirement and failure/retry acceptance remain required.**

The first producer run exposed an invalid `/` environment working directory;
the descriptor now uses its canonical default. Its negative fixtures also now
explicitly make their temporary parent private under Wick's group-writable
default umask. Review additionally found and fixed the root-to-signed-bundle
binding and the installer's cached-manifest trust defect. These failed gates
remain part of qualification history.
