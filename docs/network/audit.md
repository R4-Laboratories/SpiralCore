# SpiralCore / MVM reconnaissance and claim ledger

Audit base: SpiralCore `23380f00afb78a5712306f0001c6ad7499d5fbaa`; MVM `330abfa19ee30c33d444f14918669362f0d51f93`. Date: 2026-10-28 America/New_York. This records observed source, fresh checks and retained historical evidence separately. Later implementation status belongs in [delivery.md](delivery.md).

## Coverage

[coverage.tsv](coverage.tsv) inventories all 372 original Git blobs by identity and review depth: projects 119; archive 158; crates 20; docs 47; references 22; root 6. Source/export manifests were checked against pinned blobs. The 119 project files matched local UOR copies; 218 of 225 archive/docs/crate files matched local copies and seven were fetched at the pin. All 45 retained SHA-256 manifest entries matched.

The ten PDFs total 108 pages, including OCR of 21 image-only pages. Both ZIPs contain 62 non-directory members. Eight saved-card instances (six unique) were decoded without executing/restoring their programs; all 242 file-payload instances passed their recorded hashes. Cards contain executable source as well as state. Hash consistency is not trusted origin.

All seven webpage tabs were actually navigated: Prompt Trace, Network & Geometry, Language Grid, Symbol Registry, Bell MF Codec, Witness Lab and Scores. The deployed page matched the imported 1,026,033-byte source at SHA-256 `770142b4f343842cd0693d362d49f95272a5ded029d9682211a5215d64810fd8`. The browser already exposes Playwright; no add-in was installed. The original page reported 256/256 fixtures.

Limits: file/artifact/module review is not formal verification of every nested historical line. Some Dedekind formula glyphs did not survive PDF extraction; not every PNG/GIF preview pixel was inspected. Large inactive model patches received module/contract and relevant changed-behavior review. No archival program restoration, GPU experiment or historical WAN run was repeated. MVM received targeted architecture/source and issue review, not an every-file audit.

## Implementations and observed gaps

| ID | Evidence and conclusion | Disposition |
|---|---|---|
| OBJ-01 | Existing UOROBJ01 server absent from export but found in UOR object-store core. Exact length, full SHA-256, ternary records, CLI and vectors already exist. | Import from original export pin; retain MIT/provenance. |
| OBJ-02 | `peer::pull_one` originally imports and sends DONE before strict rebuild checks scale/padding. | Strict staged-overlay verification before publication/ACK; regression required. |
| OBJ-03 | Existing-parent shortcut misses evicted descendants; save lacks directory-sync/concurrent-writer guarantee. | Closure repair plus durable writer ownership and recovery. |
| WARM-01 | Warm seed root excludes original length: trailing-zero variants collide as representations. Patch application assumes seed length and contains unchecked slices. | Repair before sparse encoder reuse; do not use as identity/commit foundation. |
| WARM-02 | `WarmSession::Committed` denotes verification/counters, not stored reconstructed output. Incoming header is reused as local clock reference. | Do not claim durable commit or distributed synchronization. |
| WIRE-01 | UORADP01 raw/zlib TCP forwarding/mux exists; cache layer absent. Length/inflate bounds and stream lifecycle need hardening. C interposer is bypassable. | Cooperative stream milestone, behind MVM policy gate. |
| WEB-01 | DUALFACE:3→ICOSIAN:90, delta=0: K=1 falsely says unique/DISTINCTIVE; K=100 finds 63 equal 5-hop routes. | Add cap-independent tie probe and fixtures. |
| WEB-02 | 507-node/2248-edge graph uses unweighted BFS. IP-shaped labels and documentation IPv6 labels are local addressing demonstrations. | No measured Internet topology, NAT, addressing allocation or WAN routing claim. |
| WEB-03 | Bell pairs, H2 carriers, E8 operators and subnet labels have different semantics. Equal cardinality is not an isomorphism. | Preserve typed basis/frame/orientation metadata. |
| WEB-04 | Local Kuramoto RK4 and zero-frequency convergence; no peer time exchange. Shared setup cache holds clock/alphabet, not source data. | Scheduling hypothesis only; logical epochs/sequence remain authoritative. |
| WEB-05 | Whole-file score encoding expanded 1,021,647 source bytes to 1,029,630 packed bytes (1,424,384 JSON). Software Fourier timings are local channel work. | No compression/WAN claim; native whole-file confirmation pending in source. |
| PY-01 | SystemLawCodec retains 32-byte chunks, RLE, length and digest. RuntimeGeometricRouter uses counters/IP-shaped strings, no AF_VSOCK traffic. Signal Courier is local messaging. | Useful codec/simulation reference; not deployed network. |
| PY-02 | Older symbolic discovery returns fixed equation/r2=1; newer fit creates target from the same basis columns, labels PySR without invoking it. | Authored formula recovery, not independent discovery. |
| PY-03 | Minimal state is 15-byte `>IBBBfI` plus external defaults; 8-byte capsule omits complete phases/tree/history. | Summary/checkpoint scope must be explicit. |
| MATH-01 | Ternary frontier occurrences=s3(N), carries=(N-s3(N))/2. Frontier logarithmic, not universally <=16; unique retained data can grow linearly. | Explicit capacity and retained-state accounting. |
| MATH-02 | Exact signed-axis actions preserve E8 roots; anchor orbit16 and projections are not arbitrary content identities. Base63 encodes256 bits in43 digits/33 bytes. | Exact algebra/encoding does not alone prove compression. |
| MATH-03 | Exported radix16 `u64_to_atoms` shifts16 by60 before its guard; u64 wrap can reconstruct2^63 incorrectly. | Source-derived defect; independently simulated, original Rust not run. Quarantine from wire path. |
| MATH-04 | S1/S2 averages are lossy. Exact action metadata cannot restore discarded arbitrary bytes. Secant positive and later eight-comparison negative use different conditions. | Preserve both research results; neither is a network measurement. |
| MVM-01 | ADR042 FlowMux/vsock5253 is policy-controlled network path. ADR057 mailbox is authenticated, encrypted, at-least-once. | Preserve services/receipt semantics; no invented mailbox port. |
| MVM-02 | #4629 dedicated mailbox service and #4633 Iroh adapter remain open at execution check. AuthorizedMailboxFrame hides bytes; PeerChannel is send-only. | Production byte adapter needs canonical upstream seam. |
| SEC-01 | Legacy BLAKE2b96 IDs and locally implemented hashing are not validated crypto; digest possession grants no authority. | New versioned SHA256 profile; qualification remains separate. |
| HEAT-01 | No HeatNet/vnet implementation or identifying reference in scripts, PDFs/OCR, ZIP text or decoded cards. | HEAT.NET provisional historical reference; no invented dependency. |

