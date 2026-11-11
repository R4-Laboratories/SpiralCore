# MVM mailbox refresh for the SpiralCore integration

Inspected merged upstream: [`b3b2afd8f5183b48632c6ff85c0781801d60aabf`](https://github.com/tinylabscom/mvm/commit/b3b2afd8f5183b48632c6ff85c0781801d60aabf), checked November 10 local / November 11 UTC, 2026. The [earlier comparison](2026-11-10-current-mvm-mailbox-delta.md) remains a historical source review. This refresh supersedes its integration target, not its recorded measurements.

## What Ari has merged

| Change | Integration consequence |
| --- | --- |
| [#5302: bounded outbox spool](https://github.com/tinylabscom/mvm/pull/5302) | `/run/mvm/mailbox/outbox` is a directory of visible immutable message files. Use the canonical Rust `write_outbox` API; do not overwrite the directory as a single old-style slot. The public writer checks a 256-message ceiling. |
| [#5308: workload-ledger epochs](https://github.com/tinylabscom/mvm/pull/5308) | Preserve normal admission and grant reminting from the real workload epoch. Do not synthesize an adapter epoch or authority. |
| [#5309: backend-owned dial route](https://github.com/tinylabscom/mvm/pull/5309) | Consume `HostDialRoute` from the backend. Do not guess a Firecracker mux path or treat every backend as a port-socket implementation. Declared/hermetic support is distinct from a booted backend witness. |
| [#5317: separate encryption root and per-launch secret](https://github.com/tinylabscom/mvm/pull/5317) | Preserve canonical key custody and stop/rollback cleanup. A signing seed alone cannot derive the encryption key. The launch secret remains host-local; a remote host cannot reconstruct it from the mailbox address. Logical overwrite/unlink does not establish physical erasure. |
| [#5305: installed mailbox witness](https://github.com/tinylabscom/mvm/pull/5305) | Useful installed-CLI workflow source, but its shell publication needs the spool-layout correction described below before it supplies native delivery evidence. |
| [#5321: kernel preparation on first use](https://github.com/tinylabscom/mvm/pull/5321) | CLI persistent launch paths now prepare and verify missing kernels. Our direct SDK witness still explicitly prepares the kernel/runtime and records setup separately. |

The merged transport remains guest plaintext → port 5257 → original per-VM host helper → canonical sealing/validation → audited host receive. FlowMux remains a separate service on 5253. These changes do not relocate compression outside encryption or make the host blind to guest plaintext.

## What we are updating

The narrow verified-header/Reply-authority API patch is rebased on the merged source in the contribution fork at [`ee632fbb3a746709c496a95f167559a01a19fcaf`](https://github.com/Casey-allard/mvm/commit/ee632fbb3a746709c496a95f167559a01a19fcaf). Its four-file diff is byte-identical to the previously reviewed patch: SHA-256 `0e5af83ba7abafa95417b5f927ce91b965a0418d52e1b1fe8d50b40593b83790`. Independent code/Rust and security reviews pass for that delta. It adds no envelope, port or cryptographic scheme. The new public `ReplyOperationRequired` enum variant may require exhaustive-match updates; a Reply using Send authority is intentionally refused.

The standalone [canonical host/guest witness](../../tools/mvm-canonical-witness/README.md) moves both dependency manifests, both lockfiles and both reported-source constants together. Historical campaign pins and evidence remain unchanged. They cannot be blindly repinned: the older installed experiment depends on APIs and service semantics absent from this current upstream. The [reliable endpoint contract](../specs/2026-11-10-canonical-mailbox-reliable-endpoint.md) and [implementation sequence](../plans/2026-11-10-canonical-mailbox-reliable-endpoint.md) now extend Ari's spool, route, epoch and key owners.

The scoped upstream [draft PR #5328](https://github.com/tinylabscom/mvm/pull/5328) and [mailbox issue coordination comment](https://github.com/tinylabscom/mvm/issues/4629#issuecomment-6785864474) make the API contribution and existing SpiralCore mechanisms visible to Ari now. The draft does not close #4629. Whole-workspace/feature-gated/platform checks remain before readiness for merge.

[Current-source qualification](../../tools/mvm-canonical-witness/evidence/ee632fbb/README.md) is separate from the older API packet: 2,114 core tests passed with four existing ignored entries, 35 host mailbox and fourteen guest outbox tests passed, and core strict Clippy/protocol checks passed. The consumer passed sixteen distinct tests and strict Clippy; the musl guest producer built and passed strict Clippy. All source and archive identities were verified. No current canonical native mailbox delivery, object-v1 transfer or WAN performance improvement is established by a rebase or component test.

## Pending upstream changes we should reuse

At inspection, [#5325](https://github.com/tinylabscom/mvm/pull/5325) is open. It proposes retaining the spool head on QueueFull, slower redial under that refusal, detached-command return and interrupted transient cleanup. Reuse its final implementation after merge. QueueFull retention alone does not retain identity through a lost/ambiguous verdict or supply correlated application replies.

[#5324](https://github.com/tinylabscom/mvm/pull/5324) is open and adds an encryption guarantee matrix plus a socket-level restart regression. It is not live-VM, physical-erasure or cross-host key qualification. [#5327](https://github.com/tinylabscom/mvm/pull/5327) proposes fetching the pinned Firecracker release on first use. None of these open branches is silently included in the pin above. The proposed [ADR059 handoff in #5304](https://github.com/tinylabscom/mvm/pull/5304) is not a replacement for accepted ADR057 until adopted.

## Remaining concrete service gaps

[Mailbox issue #4629](https://github.com/tinylabscom/mvm/issues/4629) remains open. On the inspected source:

- The default launch template is Send-only Call. Full typed guest/host Call, Cast, correlated Reply, cancellation and late-reply disposal remain integration work.
- `forward_outbox` removes the file before reading the host verdict. Lost verdicts and refusals can consume the producer item. Do not acknowledge a verified warm cache on that assumption.
- Admission reuses a context timestamp captured at service creation. Long-lived operation needs a current trusted admission clock with deterministic expiry tests.
- Queue/frame bounds exist, but the replay protector's `BTreeSet` is unbounded, and declared rate/outstanding/queue-contribution fields alone do not prove aggregate accounting. Charge retention and aggregate plaintext/cache/workspace state under original owners.
- Host `recv` returns payload only, not an authenticated header proof. Our private-field verified-open result must be retained through the new typed dispatch; caller-supplied metadata cannot replace it. Audit-before-release and recovery semantics need explicit qualification.
- Cross-host canonical key establishment/wrapping remains required by [#4633](https://github.com/tinylabscom/mvm/issues/4633) and the delivery limitation in [#5312](https://github.com/tinylabscom/mvm/issues/5312). An outer encrypted tunnel does not make the receiver possess the new host-local inner key. Do not export the raw launch secret or invent a second envelope.

The shell witness creates `.witness.$$` and moves it to the new `outbox` directory. That preserves the dot-prefixed basename, which actual `spooled` discovery ignores. Its hermetic mock does not exercise that discovery. This is a source-identified publication mismatch, not an executed native failure; our Rust producer already uses the canonical writer.

Earlier current-runtime attempts on Wick are retained as unsuccessful: acquisition-mode refusal, missing user-session context, then a 30-second guest-agent startup timeout. No message readback was achieved. The last attempt does not identify a boot root cause; capture canonical console output before rollback on the next bounded attempt. Do not replace these results with a positive historical-fork witness.

The complete contribution still requires original-owner lifecycle, bounded reliable bidirectional operation, two actual guests, capacity/fairness, native platform roles and two-host direct/relay WAN acceptance. FIPS/CUI qualification remains a separate concrete-module and deployment gate. Neither an upstream feature merge nor this source refresh completes those gates.
