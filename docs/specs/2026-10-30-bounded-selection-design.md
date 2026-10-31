# SC-OPT-016: bounded Whole-zstd rejection

Approved by the user's instruction to optimize the remaining warm-selector CPU
while preserving byte minimum and exact fidelity. Base: `4234591e25b4ca87db6a4670bdf407f48a581575`.
Implementation follows the parent commit of this design; no new user approval is
required. This is an implementation hypothesis, not a measured speed result.

## Decision and alternatives

Use an actual **first-block lower bound** to exclude Whole after the existing
static bound, then use the unchanged full `compress2` whenever inconclusive.
Do not use the remainder of a streaming frame as the candidate. Full streaming
parity is not established: later block splitting and input boundaries may differ.
Destination-too-small errors remain inadmissible evidence. Lazy context creation
is simpler but does not remove the measured full-Whole work. Immutable prepared
base identity is deferred: it introduces ownership/copy/setup costs and leaves
that compression work in place.

The public `prepare_warm` contract, old `prepare`, receiver, formats, authorization,
16 MiB bounds and Raw/Whole/Sparse/Schema tie priority remain unchanged. Only the
eligible Whole branch may probe; Schema and all static pruning remain unchanged.
Expose `zstd_whole::first_block_probe_available() -> bool` for qualification.
The codec helper is crate-private and returns an optional encoded-length lower
bound, never an alternative encoding. Exact candidates and pruned bounds retain
their existing separate receipt fields.

## Certificate and fallback

For `128 KiB < N <= 16 MiB`, present the complete unchanged target to the existing
CCtx using safe `compress_stream2(..., End)` and a12-byte external output slice.
Reset the session before starting and after the probe. The approved profile has
a9-byte frame header and a3-byte first-block header. Validate magic, expected
single-segment/checksum/content-size flags, exact full content length,12 output
bytes,128 KiB consumed input, and a legal nonfinal first block. The block payload
length is its declared size for Raw/Compressed, or1 for RLE. Reject an unexpected
or zero-sized shape as inconclusive. A sound full-frame encoded-size bound is:

`12 + first_payload_bytes + 4 + 3 * ceil((N - 131072) / 131072)`.

The final terms are the required checksum and at least the remaining block
headers; no future payload is assumed. Convert this with monotone existing
encoded chunk/framing costs and optional warm-probe overhead. Prune only when
`(body_bound, Whole_priority)` cannot beat the incumbent. A tie with Sparse still
requires full encoding. Never send the probe output or report its bound as an
exact encoded cost. Inconclusive bounds, unavailable guards or unexpected shapes
use the original full `compress2`; true encoder, reset and resource errors fail
closed. Preserve the existing workspace allowance and drop a context exceeding
it. No probe runs at/below128 KiB or when the static bound already excludes Whole
(as on the registered16 MiB sparse fixture).

## Pinned implementation argument

The inspected dependency is `zstd-safe7.3.0` / `zstd-sys2.1.2+zstd.1.5.7`, whose
Cargo package checksum is
`481ada9c543b2323cbb7bbdbb5249177fbf0188ed959ed2c15e4b22421c84ed3`.
Line references below are to its bundled `zstd/lib/compress` source:

- `zstd_compress.c:6574` makes one-shot input/output stable; `:6147` selects the
  direct full-input path, while `:6195` selects the internally buffered block.
  Both start from full-source End parameters (`:6366`) and the same header (`:4697`).
- `:4552` and `:4601` make the first block unsplit with initial savings zero.
  `:2134` allocates internal output `compressBound(block)+1`, so the12-byte caller
  buffer limits copying rather than entropy compression. Capacity-related Raw
  fallback must still agree with the unchanged first-block oracle in tests.
- LDM receives the same first block at `:3340`; `zstd_ldm.c:361` bounds matching
  to that block, and `:542` uses `nextSrc` only as an assertion. There is no
  future-block matching input in this first-block comparison.
- All level-3 table entries are dfast (`clevels.h:31,57,83,109`); row-matcher salt
  is not used. `zstd_double_fast.c:121` bounds match/read-ahead to the block end.

Proof-subset SHA256: `zstd_compress.c`=`10c315ae609d49b2fc431521aa95908086e69fd917991c5ed9a875af3d1208e8`,
`zstd_ldm.c`=`4a2bef3612ae4a9e15a78a55541a152091fa8ba160410db24d495bd6dd57249e`,
`zstd_double_fast.c`=`9f6d003c2fe1208737bec685910324220424ba47d0323fa30d9d5f53c50e91da`,
`clevels.h`=`d764daa89b7a636d26fe96ff3536c5075e9867fb5dddbf8c9d57023a8ff4a155`.

This vendored tree differs from the upstream v1.5.7 tag, so numeric version10507
alone is insufficient. Parent-owned `build.rs`/`build_support` hash the complete
bundled `zstd/lib` C/header tree and zstd-sys build script, from the exact direct
zstd-sys dependency's `DEP_ZSTD_INCLUDE`, before enabling
`cfg(spiralcore_bundled_zstd_probe)`. Missing/foreign metadata, mismatched source,
pkg-config/system linkage or compiler-flag overrides disable the probe. A
registered explicit baseline-disable setting also disables it. The full encoder
remains available. The registered90-file tree digest is
`6434f9d74ec3cfb7fbdf62420716c8912451c046805f932e4b4ba7d02b2834de`: SHA256
starts with `spiralcore.zstd.bundled-source.v1\0`, then sorted package-relative
UTF8 path lengthLE-u64/path/content lengthLE-u64/content. No native source,
experimental zstd feature or protocol toggle is added; the current codec
parameter profile remains a prerequisite.

## Acceptance

Require first-block header/size parity with the unchanged uncapped encoder,
each bound no greater than its full encoded size, and exhaustive selector mode,
frame and byte-cost parity. Include random, repetitive, structured and mixed
first/tail content, block boundaries, all four winners, ties, rejected lengths,
repeated aborted probes and subsequent normal encoding. Compressible/inconclusive
inputs may pay duplicate first-block work: measure that negative control.

Use common current measurement code with a labelled baseline production overlay,
not a claim that the old full revision contains the new harness. Matched arms are
baseline selector, candidate selector and equally tuned Whole; each pays its
actual fitting/context setup and identical Raw base seed. Local10 pairs per size
means120 transfers in two five-pair shards; advance only if the paired small-target
sender-CPU median ratio candidate/baseline is <=0.80 with exact bytes throughout.
Then run30 pairs per size/360 WAN transfers in six five-pair shards. Each shard
retains510 MiB payload and the existing566.234375 MiB conservative charge, below
768 MiB. Stop/reopen/verify and clean each owned store after acceptance. Preserve
the earlier36-transfer qualification and all new negative outcomes. Report body
bytes, sender/receiver-handler CPU, durable wall and paid lifecycle separately;
exchange time is a subset, not pure send time, and energy is not measured.
