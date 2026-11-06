# Restored PR72 native-component custody — PASS

[Run41784774137](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41784774137)
completed successfully on its first attempt: the matrix job and all seven native
role jobs passed. Its reviewed head is
`1e69a99d27691d1db9921568d834a98814defc81`. The artifacts actually tested merge
`86d598fbd218ecdbf2d0d03b56e56ca5f67eb689`, whose API-authenticated parents are
`e66caf5a30fcbcba525932bfa0b4a64d9112059b` and that head. Both head and tested
merge have full tree `94426f89602dc2714842198f9bc4dd7fa4fa8d8b`.

All seven original artifact ZIPs remain in `archives/` (46,294 bytes total).
Their seven API SHA256 digests and sizes match the original downloaded bytes.
All42 inner SHA256 checks passed, with six distinct expected files per ZIP.
`custody.tsv`, `inner-checks.txt`, the original API JSON and verification scripts
retain the checks. Derived extraction is disposable duplication; the originals
are preserved. Windows lock bytes are CRLF and match the frozen lock converted
to CRLF; Unix lock bytes match the original LF source. No source bytes were
normalized to conceal a mismatch.

Each role independently listed and executed34 tests across9 nonzero test scopes,
with0 failures/ignored/filtered cases. The seven role observations total238,
representing34 tests repeated across platforms, not238 distinct tests. Roles
were Ubuntu24 x64/ARM64, macOS26 Intel/ARM64, Windows Server2022/2025 x64 and
Windows11 ARM64. The artifacts identify Rust1.97.1 and their native target;
each contains its successful release Clippy log. `verified-rows.json` preserves
actual identities and counts, rather than assuming the workflow minimum.

This source includes the restored TX-lock barrier. The older33-test baseline
and rejected34-test intermediate remain separately retained; their CI or
coverage is not substituted for this source. Source qualification and restored
coverage are in the adjacent packets. This receipt adds hosted artifact custody
only: it ran no Cargo command, VM, native MVM backend, WAN test or performance
campaign. TLS/Mio/Small/Ordered work remains outside this frozen component.
