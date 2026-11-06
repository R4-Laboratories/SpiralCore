# Mixed installed FlowMux workload preflight

The next matched trial will use one authenticated FlowMux connection
for 1,245,184 bytes in each direction. The registered request contains
four 8,192-row segments. Segment two exactly repeats segment one.
Segment three differs from segment one by one byte. Segment four uses
a different seed. Each row is a 38-byte JSON line. The complete SHA-256
is `cb8735a26e7674a87de33c29ad7ec8ccc18167f41f727e4bca922c5f775e4951`.

The independent Rust generator built on native arm64 macOS. Its
fixture test checked the hash, size and segment relationships. The
protected Mac peer refused a synthetic control enrollment for the
wrong workload before TLS and created no object store. The launcher
requires the complete exact payload, expected setup-artifact set,
reader and store retirement, and original child exit at both ends.

The four segments describe application input. The actual stream
relay may split them differently. These checks establish fixture
identity and pre-admission refusal. Cache reuse, measured savings,
the Wick endpoint CPU sample and the mixed native runs require
execution on the original installed path.

The preceding Wick protected host refused the new witness mode
before launching a guest. The isolated successor build was started
on Wick. The configured SSH connection then stopped responding, so
the build terminal and three-arm native execution are pending.
The last measured installed comparison remains the
[11,600-byte cold run](../2026-11-05-installed-flowmux-matched-cold/README.md).

Apply the Mac patch to the peer from that comparison. The source
and binary pin identify this preflight, and the private launcher
hashes identify the prepared orchestration. The local component
tests do not include a real MVM data transfer.
