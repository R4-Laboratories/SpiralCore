# Portable archive inputs for installed Firecracker qualification

MVM's existing legacy archive readers accepted a package made from the same
runtime that passed the direct guest-agent Vsock tests. The decoded overlay,
verity tree and initramfs matched the tested bytes. The parser also refused a
modified initramfs with valid gzip framing and refreshed transport checksums
because the decompressed content differed from its declared hash.

The deterministic tar archives total 14,230,948 bytes. The 12,772,913-byte
overlay package contains the four required runtime binaries from the same
contributor build. The initramfs package is 1,458,035 bytes. Both archives
remain in the isolated test workspace. This repository retains their hashes,
parser witness source and readback evidence.

These checks exercise archive integrity and compatibility with the parser.
They precede bundle signing or installation. The planned no-boot acceptance
step uses the existing schema-4/5 bundle exporter, a schema-2 local compatibility
manifest with exact source provenance, a private publisher and an isolated
trust store. It must verify, install and select the same runtime bytes under
the original signed bundle pin before the installed V2 test.

[Direct guest-agent result and scope](../2026-11-05-guest-agent-enforcement-native/README.md)
