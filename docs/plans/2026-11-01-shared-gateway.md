# Bounded shared-gateway implementation

Approved interfaces: [shared object/packet design](../specs/2026-11-01-shared-gateway.md),
[packet header schema](../contracts/spiralcore-gateway-packet-v1.schema.json), and
[peer transport design](../specs/2026-11-01-gateway-transport.md).

1. Freeze the shared object API and packet/transport wire contract. Root owns
   dependency selection, crate manifests and transport/CLI integration; source
   owners agree on individual module boundaries before implementation.
2. Add a failing object-facade test that sends through the existing durable
   `Endpoint`, verifies exact reopened bytes, a current exact-repeat acknowledgment,
   Raw/Whole parity, and scope refusal before exchange. Observe RED before adding
   the small facade. Reuse the existing core and immutable fixtures.
3. Replace the private sender enum in the native MVM campaign with this facade.
   Preserve canonical send/open/reply verification, sender and receiver CPU
   measurement boundaries, separate runtime cases and the existing packet-free
   mailbox channel. Run focused tests and the existing native MVM guest smoke.
4. Implement the bounded independent packet codec, strict IP length checks and
   datagram sizing/reassembly contract with focused malformed, loss/reordering,
   truncation and exact-byte tests. Implement the authenticated peer transport
   and Linux TUN adapter behind this boundary. QUIC v1 is datagram-only; disable
   reliable streams. Reliable object transfer remains the shared facade over the
   existing MVM canonical channel and durable test harness, not a new QUIC service.
5. Run ordinary host-to-host peer tests first. Run actual Linux TUN qualification
   in an isolated GitHub Actions environment; do not alter workstation routes or
   build a local Linux VM. Record the exact source, executable/environment and
   workload identities. Distinguish real TUN execution from socket fixtures.
6. Compare lossless packet Raw/zstd selection and reliable object Warm against
   equally configured whole-object zstd. Record both-direction transport bytes,
   CPU and total encode-through-verified-commit time; include setup separately.
   Packet delivery has no durable commit and must be labeled separately. Publish
   retained results and limitations even when a workload has no savings.

No broad audit, full coverage rebuild, unrelated cleanup or new production route
is part of this milestone. Root coordinates Cargo jobs (two) and measurement
windows. All retained non-success outputs remain reviewable.
