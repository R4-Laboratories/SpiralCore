# Independent packet-core review

Reviewer: `/root/mvm_tests`, acting as code-reviewer, security-reviewer and
rust-reviewer. **Verdict: PASS for this scoped source review; no material findings.**

Eligibility was verified with the installed roster script using the actual
SpiralCore worktree as `--root`: logical roster `d1927d9e7ce51354`, 13 active
agents and 14 skills. All three reviewer roles returned active. The local routing
table has no conflicting in-progress claim for these files; the author explicitly
released the production source and tests before review.

Reviewed source, authored by `/root/mvm_boundary`:

| File | SHA-256 |
|---|---|
| `crates/spiralcore-gateway-core/src/packet.rs` | `dad4fea3c5e5adfbcc520057baab495c54db2b176be7cad92887a729bb3d4a28` |
| `crates/spiralcore-gateway-core/tests/packet_selection.rs` | `8b9c712f1677effb2f90c8274e0a308135553e446be51b870c6e1eefd2bf4338` |

The review traced the diff from `4bce7ef` and surrounding encode/decode paths:

- Input is validated as a bounded IP packet before the advisory transport parser
  indexes it. IPv4 offsets/fragment flags, IPv6 extension-header fallback, TCP
  header lengths/control flags, UDP declared length and short payload handling
  are bounded. Unrecognized transport shapes force compression; they neither
  authorize a route nor bypass reconstruction checks.
- Adaptive history has at most 64 entries with deterministic eviction. Each key
  binds both addresses, protocol and ports; state is private to the encoder.
  The fixed 128-slot word table and full bounded payload scan do not allocate
  based on uncontrolled lengths. `no_saving` stays within 0–2 and `skips` within
  0–7; externally reported counters saturate.
- FastZstd retains one context and a buffer reserved to the maximum packet's
  compression bound. The fixed small window and input cap bound work; the 2 MiB
  context guard fails closed. The borrowed encoded buffer cannot be reused until
  its bytes have been copied into returned frames. Independent `compress2` frames
  introduce no previous-packet reconstruction dependency.
- Every mode still chooses Raw when the compressed body is not smaller, binds
  the original exact packet to the same digest, and emits the unchanged
  `SCGPKT01` wire format with at most two fragments. Raw and legacy Zstd retain
  their original codec path. Receiver frame/length/digest/IP validation and
  replay/reassembly behavior are unchanged by this patch.
- Tests cover exact recovery with reused encoders, both existing modes,
  negative-history skips, mandatory reprobes, immediate positive guards,
  bounded flow eviction, IPv6 and TCP classification, malformed/unsupported
  transport fallback, and a deliberately retained heuristic miss. A fresh
  decoder for each test packet also checks that encoding does not rely on a
  preceding packet.

Existing `core-all-green.txt` records nine passing core tests (the four prior
tests plus five selector tests), and `core-clippy.txt` records a successful strict
Clippy pass. Their exit receipts are zero. This review did not rerun builds,
benchmarks or tests, as requested; root owns final integrated verification.

The observed adverse byte regression is an explicit algorithm limitation, not a
fidelity defect or a universal byte-saving guarantee. Adaptive should remain
opt-in under the written contract. This review does not approve promotion past
the failed 1% framed-byte target and does not establish constrained-network or
real-WAN performance. The reviewer-authored profiling harness and its analysis
are excluded from independent source approval here.

## Receipt-deferral addendum

**PASS; no material finding.** This independently reviews the narrow repair
authored by `/root/mvm_boundary` after its release, not the full preexisting probe.

| File | SHA-256 |
|---|---|
| `crates/spiralcore-gateway/src/bin/spiralcore-gateway-probe.rs` | `18bdc36a479f798a11631df28d3049c797aa3dad8656301be5e798a690f472f0` |
| `crates/spiralcore-gateway/tests/probe.rs` | `95955d8eab689b2bb3eafbdcbf36342d7edb9f972d90f88b1fd81ff1b33f243b` |

The `report`/`write_report` split preserves existing single-transfer snapshots.
In `tcp_sequence`, lines 142–146 capture wall/CPU after successful exact exchange
and before saving the result. A preallocated vector holds only three timing
tuples and borrowed fixture references. Lines 148–156 perform all report hashing,
JSON construction and file I/O after the final exchange. No fixture is reloaded
or cloned between intervals; the same actual TCP stream and its sampled endpoint
addresses serve all three objects. Connect remains included only in the client's
first interval, and failures cannot emit a success report for an incomplete
exchange.

The external regression holds the next exchange after receiving an earlier
receipt and checks that no JSON appeared between transfers. The retained
`probe-deferral-red.txt` shows that test detecting the old behavior;
`probe-deferral-green.txt` shows all three real probe tests passing after the
repair. I inspected those receipts without rerunning the build. Per-object
application timing is now isolated from the prior object's reporting work;
gateway lifecycle CPU still legitimately includes activity outside those
application intervals and must remain a separate measurement.
