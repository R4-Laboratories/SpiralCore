# Current upstream refresh qualification

The host and guest consumers both pin public contribution commit `ee632fbb3a746709c496a95f167559a01a19fcaf`, based on merged MVM `b3b2afd8f5183b48632c6ff85c0781801d60aabf`. The narrow API contribution is [upstream draft #5328](https://github.com/tinylabscom/mvm/pull/5328); it does not close #4629 or represent the complete networking submission.

[checks.zip](checks.zip) is the original 44,058-byte archive. Its [receipt](checks.receipt.json) binds the archive and all fifteen inner files by length/SHA-256. The [source pins](source-pins.json) bind seventeen consumer product/test/lock files. All matched after the commands. All 4,437 regular files of the exact upstream-plus-patch source export were also checked after qualification. [The lock receipt](lock-repin.json) records that Cargo-generated host and guest locks differ from the previous locks only in MVM source identity; an incidental registry update was reverted through Cargo before testing.

All checks ran on Wick, Rust 1.97.1, release mode, two build jobs, incremental compilation disabled, nice level 10 and the existing shared Cargo target. Tests used one thread, fixed toolchain/system PATH and umask 022.

| Check | Executed result |
| --- | --- |
| Core all-target tests | 2,114 passed, zero failed, four existing ignored entries, ten target summaries including two zero-test targets |
| Host library `mailbox` scope | 35 passed, zero failed/ignored; 2,293 filtered |
| Guest library `mailbox_outbox` scope | 14 passed, zero failed/ignored; 1,014 filtered |
| Core strict all-target Clippy | Passed with `-D warnings` |
| Contract protocol-only/no-default-features check | Passed |
| Consumer all-target tests | Sixteen distinct tests passed, zero failed/ignored/filtered |
| Consumer strict all-target Clippy | Passed with `-D warnings` |
| Actual producer x86_64 Linux musl release build | Passed |
| Producer musl strict Clippy | Passed with `-D warnings` |

The isolated canonical-request fixture invokes itself in one child process. Its nested result raises the raw passed-summary sum to seventeen; that is **not** a seventeenth independent test. It constructs the real SDK request and verifies no VM runtime state was created.

The four ignored core tests retain their upstream scope: three live systemd cases and a fixture-regeneration command. They were not run. Neither full-workspace/feature-gated acceptance nor native mailbox/WAN delivery is established by this packet. Current-pin runtime preparation and a guest launch were not executed here. No transfer bytes, transfer CPU, send-only time or energy result is inferred from compilation or fixture duration. The previous pin's preparation and unsuccessful startup attempts retain their original identities.

Independent code/Rust and security reviews passed for the four-file API delta and coherent consumer repin. Source reviews do not replace the remaining execution gates. The wider gated workspace/conformance checks are continuing under the upstream draft before readiness for merge.
