# Installed FlowMux: second public native run

The public runner passed a fresh Linux x86-64/Firecracker invocation at SpiralCore [`a7eea837`](https://github.com/R4-Laboratories/SpiralCore/commit/a7eea8373ea321e0fa873f82d6d42af45c2c9c47). The guest observed the refused destination, then recovered every byte of its **5,804-byte echo**, including the binary suffix and trailing zero. SDK, peer, guest entrypoint and namespace all exited zero. [Exact measurements and hashes](public-final-run.json) · [First public run](public-run.md) · [Reproduction](../../../../tools/installed-flowmux-witness/README.md)

## Measured transfer and lifecycle

| Guest operation | Application sent / received | CONNECT sent / received | Guest elapsed | Guest process CPU |
| --- | ---: | ---: | ---: | ---: |
| Refused destination | 0 / 0 B | 79 / 64 B | 9.544149 ms | 1.901599 ms |
| Permitted exact echo | 5,804 / 5,804 B | 69 / 39 B | 199.139856 ms | 4.863295 ms |

The peer independently required the original body and EOF before echoing it. It counted **3,661 incoming and 3,234 outgoing TLS bytes**, with **180.872756 ms** from accept through half-close. Those counters exclude TCP/IP headers. This is one same-host cooperating exchange, without a raw/zstd control or WAN qualification.

| GNU time accounting | Wall | User CPU | System CPU | Maximum RSS |
| --- | ---: | ---: | ---: | ---: |
| Installer | 7.70 s | 6.60 s | 0.50 s | 10,940 KiB |
| Controller | 15.35 s | 11.79 s | 2.20 s | 266,676 KiB |
| Peer, including waiting for the guest | 15.09 s | 0.02 s | 0.02 s | 8,476 KiB |

The SDK separately recorded preparation at 1.814730951 s, launch at 14.630560268 s, dispatch at 266.702106 ms and preparation through retirement at **15.090177144 s**. These boundaries overlap; they must not be summed. GNU time includes waited-descendant accounting where applicable and does not isolate codec CPU. The first public run's 12.329 s lifecycle and 185.678 ms echo remain separate observations. Two runs on a shared host do not establish a latency distribution or a causal performance change.

The installer-only untuned preflight recorded 21.88 s wall and 21.11 s CPU. The two tuned public runs recorded 7.43/6.62 s and 7.70/7.10 s respectively. The original archive and resource caps match, but cache warmth and host scheduling were not controlled. These are recorded setup costs, not a matched transfer speedup or physical-energy result.

## Source and build qualification

The controller, installer, peer and three host helpers use public MVM host source [`7b224540`](https://github.com/Casey-allard/mvm/commit/7b22454077530e96146d68204b711de53e3d05a8). The sealed workload contains the separately pinned official `v0.28.0-rc.2` guest runtime from producer source `849382ad417a28f3d68b051411b5ae3bab591b13`, with image-set `0.2.4` kernel inputs. It is not a single-revision guest/host build. Later MVM development merges are outside this result.

Hosted CI first exposed missing test-only dependencies in the standalone owner/supervisor harnesses. After compilation was repaired, the original crash test correctly refused a child selected under the wrong module name. The selector now follows its actual module, while retaining the original checkpoint, `SIGKILL` and on-disk recovery assertions. The owner runs all 18 owner cases alongside its 11 Linux or 12 macOS lifecycle cases. At the repair predecessor `77946142`, all four hosted owner jobs passed 29/29/30/30 with strict Clippy, and all three supervisor jobs passed their applicable groups. The original failed jobs remain visible in [PR #256](https://github.com/R4-Laboratories/SpiralCore/pull/256).

That predecessor's Linux functional and conformance stages passed, but strict `linux-host` Clippy found private retirement internals whose production consumers require `canonical-host`. Six private feature guards now match those consumers. The transient enum is not serialized; persisted `SCDISK01` phase bytes, canonical implementations, deadlines, receipt checks and refund/fencing rules are unchanged. No warning suppression or test skip was added.

On the final source, both feature configurations passed strict all-target Clippy. All **160 library tests** passed under default parallel execution. The owner passed 29 cases and strict Clippy; the supervisor's six Linux targets passed 33, 1, 19, 3, 2 and 11 cases. Its ordinary partial library still reports its unused private-component warnings. The previously skipped default guest check and synthetic analyzer also passed. Compact receipt hashes identify the commands and original outputs.

The changed controller executable has SHA-256 `5014165ac276ff48afd147ffb687d6cbe0a459c4ceca048147c7aed7206e15a8`. This fresh native run qualifies that executable; the older `1368832f` result is not relabelled. Independent pipeline verification preceded the separate code/Rust and security reviews of the final feature delta. PR-wide hosted checks remain the separate merge gate.

## Original ownership and limitations

The one-vCPU, 256 MiB guest had no NIC. The runner retained the 512 MiB filesystem and installer/peer/controller address-space caps of 512/192/768 MiB. Original guest, guest RAM and worker charges reached zero, the tenant stopped, and the four original retained reservation identities verified. Retained charges remain **806,371,328 host metadata bytes, 9,494,528 cache bytes and 536,875,008 staging bytes**; `all_resources_released` is false. No original helper, VMM, mount or loop attachment remained.

An independent readback verified the exact 2,336-byte signed audit copy against the original public key. Its expected session came from the original published guest observation, and its selected peer came from the independent input profile. It authenticated one allowed and one refused decision. The events still have a zero plan UUID and an unbound image field. They establish signed workload/session decisions; the separate launch records supply plan/image provenance. Plan-bound host audit remains an integration gap.

Private trial `native.QCrJpqLz` retains the original drains, public verification half and process measurements. After independent readback and checks for original holders, its disposable backing image was removed, recovering 318,996,480 bytes. That cleanup does not alter the measured retained ledger disposition. Public records omit its address, session, plan ID and key values. This witness leaves sustained/concurrent capacity, complete retained-store reclamation, two-guest installed FlowMux, other installed platforms, owner-access modes, canonical two-host WAN and FIPS/CUI qualification open under [the MVM delivery roadmap](https://github.com/R4-Laboratories/SpiralCore/issues/42).
