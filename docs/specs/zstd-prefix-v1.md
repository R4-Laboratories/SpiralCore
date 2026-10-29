# Bounded zstd prefix transfer and cold graph policy

This application profile adds mode 4 to the existing object protocol without changing modes 0–3, exact object identities, canonical Offer binding, private cache ownership, authorization or durable publication. It is a configured zstd prefix control for the 16 MiB batched transfer experiment. It is not VCDIFF, rsync, or a claim of exact CLI `zstd --patch-from` parameter equivalence. The underlying technique is described in the [zstd project's patching overview](https://github.com/meta/zstd/wiki/Zstandard-as-a-patching-engine).

## Local receiver policy

`Endpoint::open(root, limits, authority)` continues to use `ColdGraphPolicy::EagerBounded`. `Endpoint::open_with_policy(root, limits, authority, ColdGraphPolicy::Disabled)` disables only optional graph construction after Raw/Zstd publication, and removes its optional graph reservation from admission. The policy is immutable for the instance and cannot be selected by network input. No Limits defaults or struct fields change.

Disabled publications retain exact verified bytes and ObjectId but have no manifest ID. They can serve as whole-object bases for sparse/prefix transfers; they are not advertised as DAG-ready bases. Explicit DAG transfers still require complete verified closure and retain their manifest. Existing publications are not rewritten on reopen. Sparse/prefix publication remains graph-free regardless of this cold policy.

The minimal receiver benchmark uses Disabled consistently for raw, ordinary zstd, sparse and prefix arms. Earlier default-policy results remain separately labeled. Plan/row metadata asserts the expected receiver policy; the retained receiver plan, policy log and executable/source identity establish the actual configuration.

## Offer and Data

`Mode::ZstdPrefixV1` has tag `4`. Offer contains the target byte ObjectId and length, no Manifest, and a required base **whole-object ObjectId**. Both target and authorized base are at most 16 MiB. DescriptorId continues to bind the entire canonical Offer. An old peer rejects this mode as unsupported.

Data body fields, in order:

| Field | Encoding | Required value/bound |
|---|---|---|
| Encoding | u8 | 4 |
| Encoded chunk length | u32 little endian | 1 through 61,440 |
| Profile version | u8 | 1 |
| Encoded bytes | exact declared length | next contiguous portion of one frame |

There are at most 512 Data chunks and at most `zstd::zstd_safe::compress_bound(target_length)` encoded bytes. The maximum cap is 16 MiB + 65,536 bytes. The existing 65,536-byte complete-frame cap and ordered sequence/replay rules also apply. The receiver validates each entire Data body before modifying encoded state. Accepted acknowledges cumulative **encoded bytes**, excluding version bytes. It does not assert decoded, verified, or durable object bytes. Existing sender `Stats.data_bytes` counts Data body bytes after the first five header bytes, so includes the per-chunk profile version; HTTP/body metrics remain separately measured.

The receiver pins a fully verified base in the same authorized namespace before Ready. Missing or corrupt base, wrong scope, revoked authority and resource failure do not silently select another mode. A new cold attempt requires an explicit caller decision and fresh transfer identity.

## Finish and exact recovery

Finish accepts exactly one ordinary zstd frame. Before allocating decoder/output state, validate:

1. Encoded length is within the admitted cap; ordinary zstd magic is present.
2. Frame descriptor has no reserved/unused bits, no dictionary-ID field and a checksum flag.
3. An explicit window descriptor, when present, advertises at most 32 MiB. A single-segment frame's window is its content size, bounded below.
4. Content size is present and equals the declared target length, at most 16 MiB.
5. The compressed frame occupies exactly the encoded stream: no trailing bytes, concatenated or skippable frames.

Decode once using the safe `DCtx::ref_prefix` and `DCtx::decompress` APIs with an exact target-length output slice. Verify zstd's checksum and exact decoded length, then the existing full domain-separated SHA256 ObjectId over target length and bytes. The frame checksum is not the authorization or cryptographic identity check. Reauthorize and use the existing verified durable publish path before Published. No partial output is published, and no executable recipe is accepted. Cancel/expiry/rejection release encoded/base/output state under the existing state/ledger rules; partial state is volatile across restart, while complete publication remains recoverable.

## Receiver resource bound

For a target absent from durable storage, admission reserves:

```
2 * target_length + 2 * base_length + encoded_cap
    + 4 MiB fixed decoder allowance + 2 MiB existing ledger/scratch allowance
```

The maximum is 86.0625 MiB per attempt. It is charged inside the existing resident ceiling, including other attempts, retained graphs and terminal ledgers. Disk quota reservation and Begin/Finish filesystem observations remain unchanged. The active-transfer count is an upper limit; a smaller configured resident budget may admit fewer transfers.

The encoded buffer is reserved once to its checked cap before accepting Data. Base reads are reserved and verified before use. Output is reserved to the checked target length, initialized, and passed as an exact-length slice rather than allocator capacity. Chunk count and encoded/output caps bound work; synchronous decode and durable writes remain cooperative operations, not hard CPU preemption or process-RSS isolation.

The decoder allowance is based on pinned zstd 1.5.7 source, reached through `zstd 0.13.3`, `zstd-safe 7.3.0` and `zstd-sys 2.1.2+zstd.1.5.7` in Cargo.lock:

- [`ZSTD_decompressDCtx`](https://github.com/facebook/zstd/blob/v1.5.7/lib/decompress/zstd_decompress.c) calls the one-shot dictionary decoder. It does not allocate the streaming input/output buffers according to advertised window size.
- In that same source, `ZSTD_DCtx_refPrefix_advanced` uses a by-reference raw-content dictionary and marks it single-use. The prefix remains borrowed and immutable for the decoder lifetime.
- [`ZSTD_createDDict_advanced`](https://github.com/facebook/zstd/blob/v1.5.7/lib/decompress/zstd_ddict.c) allocates a fixed dictionary structure; raw by-reference prefix loading neither copies the prefix nor parses it as a formatted dictionary.
- The context's fixed entropy/workspace tables and bounded literal buffer are defined in [`zstd_decompress_internal.h`](https://github.com/facebook/zstd/blob/v1.5.7/lib/decompress/zstd_decompress_internal.h). Literal workspace uses caller output and fixed internal storage, not an advertised-window allocation.

The 4 MiB allowance conservatively covers those fixed structures. Runtime `sizeof` checks before/after decode are drift guards, not the sole allocation defense. Dependency/backend changes require rechecking this source argument and qualification. No new unsafe application code or crypto primitive is introduced.

## Sender and fallback

`transfer_zstd_prefix_batched(base, target, transfer_id, warm_probe, exchange)` uses one whole-base frame: compression level 3, windowLog 25, long-distance matching enabled, `NbWorkers=0`, content-size flag, checksum, and no dictionary-ID field. The base is not rebuilt for every Data chunk. Target/base remain at most 16 MiB, and existing batch limits are unchanged.

Exact repeat inputs use a Raw identity offer. Otherwise the sender encodes once, compares full-transfer object-protocol traffic with Raw and selects Raw if prefix is not cheaper. The comparison includes the additional 32-byte base ID, per-Data frame/version and Accepted costs, and batch length prefixes. Common Begin/Finish and batch headers cancel. All codec and fallback work is included in sender elapsed/CPU boundaries; `Stats.mode` reports what was actually sent. There is no automatic fallback after Begin or a missing-base error. These are predicted complete-transfer costs; a peer's preexisting target shortcut is separately governed by `warm_probe`.

The sender configuration and bounded local inputs constrain work. A proposed 128 MiB encoder-context figure is only a qualification estimate, not an enforced preallocation, process-memory or RSS limit. The fixed-table receiver argument does not apply to encoder match-finding tables. Do not infer energy savings from CPU or primitive-codec timings.

## Qualification and later scope

Focused checks cover default/disabled graph readiness, explicit DAG behavior, exact reconstruction/reopen, insertion, empty input, wrong base/scope, revocation, malformed frame/version/chunk bounds, wrong digest, cancellation/shared reservations, replay and receipt integrity. Full benchmark fixtures, selected modes, one-time seeds, source/executable identities and capture completeness must be frozen and verified before interpreting WAN performance.

Insertion targets stay within 16 MiB by reducing the base first. The compressed-localized fixture mutates bytes inside a valid compressed base; the mutated target is not claimed to remain a valid compressed stream. A 64 MiB path requires separate batch/resource/recovery qualification and is not enabled by this profile.

Later MVM integration remains a canonical mailbox/vsock bridge for both individual VMs and a small orchestrator. Stable authenticated node/tenant/capability/epoch identities must map to current guest routes; a vsock CID alone is not identity. Define bounded backpressure, restart/rebind/revocation, persistent idempotency and at-least-once delivery. Distinguish durable object Published, mailbox delivery and application processing acknowledgments. Preserve the canonical encrypted envelope boundary and do not repurpose FlowMux 5253. No MVM transport, port allocation, FIPS validation or CUI release is claimed here.
