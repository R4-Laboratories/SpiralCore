# Follow-up hypothesis: a format bound for prefix-candidate rejection

Status: **NOT_IMPLEMENTED / NOT_MEASURED / NOT_YET_INDEPENDENTLY_REVIEWED**.
This is outside SC-OPT-007's sparse-packing change and its acceptance experiment.

The [Zstandard format shipped with v1.5.7](https://github.com/facebook/zstd/blob/v1.5.7/doc/zstd_compression_format.md#blocks)
limits each block's regenerated content to at most 128 KiB and gives each block
a three-byte header. Therefore a frame representing `N > 0` bytes requires at
least `ceil(N / 131072)` block headers. Ignoring all block payload gives a weak
but potentially useful lower bound. The current SpiralCore prefix profile also
requires the frame checksum and uses one standard frame.

An initial derived bound is:

```text
encoded_prefix_bytes >= 4-byte magic + 2-byte minimum frame header
                        + 4-byte required checksum + 3*ceil(N/131072)
successful body bytes >= 400 + 134 + encoded_prefix_bytes
```

The second line uses existing `selection.rs` accounting: 400 common/base-bound
bytes and at least one Data exchange with its six-byte prefix body header.
Additional Data chunks only increase the cost. At 16 MiB the derived body bound
is 928 bytes, above the existing 600-byte localized sparse transfer. This suggests
prefix compression might be safely omitted in that case. At 1 MiB the bound is
568 bytes, which does **not** justify skipping a candidate against 600 bytes.

Before implementation, independently prove the bound against the pinned library,
frame profile and boundary cases; preserve the 128 KiB prefix-beats-sparse
counterexample. Design truthful selector diagnostics: an uncomputed candidate has
a lower bound, not an actual encoded cost. Do not silently change the meaning of
the public candidate-cost list. Any revised selector needs its own frozen oracle,
qualification and CPU/elapsed measurements. No speedup follows from this note.
