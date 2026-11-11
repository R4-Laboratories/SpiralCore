# Implement the canonical reliable mailbox endpoint

Implement the [reliable endpoint contract](../specs/2026-11-10-canonical-mailbox-reliable-endpoint.md) after the actual current one-way native baseline. User authorization covers this sequence. Start from inspected canonical upstream MVM `b3b2afd8f5183b48632c6ff85c0781801d60aabf` and the separately rebased four-file API patch, then recheck upstream before implementation. Preserve the existing SDK worktree and published evidence. All new product code is Rust.

The source already includes Ari's 256-message public outbox spool, backend-owned `HostDialRoute`, workload-ledger grant/authority epochs, separate encryption root and shredded per-launch secret. Extend those implementations. The old one-file producer and old per-host-only key assumption are superseded. The source additions do not yet retain a message through a lost/refused verdict or establish cross-host key interoperability.

The merged #5321 first-use kernel preparation applies to CLI launch paths. The direct SDK consumer still explicitly prepares and verifies its kernel/runtime before launch. Open [#5325](https://github.com/tinylabscom/mvm/pull/5325) proposes QueueFull-only retention and detached/transient lifecycle repairs; incorporate its final merged implementation before changing that branch, rather than duplicating it. Lost or ambiguous verdicts, other refusals and application replies still require the reliable contract below. Open [#5324](https://github.com/tinylabscom/mvm/pull/5324) adds an encryption guarantee matrix and socket restart regression, not cross-host key establishment. Neither open PR is part of this source pin.

## 1. Freeze legacy and admit the new mode

Own `mvm-contract/src/protocol/mailbox.rs`, `mvm-core/src/plan` validation/synthesis, `mvm-client/src/admission/mailbox_grant.rs`, `mvm-vmm/src/host/egress_bridge.rs` and the exact cmdline tests. First add REDs for an unsupported signed local version, malformed limits, conflicting cmdline versions and requested reliable service on an unsupported runtime/backend. Freeze old MailboxGrant and signed-plan JSON/signature vectors, old OutboxMessage bytes and legacy verdict bytes. Implement the optional skipped-on-None grant and explicit reliable token. `at_workload_epoch` must retain admitted version/limits while reminting the ledger epoch and its actual authority key. No reliable decoder is reachable through legacy sniffing. Update existing grant literals without changing their serialized legacy value. Keep the backend's original `HostDialRoute` selection and refusal; the adapter must not synthesize per-port or mux paths.

Run contract/no_std checks, focused plan/cmdline tests and current mailbox API tests. Review the serialization and launch boundary independently. A source-compatible backend capability report must reflect actual guest/runtime support before reliable launch can succeed.

## 2. Add the bounded local codec before sockets

Define local version, connection role, request/result/delivery/control types in `mvm-contract::protocol::mailbox`; use a new child module if it keeps the existing envelope readable. Add checked byte encoders/decoders in `mvm-core/src/mailbox/frame.rs` or its child module. Reuse canonical ContentType, IDs, operation/kind, Refusal vocabulary and destination-receipt signing/verifying. Keep old frame functions byte-exact.

RED vectors cover every field and opcode, zero/oversized length, bad reserved byte/version/role, unknown enums, trailing bytes, truncated prefix/body, content-type limits, bad nonzero IDs, reply-token size and integer overflow. Require decode/encode exactness and bounded allocation before accepting an external length. Add greeting role/generation mismatch and original five-second deadline tests with deterministic mock I/O. No real VM is needed for this step.

## 3. Extend canonical spool custody

Own `mvm-agentd/src/mailbox_outbox.rs` and the original guest listener. First REDs reproduce current remove-before-verdict loss and concurrent count-reservation hazards. Keep the existing outbox directory and 256-message ceiling. Add reliable-mode typed records, serialized count/byte reservation, checked monotonic request IDs and no-replace appends. Retain one oldest forwarding claim while allowing later writes to append within budget; do not replace the spool with one global producer slot. Keep current `write_outbox`, filenames, legacy wire and legacy tests unchanged by default.

