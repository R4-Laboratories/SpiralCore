# Small shared-core profile implementation

Scope: B6b prerequisite for FlowMux16KiB blocks/64KiB artifacts and OwnerAccess;
one shared codec/endpoint core. Contract:
[small-resource-profile-v1](../api/small-resource-profile-v1.md). C8 completed-tail,
installed/native lifecycle and Windows FFI extraction sources remain frozen.

1. Review contract and the pinned C workspace inequality before product changes.
   Capture a public missing-API RED and baseline fixture bytes/CPU/time/context
   sizes using the existing General implementation; measurements do not prove
   admission bounds. Keep the existing general profile and signed limits.
2. Implement the resource module and bounded common CCtx factory through safe
   APIs. Whole/Schema/selector constructors become additive; no codec copy or
   unsafe FFI. Derive source-length-dependent workspace before allocation and
   retain sizeof guards afterward. Unknown source settings refuse SmallV1.
3. Thread the same profile through session admission, Runtime conventional
   candidates and bounded prediction construction. Sender input length AND Vec
   capacity are checked before preparation. Keep all evaluated codecs charged.
4. Add receiver profile through Endpoint/ObjectEndpoint, keeping general open
   unchanged. Replace only Small Runtime's global32MiB prediction reservation
   with its bounded cap; reject replay length/shape before output allocation or
   evolution. Bound/disable retained decoder caching under the same envelope.
   Coordinate endpoint.rs with the factory owner; never touch storage/Cargo.
5. Expose ObjectSender constructors to real factory consumers. Root/retained
   reviewer reserve the declared operation envelope in their shared host ledger
   BEFORE any profile-owned setup, and enforce application purpose before core
   dispatch. No private adapter resource formula or independent quota.
6. Run focused RED/GREEN, existing600-byte/large General regressions and strict
   Clippy in the owned Wick jobs2 target, no VM. Compare both profiles on identical
   fixture inputs with exact readbacks and actual bytes/setup/send CPU/wall;
   report context/buffer sizes and RSS as observations separately from bounds.
   Request independent source/security review of the exact frozen source.

No native/WAN/energy or full B6b/fleet-capacity claim is established by these
component tests. A profile or recipe that cannot meet8MiB must refuse honestly.

Measurement clarification before final frozen-source comparison: retain the
initial default-General observations and add the existing General
ColdGraphPolicy::Disabled control, because SmallV1 explicitly disables eager
graph construction. Run that control on the same before2a9/after sources and
identical three-trial fixtures. This separates optional graph policy from the
codec-profile comparison; it does not change target data, methods or source
admission bounds. Keep initial records and every regression. Build before
timing; execute the exact test binary so compiler RSS is excluded.
