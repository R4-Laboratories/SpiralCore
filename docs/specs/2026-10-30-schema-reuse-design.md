# SC-OPT-012: Compressed schema streams and reusable codec state

Accepted under the user's instruction to compress schema values/residuals, reuse prepared codecs and measure persistent receiver sequences. Base: `e0a29bc18a09e1aeff4f3a36d2d801ffca346255`. APIs agreed with the parent on 2026-10-30; no new user approval is required. Existing `SCSCH001`, `SCVAL001` and `SCZDICT1` bytes remain valid. The coupled runtime remains an active family.

## APIs and ownership

`schema::PreparedSchema::new(artifact)` validates and owns the artifact, exact literal fragments and layout index once. It exposes `artifact()`, `artifact_id()`, `retained_bytes()`, `retained_charge()`, `encode(&self,target)` and `decode(&self,encoded,target_len)`. Owned `Cow` fragments avoid unsafe self-references. The existing free schema functions remain available and use the same byte-producing/validating helpers.

Both `schema_zstd` and `zstd_dictionary` expose separate `PreparedEncoder` and `PreparedDecoder` types. Constructors take artifact bytes; each exposes the same artifact/identity/accounting accessors and mutable `encode(target)` or `decode(encoded,target_len)`. Encoder and decoder contexts are separate so a receiver cache does not retain compression workspaces. Dictionaries are copied into `CCtx`/`DCtx` once with `load_dictionary`; no parameter-changing `CDict` shortcut is introduced. The fixed compression settings remain level 3, window log 25, long-distance matching enabled, zero workers, content size/checksum enabled and dictionary-ID field disabled. Repeated/alternating calls, including calls after rejected input, must match fresh dictionary encoding bytes and decode exactly.

## New bounded representation

Parent-owned `Mode::SchemaZstdV1=8` carries exactly one checksummed zstd frame whose decompressed bytes are the existing `SCVAL001` stream. The schema artifact ID remains bound by the offer and the inner stream. No lossy parsing, changed numeric spelling or new default inference is introduced.

`schema::encoded_cap(N) = N + 5*min(N,65536) + 48`, for `N <=16 MiB`. Nonempty records each consume at least one original byte; newline-heavy fallback is one literal record. This bounds the complete intermediate even for binary/literal fallback. `schema_zstd::encoded_cap(N)` is zstd's compression bound of that intermediate cap.

Before allocation/decompression, the new validator requires a standard single zstd frame, known content size between 48 and the target-specific intermediate cap, required checksum, no dictionary-ID field or reserved/unused descriptor bits, window at most 32 MiB, and exact compressed-frame extent with no appended frame/trailing data. One-shot decompression receives the exact output slice. The inner schema decoder then checks its artifact ID, original length, record/token structure and output preflight. Old prefix/dictionary limits are unchanged.

## Retention, admission and errors

Prepared schema quota charge is 512 KiB, covering at most 64 KiB artifact bytes, another at most 64 KiB owned literal bytes and bounded fragment/index/container metadata. Decoder context allowance remains 4 MiB; schema-zstd decoder charge is 4.5 MiB and dictionary decoder charge is 4 MiB plus 64 KiB. Sender compression contexts use a conservative 128 MiB allowance plus their schema/artifact charge. Zstd `sizeof` checks are drift guards, not a custom-allocator or RSS enforcement mechanism. `retained_bytes()` reports owned payload bytes plus zstd-reported context size where applicable, excluding Rust/allocator bookkeeping; `retained_charge()` is the conservative quota figure. Construction and temporary input/intermediate/output buffers are separate costs and reservations. No per-call temporary output is retained implicitly.

The parent owns a default-off receiver cache keyed by namespace, mode and full artifact ID, bounded to eight entries and charged 5 MiB per entry within the shared resident limit. Artifact bytes are read and verified on disk, and authorization is rechecked at Begin before cache lookup. The extra cache charge is admitted before construction; pressure may use unprepared decoding with explicit miss/prepare counters. Existing attempt decoder allowance remains conservatively charged, and mode 8 reserves its intermediate separately. Active attempts pin `Arc<Mutex<PreparedDecoder>>` entries; only unpinned entries may be evicted. Clearing/restart drops retained state, and toggling while active is rejected. Revocation, failed decode and epoch changes preserve existing publication/authorization rules. Codec ownership is not authority. A failed decode cannot publish bytes; subsequent calls must either recover correctly or fail closed without retaining a context beyond its charge.

## Qualification

Use RED/GREEN tests for prepared schema byte parity and immutable ownership, repeated fresh-versus-reused dictionary bytes, post-error recovery, compressed schema exact roundtrips including lexical/drift/binary fallback, wrong artifacts, malformed frames and hostile intermediate sizes. Parent tests persistent receiver sequences, scoped cache admission/accounting and unchanged durable commit. Measure setup separately, reuse count and amortization, both-direction body bytes, encode/decode CPU and durable elapsed time, with frozen training and controls. Local results establish no new WAN or energy claim.
