# SpiralCore object mailbox application profile v1

Status: reviewed by architect/planner and approved for bounded experimental implementation on2026-10-28. User subsequently prioritized standalone SpiralCore operation; MVM harness/runtime integration remains deferred. This application payload rides inside the existing MVM authenticated encrypted Call/Reply envelope, content type `application/spiralcore-object`, version1. It is not a replacement mailbox envelope, destination receipt, transport, port or authority. Host relays see canonical ciphertext only. Current execution uses a standalone loopback TCP harness and a trusted-context interface. A future canonical-envelope harness and live guest mailbox/Iroh adapters are deferred; live adapters require upstream #4629/#4633.

## Identity and data structures

The standalone implementation also supports [Sparse COPY/LITERAL v1](../network/sparse-v1.md),
an explicit mode-3 extension. The original modes 0–2, identities and binary vectors below are
unchanged. Mode 3 uses a full ObjectId as its base and has its own bounded Data encoding; an older
receiver rejects the new mode as Unsupported. The current HTTPS experiment is documented separately
in [WAN instructions](../network/wan.md); it does not implement the future MVM envelope.

All integers little-endian. Hash is SHA-256; implementation is experimental, not a validated cryptographic module. Fixed domains include their terminal NUL.

- ObjectId32 = H(`spiralcore.object.bytes.v1\0` || byte_len:u64 || original bytes).
- NodeId32 = H(`spiralcore.object.node.v1\0` || canonical node body).
- ManifestId32 = H(`spiralcore.object.manifest.v1\0` || canonical manifest body).
- DescriptorId32 = H(`spiralcore.object.transfer.v1\0` || canonical Begin body), binding this attempt's immutable encoding/target/base descriptor.
- Leaf body: type0:u8 || scale0:u8 || data32. Parent body: type1:u8 || scale:u8 || ordered three child NodeId32 values. A record is NodeId32 || body (66/130bytes). Parent scale>=1; every child has scale exactly one less.
- Manifest body: profile1:u16 || byte_len:u64 || ObjectId32 || root_count:u8 || ordered(scale:u8, NodeId32)*. Root scales exactly equal descending nonzero base-three digits of ceil(byte_len/32), with repeated roots retained. Empty has zero roots. Final partial leaf zero padding is mandatory. No extra bytes or ambiguous encodings.

Object identity does not change with representation. UOROBJ01's 96-bit records are not v1 cache acknowledgments. The extracted core supplies ordered fold construction; the adapter translates its immutable structure to typed SHA-256 records and independently verifies exact ObjectId, shape and padding. If a legacy construction cannot reconstruct the input exactly, encoding fails.

## Framing and operations

Application frame: magic`SCMOBJ01`8 || kind:u8 || flags0:u8 || reserved0:u16 || transfer_id32 || request_seq:u64 || body_len:u32 || body. Total payload<=65536bytes, header56bytes. Unknown kind, nonzero flags/reserved, overflow, truncation and trailing bytes are errors. Transfer IDs identify application work, never cryptographic nonces.

| Kind | Body | Result |
|---|---|---|
| Begin1 | mode:u8(raw0,zstd1,dag2), byte_len:u64, ObjectId32, manifest_len:u32, optional target_manifest, base_present:u8, optional base ManifestId32 | Ready101: selected_mode:u8(equal Begin.mode), base_pinned:u8, next_seq:u64 |
| Data2 | encoding:u8(raw0,zstd1,records2), decoded_len:u32, bytes | Accepted102: accepted_input_bytes:u64 |
| Finish3 | empty | Published103:ObjectId32 or Need104:count:u16, up to1024NodeId32 values |
| Resume4 | DescriptorId32 || canonical Begin body | Ready101 or Published103; incomplete volatile staging after restart returns Unknown |
| Status5 | empty | State105:state:u8(receiving0,published1,cancelled2,rejected3), received_bytes:u64 |
| Cancel6 | empty | State105; published content is not rolled back |
| Error127 | code:u16 | Typed refusal, no sensitive diagnostics |