## Historical performance evidence

The retained independent 128 KiB object experiment reports cold230705 bytes; identical warm367; one-byte overwrite1083 with8 repair rounds; prefix insertion230768. Exact reconstruction passed. This supports conditional sharing and exposes cold expansion, alignment sensitivity and WAN round-trip risk. Source: `archive/labs/spiralcore-attention-assessment-2026-10-14/evidence/independent-results.json`.

The native review records813 range cases/2374 assertions and distinguishes partial-range integrity from full materialization. Source: `archive/labs/spiralcore-native-attention-review-2026-10-14/README.md`.

UOR object-store historical records report unique-content storage expansion about2.17x. A cold2MiB artifact needed5,725,408 DAG wire bytes versus39,920 compressed bytes. A52,535,088-byte random WAN object took236.19s/143,659,164 wire bytes at default batching and91.50s/143,651,786 at larger batching: latency improved about2.58x while volume barely changed. These are historical UOR receipts, not newly qualified secure transfer results.

The warm-cache remote receipt used TCP over SSH with UDP blocked; small frames were134/4238 bytes and ACK timings roughly209–217/206–266ms. Frame sizes omit several outer costs and seeding/encoding. The reported120ms emulated RTT was a server sleep, not bidirectional network shaping. The wire WAN summary37.93→32.08s lacks the underlying raw artifact in this export. These are hypotheses for reproduction, not acceptance evidence.

## Research experiments retained for synthesis

Geometry/cache placement against shortest-path/random/rendezvous; ternary sharing against content-defined chunks; Kuramoto pacing under drift/partition; CA seed+checkpoint+event residual replay; history-aware versus memoryless prediction; counterfactual policy replay; physical Bell channel BER/FEC/goodput. All require complete setup/residual/control accounting. No executable received cards or global physical-clock equality requirement enters the initial protocol.

## Security and provenance

The object-store core is explicitly MIT. Owner-supplied ZIPs lack explicit license notices; do not infer blanket redistribution permission for arbitrary copied embedded implementations. Unclassified/CUI is the target requirements context, not a claim of approved deployment. Inventory cryptographic envelope, transport, storage, identity, signing, RNG and keys against concrete validated module/OE/mode before qualification. Public/synthetic fixtures only for current demonstrations.
