# Graceful exit of the original paired mailbox helpers

The first native A→B transfer recovered every byte and verified B's acknowledgment. Both helpers still needed assistance after the VMMs stopped. Their ordinary lifetime rule kept each helper serving while either the SDK launcher or the VMM lived. The SDK had to stay alive and retain the state directories to collect the original helpers. [Native transfer](../paired-durable-native/README.md) · [Original SDK custody](README.md) · [Acceptance issue #168](https://github.com/R4-Laboratories/SpiralCore/issues/168).

The paired launcher now hands the original VMM's process ID and birth identity to its helper on the private launch pipe. The helper checks that generation before and after arming its kernel exit observer. It observes exit in the service loop, during the wait for a first guest byte, and during reconnect backoff. On normal exit it closes its serving loop and joins its local listener. The SDK still verifies VMM exit, collects its original child, completes accounting, and only then releases that generation's custody. The ordinary mailbox keeps its existing launch format and lifetime. [Tested source](https://github.com/Casey-allard/mvm/commit/628bf93a2573b21287776dab1c9ac89621242d59).

The collector allows up to one 12-second in-flight exchange to unwind, subject to the caller's deadline. It reserves the smaller of 250 ms or one quarter of the remaining time for final collection. If orderly completion still fails, the SDK can assist its original child after verified VMM exit and reports that assistance. On an unresolved deadline it retains custody. A normal helper exit alone does not attest to every in-flight operation or meter footer.

## Linux process regression

The fixture launched two real mailbox helpers, representing each VMM's lifetime with a separate process. Three submissions carried the same 257-byte object. The fixture discarded the first guest reply, verified the signed V2 receipt on retry, compared the repeated receipt, and completed B's exact readback and signed ACK. It then stopped only A's original process while A held a guest stream waiting for a first byte. B had to remain available until its own original stopped. The launcher, both state directories and both ownership markers remained present.

| Observation | Before | After | Same-binary repeat |
|---|---:|---:|---:|
| Elapsed from A process termination request through A helper collection | Still pending at 2,000 ms | 206.620 ms, exit 0 | 206.249 ms, exit 0 |
| Elapsed from B process termination request through B helper collection | First deadline ended this case | 15.666 ms, exit 0 | 15.555 ms, exit 0 |
| First guest connection through B's signed ACK | 57.692 ms | 57.992 ms | 59.886 ms |

The fixed run transferred 5,498 bytes from A's helper to B's helper and 1,128 bytes in reply on each of the three turns. B's reader transferred 357 bytes toward its helper and received 5,654 bytes for the lease, then sent 2,203 and received 2,207 bytes for the ACK. The independently timed repeat consumed 0.04 s of user CPU, 0.01 s of system CPU and 0.31 s of wall time for the complete test process tree. That repeat used freshly issued signed credentials and has its own measured wire sizes. Per-role CPU, VM traffic and WAN performance were outside this test. [Per-turn bytes, executable hashes and timing](original-lifetime.json).

## Reproduction

Use Linux, a fresh private fixture directory and the pinned source. Forty mailbox tests passed and the separately executed two-helper test passed. The separately built helper and test executable must come from the same source and Cargo target. [MVM source](https://github.com/Casey-allard/mvm/commit/628bf93a2573b21287776dab1c9ac89621242d59) · [Patch from the SDK custody baseline](original-lifetime.patch).

```sh
cargo build --locked -p mvm-hostd --bin mvm-mailbox
cargo test --locked -p mvm-hostd --lib mailbox_process:: -- --test-threads=1
cargo test --locked -p mvm-hostd --lib original_a_and_b_helpers_relay_same_sealed_intent \
  -- --ignored --nocapture --test-threads=1
```
