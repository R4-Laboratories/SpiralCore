# Explicit owned-filesystem workspace funding

Status: implementation plan for #258; not a completed qualification or new MVM authority. Baseline is SpiralCore c11cb092. Preserve all five #42 outcomes.

Independent source review identified the missing composition of directory buffers, retained store/name state, filesystem index/scan/eviction metadata, allocator rounding and existing worker stacks. Replacing further discarded body reads is optional and does not close that requirement.

## Contract

Expose derived `OwnedWorkspaceRequirements` and explicit `OwnedWorkspaceFunding`. `OwnedStorePolicy::small_v1` requires funding as its fourth argument; validate it before capture allocation, claim, recovery or unlink. Retain the accepted requirement/funding with the original lease. Numeric funding is a caller declaration that must come from its existing original reservation; it grants no protocol, mailbox, filesystem or egress authority. Existing immutable consumers continue using their pinned core until a separately qualified adoption.

The requirement separates requested heap layouts from the existing worker-stack reservation and explicitly requires the original owner's enforced memory envelope for allocator rounding/overhead. Do not claim a universal malloc or RSS bound. The frozen installed Linux model already checks effective process-address-space enforcement, controller/bootstrap metadata and main/session stacks separately. Preserve non-Linux production refusal until a corresponding bound or enforcement alternative is qualified. A CI service cap is not that original runtime receipt.

Keep the existing MVM 1 MiB filesystem floor. If the derived requirement exceeds it, refuse; do not grow host budgets. Do not add the existing 2 MiB session/reader stack to codec bytes again. Codec payload, attempt/ledger/history and decoder/prepared-state allocations remain in their existing operation/reserve accounting; carrier/plaintext and caller authority state retain their original payers.

## Bounded implementation

- Keep both actual-capacity-checked 16 KiB directory buffers.
- Store the captured component in fixed bounded bytes, and use bounded stack names/profile parsing/C-string arguments to avoid unquoted name conversions, including valid long macOS Unicode names.
- Preallocate the count-bounded cache index and scan-record vector at capture; check actual capacities before claim/recovery. Transfer their storage only to the single ordered endpoint. Their debit remains with the original lease/permit until final retirement even if a vector drops earlier.
- Use bounded stack storage for protected IDs, pending quota entries and candidate/victim lists. Preserve exact victim ordering by age and ID. No unbounded queue, alternate iterator or second codec.
- Hash namespace transcripts without allocating a concatenation buffer, preserving the existing bytes and domain. Public identity vectors remain compatibility controls.
- Compose typed allocation layouts and simultaneous ownership phases into the requirement. Report requested capacities, allocation count, required existing stack and the original-owner enforcement condition separately. Do not represent the requested layout as rounded physical memory.

## Controls and publication

First retain the missing-contract RED compile outcome. Then require exact derived funding to succeed and one-byte-short heap/stack funding to refuse before effects. Exercise 64 distinct one-byte objects, both namespaces, maximal valid residual stages, supported long names, scan/index overlap, eviction planning, recovery and retirement. Preserve 101-entry success, 102-entry refusal before unlink, original identities/locks, uncertainty quarantine, destruction-before-credit, and every existing deadline/security assertion.

Run bounded Rust qualification on Wick using the shared Cargo target, compact exports and no local build tree. Record actual readbacks, encoded/network bytes in their proper scopes, CPU, setup and lifecycle time. Get independent exact-source/security review, native CI and original-artifact review before merging the core contract. Then explicitly port the consumer into Ari's current owner; do not blindly advance the frozen contributor.

Refreshed public MVM `70a6d51a` includes Proposed ADR-062 and its open phase-0 launch/audit/epoch gates, schema-nine policy-evidence inventory (#5549, reader cap remains eight and production admission still refuses), and requested-egress intersection with an independently authorized owner grant (#5552, a pure projection with no production caller). These additions do not publish a compatible signed runtime or activate a new grant. Release smoke now recognizes dotted prerelease package names and narrowly compares re-signed Darwin code content; installed artifact identity must still record the actual bytes and signing context. Current FlowMux is under `supervisor/endpoint/flowmux` with VM-shared registry/rate/session resources, whereas the frozen contributor uses `supervisor/flowmux`. Preserve that ownership and the selected signed-runtime/private-MVMD gates. ADR-062's proposed density/burst figures activate no budget here.
