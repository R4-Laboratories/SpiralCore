# Exact-source hosted qualification

Both workflows passed at reviewed head
`5c41d6ac2d9b40e6eead62e201d5e9b020f96019`:

- [Linux host contracts, run 41629079360](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41629079360):
  92 release-stage checks, 124 shared-stage checks (eight object-v1 and 116
  adapter), strict Clippy, default Linux guest entry and synthetic analyzers.
  [Original log](linux-run.log) and [job metadata](linux-run.json) are retained.
- [Portable clients, run 41629079515](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41629079515):
  81 checks on each of seven native client rows, including 21 receipt checks,
  plus eight Unix storage executions across four rows. Original seven ZIPs
  total 188,522 bytes; all API digests and 236 inner checksums pass.
  [Independent verification and ledger](portable/verification.md).

The checkout was tested merge `3e7c94b23cfc4a8b1bdf0bc09cb7d8e832619527`.
Its parents are base `231ab321...` and reviewed head `5c41d6ac...`; its entire
tree `ea05391bd4f077339a6338a102b68e62d83f8815` equals the reviewed head tree.
Original Windows CRLF bytes are preserved and checked against the corresponding
Git blobs. No archives were normalized or silently replaced.

Execution counts overlap other stages and rows. Zero-test summaries and all
zero failure/ignored/measured/filtered flags remain in the ledger. Native
Windows/macOS/Linux clients and TCP-loopback components are distinct from native
MVM backends, durable Windows storage, WAN or fleet capacity. Native guest
measurements are separately recorded in the parent packet's `native/` directory.