Cold Begin omits the DAG manifest/base; its ObjectId/length is independently authoritative. DAG Begin requires a matching target manifest. This allows cold64MiB transfer without requiring DAG materialization.

Manifest length<=1024bytes and roots<=28; absence flag0, presence1 only. Cold manifest length0 and base flag0 are mandatory; DAG manifest length>0 mandatory. Reject parent scale>13.

Data raw/zstd chunks decode to<=60KiB. Zstd chunks are independent, no dictionary; max zstd window1MiB; reject concatenated frames, trailing bytes and excessive expansion. Use raw per-chunk fallback when compression grows. Records data: count:u16 then contiguous fixed-size records; decoded_len is total record bytes, exact. Raw mode accepts only raw chunks. Zstd mode accepts raw or zstd chunks. DAG mode accepts only records. Accepted counts cumulative decoded object bytes for cold modes, and cumulative distinct staged record bytes for DAG mode. Bound record batches so the complete frame fits64KiB. All status replies obey the same frame identity/sequence.

Begin uses sequence0. Successful state-changing requests then use the next sequence, starting1, including Finish/Status/Cancel/Resume. Reply failures do not advance sequence; a validation failure may terminate the attempt. Keep every accepted request hash/reply for up to4096requests per transfer within charged quota. Older duplicate requests are matched against this ledger, not guessed after a short retry window. After process restart Resume uses sequence0 with its complete immutable Begin body and matching DescriptorId; only a verified existing publication can return Published, otherwise Unknown.

One authorized Call obtains one Reply. Up to two outstanding Data requests per transfer may be sent, but receiver processing is ordered by request_seq. The current sender is synchronous stop-and-wait; a two-request pipeline is not implemented. An identical retried sequence/body returns the same application result; conflicting reuse rejects the transfer. A gap rejects that request without advancing it. Retries use fresh canonical mailbox message/correlation IDs; application IDs remain stable. MVM admission replay protection is distinct from application retry.

Warm Begin with a base requires exact authorized verified base closure pinned for the transfer before Ready(base_pinned=1). An unavailable base is a typed BaseMissing refusal, requiring an explicit new cold attempt. Sender sends target records absent from the base's complete closure. Finish independently checks complete target closure; Need requests complete descendant closures (not only parents), limited to two repair passes. Further misses reject this attempt. Exact repeat may return Published for an already verified authorized whole-object publication without DAG warming.

## Trusted boundary and authorization

The implemented `Authority` grants all namespace operations used by the endpoint; it does not itself authenticate a peer or implement distinct per-object read/write ACLs. An in-process `Authority` verifies authenticated context on every operation and immediately before publication/cache disclosure. Context binds tenant, authorization domain, sender, recipient, mailbox, policy_epoch, execution_epoch and capability/revocation identity. These values come from successful MVM envelope opening, never from asserted application-body fields. The application library trusts its authority implementation; constructing Rust values is not cryptographic authentication.

The endpoint API accepts context+frame and returns a reply; an adapter must authenticate/decrypt first, invoke current policy, and use the canonical reply handle. Namespace identity includes all authorization-domain/recipient/capability/epoch fields. Digest possession grants no rights. Resume across epoch changes is not accepted as continuation. Wrong-authority input cannot probe content membership, transfer state or quota use. No cross-namespace compression dictionary/dedup advertisement.

## Limits and reservations

Budgets belong to one Endpoint instance. The current deployment requires one endpoint instance per process for the stated process-wide ceilings; separately constructed library instances do not share a global allocator. The host supplies trusted monotonic elapsed seconds as `now`. Returned read buffers are caller-owned and outside the endpoint budget.

Defaults: object<=64MiB; decoded chunk<=60KiB; application payload<=64KiB; active transfers<=4 process-wide and per peer, additionally constrained by global budgets; staged nodes<=262144; scale<=13; pending payload<=128KiB/transfer; process resident reservation<=512MiB; namespace disk cache<=512MiB; aggregate disk cache<=1GiB. Logical output visits<=ceil(3*ceil(byte_len/32)/2). Idle30s, retention300s. Expired Status returns Error8; it has no State105 enum value. Active Resume requires the caller to retain next_seq or retry an unacknowledged request; it does not discover a lost client sequence. Every operation checks deadlines/current authorization. Checked arithmetic precedes allocation.

