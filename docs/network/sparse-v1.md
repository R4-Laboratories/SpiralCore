# Sparse COPY/LITERAL v1 contract

Reviewed by the architecture/security investigator before implementation on 2026-10-29.
This extends SCMOBJ01 with mode 3; raw/zstd/DAG encodings and object identities are unchanged.
Old receivers reject mode 3 as Unsupported. No silent descriptor change is permitted.

Offer: mode=3, existing exact target length/ObjectId, no manifest, required base=full ObjectId
(not the ManifestId used by mode 2). Authentication and the complete Context namespace remain mandatory.
Begin reads and verifies the base, pins it, reserves the complete target and base/scratch/ledger memory,
and preallocates target capacity. Unknown bases return Missing. Fallback requires a fresh attempt ID.

Data body (all integers little-endian):

```
encoding:u8=3 | expanded_bytes:u32 | version:u16=1 | op_count:u16
COPY    = tag:u8=0 | base_offset:u64 | length:u32
LITERAL = tag:u8=1 | length:u32 | bytes[length]
```

Operations append in order. COPY can refer only to the verified pinned base, never generated output.
The frame including its 56-byte envelope is at most 65,536 bytes; at most 256 operations per frame,
8,192 operations per attempt, and 61,440 aggregate literal bytes per frame. Every operation has positive
length. Empty targets have no Data frames. Unknown versions/tags, trailing bytes, zero-op frames,
invalid declared expansion, overflowing/out-of-range ranges and over-budget output/operations fail closed.
COPY expansion may consume the remaining declared output, bounded by the 64 MiB object ceiling.

Validate the entire frame before appending anything, then execute a second parsing pass into the
reserved target. Accepted reports cumulative reconstructed bytes. Only newly accepted sequence numbers
count toward work; identical replay returns the recorded response. Final publication requires exact
length and the existing full-byte SHA-256 identity, current authority and durable file/directory sync.
Mode 3 publishes without optional DAG warming; its next sparse base is available by ObjectId.
Existing cancellation, expiry, four-active-transfer, ledger, disk and shared resident limits apply.

The first encoder compares bounded aligned runs and common prefix/suffix recipes. It coalesces runs,
splits literals to frame bounds and selects raw fallback when the estimated Data request/reply bytes
plus the extra base ID are not smaller. This selection estimate excludes batch headers, HTTP/TLS and
fallback exchanges; actual counters determine transport savings. It is not content-defined chunking. Setup, comparisons, full hashing and fallback work
belong inside timing. Scope checks, exact reconstruction and limits remain mandatory even if an
optimization is slower. No cryptographic algorithm or FIPS qualification changes in this extension.
