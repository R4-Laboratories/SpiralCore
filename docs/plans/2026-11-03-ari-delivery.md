# Complete the networking contribution for Ari / MVM

This is the user-confirmed delivery goal as of 2026-11-03. It supersedes a
next-milestone-only interpretation of the continuing work. All five outcomes
below are required. The live [GitHub roadmap #42](https://github.com/R4-Laboratories/SpiralCore/issues/42)
and linked issues own status; component merges are intermediate progress.

## Required outcomes

1. **Finish the installed mailbox service** ([#43](https://github.com/R4-Laboratories/SpiralCore/issues/43)).
   Complete worker/VM ownership, aggregate plaintext/cache/workspace budgets,
   shutdown and orphan recovery. Preserve independent service queues and
   canonical MVM authority. Qualify actual supervisor and worker death, surviving
   backend exclusion, final backend exit and positive next-epoch admission.
2. **Finish persistent-session recovery and sustained operation**
   ([#43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
   [#49](https://github.com/R4-Laboratories/SpiralCore/issues/49)). Close final-ACK
   recovery, safe retention/expiry and same-owner native sessions beyond the
   current 2,048-Call witness limit. Qualify large transfers alongside control
   traffic, with bounded state and no accepted-message loss or repeated object
   execution caused by retry. Preserve original authorization and deadlines.
3. **Implement FlowMux and access endpoints**
   ([#44](https://github.com/R4-Laboratories/SpiralCore/issues/44),
   [#45](https://github.com/R4-Laboratories/SpiralCore/issues/45)). Use the same
   protocol/core with distinct service authorization and lifecycle. Encode at
   authorized plaintext endpoints; encrypted host relays forward envelopes
   without reading their contents. Preserve signed-plan, DNS, transforms,
   protocol/IP/port, rate, audit, cancellation and teardown decisions. Implement
   separately admitted development access and sealed typed operations.
4. **Execute canonical two-host WAN and platform acceptance**
   ([#46](https://github.com/R4-Laboratories/SpiralCore/issues/46),
   [#47](https://github.com/R4-Laboratories/SpiralCore/issues/47)). Qualify Linux/
   Firecracker and Apple Silicon macOS/HVF in their supported host roles, and
   Linux/macOS/Windows client/gateway roles in the declared version/architecture
   matrix. Finish Windows durable gateway/storage qualification. Execute direct
   and explicitly admitted relay WAN paths with actual MVM boundaries. Native
   Windows MVM hosting is not an established supported role; do not infer it
   from Windows client tests, cross-compilation or WSL.
5. **Freeze the supported capacity envelope and prepare the upstream package**
   ([#49](https://github.com/R4-Laboratories/SpiralCore/issues/49),
   [#47](https://github.com/R4-Laboratories/SpiralCore/issues/47)). Establish
   supported concurrency, throughput, resource bounds, overload behavior,
   recovery, sustained fairness and tail latency. Distinguish logical endpoint
   loads from booted VM counts. Prepare the reviewed, reproducible contribution
   and submit the authorized outside-contributor PR to `tinylabscom/mvm`, with
   every upstream acceptance criterion mapped to evidence.

## Execution order and evidence

Finish the active [installed supervisor implementation](2026-11-03-installed-supervisor.md)
and its actual native lifecycle first. Then complete recovery/retention and
large-object/control scheduling, followed by the FlowMux/access mappings. Run
WAN/platform acceptance and capacity qualification on that integrated source;
earlier component and transfer measurements retain their original scope.

Independent endpoint work proceeds alongside that critical path: the
[FlowMux mapping contract](../api/mvm-flowmux-v1.md) defines the admitted
cooperating-peer boundary, and the
[Windows private storage contract](../api/windows-storage-v1.md) defines the
initial native NTFS client/gateway profile. Their implementations require
independent review and executed endpoint/native tests before those delivery
gates close. The installed launcher and the final integrated source own the
shared lifecycle and acceptance requirements.

Use Rust product and measurement code, one explicitly versioned protocol/core,
exact lengths and byte readbacks. Record actual traffic in both directions,
setup, sender/receiver CPU, verified end-to-end time, lifecycle, memory, cache
and workspace costs against matched raw and tuned whole-object zstd controls.
Keep cold/warm, native/shaped/real-WAN results and all losses separate. Never
infer send-only time, energy, certification or fleet readiness from those data.

Reuse the current isolated worktree and bounded remote build resources. Keep
local storage clear, publish compact retained evidence, and remove only verified
owned disposable payload/build stores. The Wick native test budget remains at
most two simultaneous guests, each one vCPU/one GiB, without a guest NIC.

Review and merge validated changes through PRs to `origin/main`. Keep #42 and
each incomplete child open. Goal completion requires all five outcomes and the
upstream contribution; a green component suite or a single native pair is not
the completion condition.