Reserve output, scratch, record/index costs and cache publication bytes before Begin. Use conservative per-node allocation charges, not just encoded record bytes. Cold whole-object transfer can skip DAG warming when its node/work budget cannot fit; report this separately. In particular, four unique64MiB DAG ingests are not promised to fit. Mandatory initial correctness demonstration uses objects through16MiB; larger-object qualification is separate. Namespace cache currently preserves immutable objects; explicit eviction may remove unpinned objects, never active base records. Staged data is excluded from verified cache inventory.

## Durability, retries and errors

States: admitted→receiving→verifying→published, or cancelled/rejected/expired. Transfer staging is inaccessible to normal reads. Validate record identities, scales, complete closure, zero padding, length and ObjectId before publication. Write content and publication descriptor to an owned temporary file, sync it, atomically rename, sync directory, then reply Published. A single writer lock owns load/modify/publish. An uncertain fsync/rename outcome must return an error; recovery revalidates any publication before reporting it. Cleanup removes only owned staging; interrupted staging is never interpreted as committed.

Publication identity binds namespace and ObjectId. Files are `hex(namespace)/hex(ObjectId).object`, with fixed header `SCPUB001`8 || byte_len:u64 || ObjectId32 || manifest_present:u8 || optional ManifestId32, followed by exact original bytes and no trailing bytes. A present ManifestId is written only after its complete DAG can be regenerated and verified under the node/work budget. The durable cache stores original bytes; DAG records may be reconstructed into bounded memory when pinning a base, with that CPU cost included in benchmarks. No unverified directory entry counts as a verified base. Namespace identifier is SHA256(`spiralcore.object.namespace.v1\0` || seven UTF8 fields in order tenant,domain,sender,recipient,mailbox,capability,authorization_scope, each prefixed by its byte length as u16LE, followed by policy_epoch:u64LE and execution_epoch:u64LE). Each field is1..256bytes with no implicit normalization. Context is trusted only through TransferAuthority. Limit published objects to4096 process-wide and charge file/header bytes. An exclusive process/file writer owns a dedicated marked endpoint root. Unix directories/files use0700/0600, unsafe or symlink paths are refused. Unmarked nonempty roots are refused; recovery removes only exact owned numeric staging names in canonical namespace directories. Temporary files are never considered publications. The current implementation targets Unix (macOS/Linux); other platforms require explicit filesystem protection support. A crash before publication discards volatile progress and Resume returns Unknown; a crash after publication can recover Published only after fresh authorization and exact verification. This v1 supports restarting a partial transfer; it does not claim persistent partial-progress resume. Cancel before publication releases reservations, clears staging and retains a bounded terminal tombstone. Cancel after publication returns published state. Published is object commit only, not mailbox delivery or application processing.

Error codes:1 malformed,2 unauthorized,3 limit,4 integrity,5 missing/base-missing,6 sequence/conflict,7 unknown,8 expired,9 storage,10 unsupported. Failed admission must not consume a transfer slot. Bounded terminal/retry entries expire after300s and are charged to resident quota. A repeated committed Begin remains idempotent through verified publication after tombstone expiry.

## Verification and qualification boundaries

Tests cover canonical binary vectors and exact byte reconstruction, raw/zstd/DAG behavior, lengths/padding, malformed/truncated/oversized data, missing closure/repair, replay/conflict/gap, revoke/epoch/wrongrecipient, quotas, cancel/expiry, restart and publication failure. Future canonical MVM integration must use pinned mvm-core and real envelope/open/replay APIs with test keys, not a replacement cipher; no such integration is claimed by this standalone implementation.

Benchmarks report application bytes, canonical envelope bytes and actual link bytes separately. All baselines use the same envelope/security/durability. Raw canonical MVM payload already incurs base64/JSON overhead; the5% cold regression target is relative to that same secured baseline, not naked TCP bytes. Local CPU/loopback evidence is not WAN or live MVM qualification. No production service, FIPS validation or classified-network approval is claimed.
