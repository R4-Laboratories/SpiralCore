# SC-OPT-010: Preserve byte choices while reducing sender computation

Accepted for implementation under the user's existing instruction to optimize sender selection, residual construction and hashing. Baseline `11fb80931a97b06f3c1b40621b7e8c40710274e5` was preserved before edits. No new approval pause is required.

The runtime sender will evaluate its explicit replay/residual candidate before other codecs. It may omit a conventional candidate only when a proved lower bound cannot improve the incumbent under the original priority: Raw, chunked Zstd, Sparse, Prefix, Runtime. Thus an earlier-priority codec must still be evaluated when its lower bound ties a later-priority incumbent. Selected mode, exact encoded frame bodies and successful-miss body bytes must match exhaustive selection.

## Bounds and receipts

Let `N` be target length, `B` checkpoint/base length, `C=MAX_CHUNK`, and `W=40` when warm probing, otherwise zero. Raw retains its exact cost. Chunked Zstd costs at least `368+W+134*ceil(N/C)`: each nonempty chunk contributes a five-byte codec header, at least one payload byte, and 128 bytes of request/reply framing. Empty input has no Data frames.

For nonempty targets, Sparse costs at least `400+W+137+ceil(13*N/max(B,13))`. At least one frame contributes 128 bytes plus its nine-byte recipe header. Each COPY costs 13 bytes and emits at most `B` bytes. Each LITERAL costs `5+n >= n` bytes. Therefore each operation's encoded bytes are at least `13*decoded/max(B,13)`; summing and rounding up gives the bound. Extra frames and operation headers only increase real cost. Empty-target sparse behavior remains `None`.

`Selection.candidates` contains exact costs only for evaluated candidates, in original tie order. A separate `Selection.pruned` list records the skipped mode and its proved HTTP-body lower bound. A bound is never presented as an encoded size. The ordinary adaptive API retains exhaustive behavior, limiting this optimization to runtime senders. The exact-repeat warm-probe shortcut is unchanged.

## Prefix structural bound

The prefix frame contains four magic bytes, a frame header of at least two bytes and the profile's mandatory four-byte checksum. Each block has a three-byte header and can emit at most 128 KiB of decompressed data. This gives `encoded >= 10+3*max(1,ceil(N/131072))`, ignoring all block payload bytes. These requirements are specified in [RFC 8878 sections 3.1.1 and 3.1.1.2](https://www.rfc-editor.org/rfc/rfc8878.html#section-3.1.1). Pinned zstd 1.5.7 `lib/zstd.h` lines 147–148 defines `ZSTD_BLOCKSIZE_MAX = 1<<17`; `lib/compress/zstd_compress.c` lines 1714 and 1816 bounds block size by that limit and the window.

At least one prefix transport frame adds six codec bytes and 128 framing bytes. Therefore its complete body cost is at least `544+W+3*max(1,ceil(N/131072))`. At 14,851,384 target bytes this is 886 bytes before warm-probe overhead, exceeding the matching runtime candidate's 747 bytes. The same original tie rule applies to this bound. It is a structural minimum, not the measured encoded cost.

When eligible, prefix compression remains uncapped with identical settings. A `compress2` destination-capacity failure does not establish that the usual encoded output exceeds that capacity: pinned zstd 1.5.7 `lib/compress/zstd_compress.c` requires the maximum frame-header space at line 4712 before writing a potentially shorter actual header, and lines 3023–3027 include capacity-dependent raw-block fallback. Safely recognizing the destination-too-small error would not prove the required inequality or byte parity. Capacity-based and streaming pruning are excluded.

## Other bounded changes

The residual encoder replaces scalar prefix/suffix/equality-run scans with bulk comparisons while preserving operation boundaries and emitted bytes. This is parent-owned and checked against the baseline encoder. Prepared predictions retain immutable checkpoint and prediction IDs. The sender reuses the prediction ID only after full target-byte equality; edited targets use normal whole-object hashing. Identity access runs within the measured sender interval and after size checks. Checkpoint identity is used only for a selected base-dependent mode. Cached IDs add 64 bytes of fixed metadata; retained-buffer accounting remains explicitly buffer-only.

No wire format, receiver decoding, authorization, commit, math, or resource-limit change is introduced. Evaluated-encoder errors propagate. Qualification compares exhaustive versus optimized winners and complete bodies, exercises ties/boundaries and changed-target identities, and records local CPU/wall costs with the same durability. Existing WAN results do not become results for this revision. Byte parity and zero loss are requirements; compute improvements remain measured outcomes.
