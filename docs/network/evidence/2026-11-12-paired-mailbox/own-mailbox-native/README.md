# Firecracker own-mailbox journal and guest reader

One real Linux x86-64 Firecracker guest submitted a **313-byte SpiralCore frame** to its own signed production mailbox. After the SDK stopped and removed the VM, the host retained a **12,009-byte journal entry** and an **803-byte anchor at cursor 1**. The guest received an authorization refusal. An exact guest readback, application ACK and verified transfer time remain open. The two-guest delivery gate is tracked separately. [Machine-readable evidence](results.json) · [Reproduction](../../../../../tools/ari-own-mailbox-witness/README.md) · [Issue #168](https://github.com/R4-Laboratories/SpiralCore/issues/168)

| Native trial | SDK and workload observation | Host journal entries after stop | Wall | User CPU | System CPU | Max RSS |
|---|---|---:|---:|---:|---:|---:|
| 1 · Ari’s journal draft | VM booted. SDK returned no mailbox posture. Driver stopped before the application Call. | 0 | 3.95 s | 4.40 s | 0.07 s | 82,540 KiB |
| 2 · Persistent first-start grant forwarded | SDK verified the Prod posture against the original plan and VM. Worker rejected runtime metadata from the guest agent. | 0 | 6.32 s | 4.57 s | 0.08 s | 81,648 KiB |
| 3 · Workload accepts the standard worker request | SDK verified the same posture. Application staged the frame. The production mailbox returned a reader authorization refusal. | 1 | 6.22 s | 4.53 s | 0.09 s | 80,136 KiB |

Each row used a fresh private MVM home, one 1-vCPU/256-MiB guest, the same version-matched runtime and the same synthetic body. The body contains all 256 byte values and one trailing zero. Each application Call used the complete encoded frame. The host signed a 10-second retention grant within its configured 30-second ceiling. The SDK stopped and removed each original VM. All three original audit chains verified with 11 entries, one session and one seal.

The table measures the host command from launch through the failed gate and cleanup. CPU and max RSS come from GNU time’s command accounting, which includes waited-for descendants. Detached service or VMM CPU and whole-host peak memory were not metered. Journal file sizes measure storage. The successful transfer, guest CPU, bidirectional socket bytes and WAN fields remain unmeasured. There was one attempt per configuration and no raw or zstd comparator.

## What the third trial establishes

The source recorded one send attempt. The journal's only retained entry and anchor demonstrate local host progress. The canonical journal must append a lease and advance the anchor before releasing a message to the authenticated guest reader. Cursor 1 and the failed workload provide no evidence of a successful lease, exact readback or application ACK. The public refusal combines several authorization errors. Neither the validator's specific error nor the original guest wall clock at that instant was captured. The host-local journal uses the trusted-host storage tier. [Guest reader handshake](https://github.com/tinylabscom/mvm/blob/543efc7f94aa756c09785b38bdd024b6ba8199c7/crates/mvm-agentd/src/bin/mvm-guest-agent/mailbox_reader.rs) · [Canonical journal lease transition](https://github.com/tinylabscom/mvm/blob/543efc7f94aa756c09785b38bdd024b6ba8199c7/crates/mvm-core/src/mailbox/journal.rs#L1096)

## Source and controls

1. Trial 1 used Ari’s exact journal draft. The persistent SDK launch discarded the request's first-start mailbox grant. [Ari’s journal PR](https://github.com/tinylabscom/mvm/pull/5377)
2. Trial 2 incorporated the contributor's existing first-start fix. The clean merge exposed a test call-signature mismatch. Supplying an empty retention policy then triggered the production guard. The final fixture supplies its own ceiling and signed duration. Both Linux regressions passed, covering the first request reaching canonical backend admission and the later persistent restart's lack of inherited authority. Only test-fixture code differs from the clean two-parent production merge. [Original contributor PR](https://github.com/tinylabscom/mvm/pull/5336) · [Qualified combination](https://github.com/Casey-allard/mvm/commit/1755a2dea880ae5b7fcdf0479ca7f9807face651)
3. Trial 3 used a newly sealed image whose pinned workload ignores the agent's runtime environment metadata and reads only the original request bytes and bounded timeout. The operator verified the ELF inside the sealed ext4 filesystem. The original guest archive, initramfs, runtime overlay, verified kernel and host binaries stayed the same as trial 2. [Workload correction](https://github.com/R4-Laboratories/SpiralCore/commit/01c8ad87856b81440353fed0c07bc703b12dbebd)

The next diagnostic is to distinguish host enrollment rejection, guest enrollment validation, and the journal's proof-of-possession or receive checks before lease issuance. Preserve the original time window and authority. Cross-guest and WAN delivery each require independent measurements.
