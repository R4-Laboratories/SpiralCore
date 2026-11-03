# Exact-source hosted qualification

- Portable run: [41590624501](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41590624501), all seven native-process rows successful.
- Linux run: [41590624517](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41590624517), successful.
- Reviewed head: `753198531d4f3ef87e8a79c63d44f4f081ce4077`.
- Executed PR merge: `a0d02f5c6d2f56c9d717f7e7788248f0d2bf7cbe`.
- Both trees: `19f1cfe94ce45b92a086028acf91f1a2f54ee129`.

Each archived identity/rustc record verifies native process architecture, source
and pinned dependency revision. No cross-build or WSL row substitutes for a
native Windows process. The rows are Ubuntu 24.04 x86-64/ARM64, macOS 26
Intel/ARM64, Windows Server 2022/2025 x86-64 and Windows 11 ARM64.

The portable target count is 72 per row (504 total), plus two `canonical_storage`
tests on each of four Unix rows (eight total). Linux records 59 release-host
checks and 114 shared conformance checks plus successful strict Clippy/default
guest/analyzers. Overlap between scopes is retained. These are component and
loopback checks; the separate native Firecracker receipts carry actual guest
evidence. The host-budget module's new native execution evidence is Linux;
Windows client checks do not establish its packaging or durable gateway support.

Seven original ZIP digests match the GitHub artifact metadata and all 222
inner-file checks passed. `original-zips/EXPECTED-SHA256SUMS` contains the original
API digests, and each extracted artifact retains its original `SHA256SUMS`.
Windows manifests use CRLF; verification initially interpreted carriage returns
as filename bytes. Stream-only `tr -d '\r' < SHA256SUMS | shasum -a 256 -c -`
then verified the original payloads without altering any archive or manifest.
The `verified-members.txt` files are derived verification receipts, not archive
members. The original seven ZIPs have 229 members including their manifests.

Full compact status/step output and Linux execution records are retained. The
public artifact retention window does not control this permanent repository copy.
