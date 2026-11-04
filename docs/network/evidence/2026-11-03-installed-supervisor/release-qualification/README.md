# Final installed-supervisor qualification

Product PR [#71](https://github.com/R4-Laboratories/SpiralCore/pull/71) merged at
`7e7479835a3150125b903bacc55b28bf765fbd28`. Reviewed head
`922414932488256f4e9eda3d47760604089f3fc8`, hosted synthetic merge
`dd3836a807e3b6ba12946750ad111aa66cc3b73b` and the actual merge share tree
`368679d28580e460ed69ede5d43506e702a4802e`.

The [independent final review](independent-final-review.md) approves this bounded
slice. It preserves the independently reviewed worker channel, Windows storage,
macOS clean-EOF correction and previously merged QUIC completion correction.
Later canonical resident-parent, owned-helper and endpoint work is excluded.

## Executed hosted gates

All **15 relevant checks pass**. Two manually gated runtime-object workflows
are **SKIPPED**, not executed successes. Original metadata is in `pr.json`.

| Scope | Executed result |
|---|---|
| Seven native portable rows | Linux x64/ARM64, macOS ARM64/Intel, Windows 2022/2025 x64 and Windows 11 ARM64 pass |
| Private object storage / endpoint | Unix rows each run three private storage and 21 exact endpoint checks; Windows rows each run four outer private storage and 20 endpoint checks, plus two isolated backend primitive checks |
| Unix owner components | Four native Unix rows pass their declared 11/12 entries; helper entries are not additional VM boots |
| Linux live | 142 release checks and 138 shared object-v1/mailbox checks pass, with strict Clippy/default guest/analyzers; scopes overlap |
| Candidate transfer campaign | 18 sessions, 336 operations, 288 exact reopened deliveries; capture, CPU invariants, cleanup and unchanged host routes pass |
| Isolated TUN campaign | 12 sessions, 48 exact flows; capture and unchanged host routes pass |

Seven original portable ZIP digests and **294 inner hashes** verify. The exact
platform manifest and adapter lock match Unix LF and Windows CRLF checkout
hashes. The initial unaccepted verifier targeted the wrong manifest, then
compared Windows checkout bytes directly with LF Git blobs; `custody.json`
records both corrections. No test or source was changed for those checks.

`portable-rows.tar.gz` losslessly retains all extracted portable records, their
original manifests and original ZIP identities. `candidate-json.tar.gz` and
`tun-json.tar.gz` retain the original JSON records; the separate archive
inventories retain every original member path, length and SHA-256, including
members not copied into these compact JSON selections. `linux-log.txt.tar.gz`
retains the original hosted Linux output. These archives are compact derivative
records, not the original GitHub artifact ZIPs. Executable/payload/pcap members
were not copied here. The original artifact IDs and ZIP hashes are explicit.

The historical TUN capture failure at run41757732453 and the failed old
candidate's terminal-acknowledgment cell remain retained in the parent packet.
These subsequent successes neither erase nor explain the earlier capture gap.
The QUIC correction and its separate negative/loss limits are recorded in the
[object-finish packet](../../2026-11-03-object-finish/README.md).

## Coverage and source limits

`coverage/summary.tsv` reports changed mapped Linux production lines
**1002/1179 = 84.99%**, broader mapped scope2302/2703. It compares base86cae0a5
with the frozen f1f30b8f implementation. The source manifest, completed source
verification, protected executable identities, original test output, build and
native instrumentation receipts, LCOV and Rust summarizer are retained.
`coverage/selective/owned.lcov.gz` losslessly contains the original LCOV.

The summarizer explicitly excludes test-only ranges. Its source preserves the
exact mapping rules and unhit lines. `host.rs` is **UNMAPPED** in this Linux
scope; native macOS receipts remain separate. This is not Windows/macOS line
coverage, whole-repository coverage or branch coverage. LLVM reports65 function
data mismatches; both original warnings remain in the export receipt. Raw
profiles and protected instrumented binaries stay in the owned Wick evidence
directory. The native instrumented cells are coverage evidence and must not be
used as performance observations. Original interrupted/debug attempts remain
separate from the final selective release collection.

The later Windows extraction, macOS EOF and QUIC changes have their own source
pins, focused tests and reviews. This Linux coverage percentage does not
silently extend to those changes. The actual local native transfer results,
their source snapshots and limitations are in the
[results report](../../../installed-supervisor-results-2026-11-03.md).

## Remaining delivery gates

Canonical VM/worker/helper ownership, aggregate plaintext/cache/workspace
partitions, macOS installed death recovery, completed-tail recovery, production
sessions beyond2048 Calls, endpoint integration, multi-tenant fairness and
saturation, and canonical direct/relay WAN/platform acceptance remain open.
Native Windows storage/client tests do not establish a Windows MVM host.
No new speed, send-only, energy, certification or full installed-service
completion claim follows from this release record.

All files in this directory except `SHA256SUMS` are covered by that manifest.
No payload store, VM disk or new local build tree was retained for publication.