Test EOF before/after a complete send, truncated positive reply, incorrect ID/digest/generation, malformed item, concurrent appends, refused admission, unlink failure, existing-claim recovery, result-spool exhaustion, explicit abandon/retire and u64 exhaustion. Write at least two messages back-to-back through the actual public producer API and deliver them in order, then test the full 256-entry count and smaller byte ceiling without dropping accepted items. Test that a stalled claim preserves later queued bytes, that hidden metadata counts toward quota and that a new helper generation cannot adopt old items. Positive matching local admission is the only normal removal condition. Use finite fake streams and clocks; no timer sleeps establish correctness.

## 4. Implement original host admission and budgets

Own `mvm-hostd/src/mailbox_service.rs`, `mailbox_launch.rs`, `mailbox_process.rs` and focused child modules. Add one broker-owned aggregate budget and per-scope reservations before per-service reliable activation. Implement the provisional/prepared/accepted/retired transaction and constant-size retired high-water mark. Retain the original canonical prepared envelope and exact request encoding. Use checked canonical sequence allocation; replace the reliable path's unchecked legacy sent counter with checked allocation.

First REDs cover two identical submissions sharing one admitted result/ciphertext, conflicting bytes, queue full preserving the guest request and existing cache, failed crypto releasing reservations, simultaneous duplicate preparation, cancellation/revocation/epoch/key change at the commit boundary, and every aggregate dimension across distinct VMs/tenants/senders. Track dedup/replay retention as charged state. Exceeding retained identity capacity refuses admission. Never silently discard accepted normal messages.

Revalidate current authority using canonical helpers and the private verified-open proof. Do not reopen a retry as a new envelope or bypass replay verification. Security review must inspect the new revalidation boundary, generation checks, resource release and reservation ownership. No state lock or broker dispatch lease spans external I/O.

## 5. Connect bounded Submit, Deliver and Control roles

Use only port 5257, the original service process and the explicitly signed reliable mode. Extend the reliable branch of the guest's one-at-a-time legacy accept loop with at most three admitted role owners. Host dials consume the backend-provided `HostDialRoute` and actual admitted VM identity. Register all worker handles and wake/shutdown paths with the original helper lifetime owner. The current epoch-secret owner and shredding path remain authoritative; a new connection, mode or retry cannot remint a secret or resurrect a terminated epoch.

Mock-I/O REDs require an indefinitely stalled Submit or Deliver to leave Query/Cancel/Retire and terminal replies responsive within their original deadlines. Cover duplicate role connection, stale generation, no host CID, partial greeting/frame, pending shutdown, full inbox and a lost DeliveryAck. Bound streams, worker stacks, frame buffers and timers. Add a deterministic no-lost-wake test for control and data ownership. Keep telemetry, FlowMux and machine-control budgets independent.

## 6. Expose typed workload and host operations

Add workload `cast`, `call`, `reply`, `cancel` and bounded inbox consumption to the existing agentd API. Add equivalent typed host operations under `mvm-client::mailbox`. Reuse the service state from step 4. Add reliable Unix claim/ack reads and audit-before-release so an audit error cannot consume a value invisibly. Keep the legacy host JSON request decoder separate from the reliable guest codec.

RED cases include reversed replies to two Calls, same correlation proposed for a later Call, wrong sender/recipient/capability/causation, differing legitimate source/destination epochs, expired and cancelled Calls, late Reply disposal, lost terminal result, Cast producing no application Reply, repeated Retire, and signed destination receipts producing no receipt recursion. Reserve state before reporting local acceptance. Assert independent acknowledgment states in tests rather than a single success boolean.

## 7. Integrate object-v1 through the shared endpoint API

