# Restarting both mailbox helpers with the original Firecracker guests running

One original Linux x86-64 pair completed ordinal 1 across two controlled helper replacements. A staged a 257-byte synthetic payload, B performed an authorized empty read, and A forwarded its sealed original. The host verified B’s signed durable acceptance at 319.437 ms. The relay carried 5,846 bytes toward B and 1,198 bytes toward A. The initial signed receipt had reached A before interruption. [Original operation row](sender-ordinal-series.jsonl)

The SDK interrupted and collected its original A helper by its held child handle and started a replacement in 106.012 ms. It repeated the operation for B in 107.364 ms. Both original child statuses recorded signal 9. The original guest boots and signed direction remained live. A’s next Status returned the same message ID and signed receipt digest through the replacement A helper. This operation used the retained receipt at the source. B leased the original delivery through its replacement, returned all 257 bytes for independent comparison and acknowledged it at 1.178538 s. A settled the same receipt and advanced the contiguous prefix to 1 at 1.270976 s. The timings start at the beginning of the ordinal operation. [SDK restart control](https://github.com/Casey-allard/mvm/blob/cef3b6feaac41c42dda434ef98f541432c62995b/crates/mvm-hostd/src/mailbox_process/original_journal/reconnect.rs) · [Counters, source and executable hashes](results.json)

## Port 5257 on each original VM

The host counted bytes read from the guest and bytes accepted on writes back to it. Each counted row is one completed userspace exchange. SIGKILL prevented the first two helpers from writing a session footer. Their recorded prefixes establish a lower bound, and their final totals are unknown. Both replacements wrote a footer. Each recorded one write error in its final zero-byte turn after the positive data operations. [A original](original-sc-journal-a.counts) · [B original](original-sc-journal-b.counts) · [A replacement](replacement-sc-journal-a.counts) · [B replacement](replacement-sc-journal-b.counts)

| Helper | Completed turns | Guest → host (B) | Host → guest (B) | Write errors | Measurement |
| --- | ---: | ---: | ---: | ---: | --- |
| A original | 1 | 682 | 1,417 | 0 | Prefix |
| B original | 1 | 348 | 3,520 | 0 | Prefix |
| A replacement | 3 | 1,321 | 1,651 | 1 | Closed session |
| B replacement | 3 | 2,638 | 8,259 | 1 | Closed session |

## Execution cost and teardown

The application-to-agent Unix socket and the SDK application calls were measured separately. The A row includes the extra post-restart Status. The B row includes its empty control, lease read and acknowledgment. These counts and the sealed relay measure different interfaces. [Per-operation measurements](sender-ordinal-series.jsonl)

| Role | SDK request / response (B) | Workload → agent / agent → workload (B) | Guest workload CPU |
| --- | ---: | ---: | ---: |
| A | 555 / 936 | 1,560 / 3,282 | 2.735 ms |
| B | 155 / 902 | 2,108 / 4,841 | 1.821 ms |

The coordinator measured 9.107 s wall and 8.011 s process CPU across the lifecycle. GNU time around the invocation reported 9.21 s wall, 7.88 s user CPU, 0.60 s system CPU and 84,504 KiB maximum RSS. Both original Firecracker scopes ran with one vCPU, 256 MiB guest RAM and no guest NIC. The last sampled scope counters recorded 298,391 µs CPU and 38,858,752 bytes peak memory for A, and 289,642 µs and 38,846,464 bytes for B. Host load averages at start were 16.99, 17.23 and 17.12. This is a single functional trial. [Accounting domains and original hashes](results.json)

Both replacement helpers exited with code 0, unassisted on the first typed stop. The two original Firecracker scopes became inactive and their cgroups disappeared. The audit chain verified 17 entries, two sessions and two seals. Final observation found zero Firecracker VMMs and zero mailbox helpers. Receipt loss before acceptance reaches A, VMM restart, power failure, repeated helper turnover and WAN capacity require separate evidence. [Recovery and scale acceptance](https://github.com/R4-Laboratories/SpiralCore/issues/195)

## Build and measurement controls

The host binaries and canonical x86-64 guest runtime were built from the same clean MVM checkout at version 0.28.0-rc.2. The separately sealed workload images retained their original release-built application binaries. Both were reread from ext4 and matched their archived hashes. The contributor runtime manifest identifies the exact source commit. The source build took 2,241.57 s and produced archive SHA-256 `ccb00a298a612415e8925a26595dcfdc03db72145e0b9c0e607765b8a028b6a0`. The host helper and driver used unoptimized native builds. The witness lockfile updates 20 MVM path dependencies from rc.1 to rc.2 and declares its already locked TOML parser. [MVM source](https://github.com/Casey-allard/mvm/commit/cef3b6feaac41c42dda434ef98f541432c62995b) · [SpiralCore driver](https://github.com/R4-Laboratories/SpiralCore/tree/0b19ecb10fb3080a7e853af302b3e2114f9be232/tools/ari-own-mailbox-witness) · [Runtime, application and binary identities](results.json)

With measurement enabled on the predecessor, a real two-helper component test reached signed acceptance, then replacement readiness returned unexpected EOF. Both processes previously selected the same exclusive recorder filename in their original state directory. This witness permits exactly one original and one replacement set per VM. The existing validated reader binding selects the second fixed set. Exclusive creation rejects a third measured incarnation or a preexisting file. Each transcript keeps the existing 64-exchange limit and each aggregate uses constant space. The corrected component passed alongside 25 original-journal checks, three recorder controls, 22 driver tests and two preparation tests. Linux strict Clippy on the selected host feature, the Linux workspace all-targets check and the explicit BDD target compile passed. [Original driver invocation](../../../../../../tools/ari-own-mailbox-witness/paired-native.md) · [Component evidence hashes](results.json)