Wrap the existing SpiralCore endpoint API as one mailbox application content type/version. Its authorized plaintext encoder runs before canonical encryption, and verified open precedes decoding. Use the same codec selection and exact object commit as the other endpoints. No alternative envelope, host key derivation or port is allowed.

First deterministic tests exchange exact object-v1 frames through both typed local directions and verify wrong/evicted bases, compressed literals, trailing zeros, insertion/deletion and replay/refusal. Then qualify one current native guest with the original helper. Record local role framing, both-direction totals, compute, setup and verified total time against an equally tuned baseline. A local successful transfer does not complete WAN acceptance.

Before a real cross-host transfer, implement the canonical recipient encryption-key agreement/wrapping contract tracked by #4633/#5312. The current per-launch secret stays on the sending host and its encryption root is independent of signing. A remote decoder cannot open that ciphertext from mailbox addresses or public keys alone. First REDs prove two independent hosts cannot currently open each other's sealed bytes, then test admitted key establishment, recipient/key-ID binding, mismatched/revoked/rotated keys, epoch replacement and termination. Use an MVM-owned implementation without exporting the raw launch secret. Only after that gate may this plan claim actual cross-host canonical traffic, retries or WAN measurements.

## 8. Finish lifecycle, two-guest and contribution gates

Run idle and stalled stop, cancellation during prepare/admit/send, lost-verdict reconnect, helper crash with retained guest slot, late Reply after restart and original process reap. Ephemeral restart must report Unavailable for lost local accepted state. Do not label it durable recovery.

Run two actual guests under the original orchestrator, followed by 2/8/32 endpoint admission/load tests with one shared host budget. Retain booted-guest counts separately from simulated endpoint counts. Publish capacity and saturation, exact bytes, CPU, memory and latency. Execute supported Linux/macOS/Windows role and actual WAN matrices before claiming the complete MVM contribution. Keep native backend support distinct from remote client/gateway support.

Update the SpiralCore issue graph at implementation, test and delivery checkpoints. Build the outside-contributor MVM PR from current canonical source only after exact release validation and independent code/security review. Link each fulfilled #4629 criterion to its executed evidence and leave incomplete platform, durability, transport, FIPS/CUI and capacity criteria open.

## Immediate implementation handoff

The next reviewable change is step 1 plus step 2 contract vectors. The pending native baseline runs independently and supplies a current endpoint reference. The three role connections are a concrete bounded design decision, not a request for new operator permission. The first mode remains ephemeral, extends the existing bounded many-message spool and serializes only its active forwarding claim. Any durable accepted mode, larger envelope or capacity budget needs its own implemented contract and measurements.

## Historical public witness limitation and upstream correction

Ari's [#5305](https://github.com/tinylabscom/mvm/issues/5305) installed witness is a useful new source checkpoint. At `f6636558`, `scripts/mailbox-live-witness.sh::write_frame` still creates `/run/mvm/mailbox/.witness.$$` and moves that path to `/run/mvm/mailbox/outbox`. Because `outbox` is now a directory, this leaves a dot-prefixed `.witness` file inside it. The real `spooled` implementation ignores that file. Its hermetic shell test models the body at the command boundary and does not exercise real spool discovery, so a passing mock does not establish delivery through this new layout. Preserve this source finding, then repair and test the public witness separately against `write_outbox`/actual visible spool behavior before using it for native acceptance. This documentation change does not modify the script or claim a native failure execution.

Recheck on 2026-11-11: merged upstream `46d50a5f9e300c34ff075d27388116bc6ccad6dc` (#5331) already repairs publication to `outbox/$(date +%s)-$$`, a visible spool member compatible with BusyBox `date`. The script SHA-256 at that revision is `12351c8804e1cff59b70361f8e9ee15168e3a53209402f4055ea32f1e25c1e6c`. Do not duplicate that fix or present the historical hidden-item defect as current. This resolves the source finding; actual native script execution remains a separate gate. Our SDK diagnostic uses canonical Rust `write_outbox` and is independent of this shell witness.
