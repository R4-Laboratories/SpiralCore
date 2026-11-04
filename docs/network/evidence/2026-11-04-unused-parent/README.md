# Original-parent cancellation and public dependency checkpoint

2026-11-04. Component development evidence; **no guest was booted**.

The canonical client returns the original preparation only for four pure
pre-conversion failures. The installed parent can cancel only that same private
owner, with matching full binding, no prior admission and no endpoint effects.
It keeps the reservation until the last original owner drops. This prevents both
leaking an unused reservation and treating an uncertain launch as unused.

The actual locked Linux release graph uses MVM
`77752e1967d20caf8bad710de864a0ccd967f9e6`, shared core
`ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1` and adapter
`13d6ec22bab21f60bdf2a184fa9eb069e5537200`. Metadata contains one source identity
for each critical MVM/core package and no local dependency overrides. The test
harness intentionally selects the actual local Live crate.

| Executed scope | Result | Boundary |
|---|---|---|
| Retirement policy | 4 passed | Private state machine, including unused cancellation |
| Resource accounting | 3 passed | Corruption, normal close and original cleanup identity |
| Installed parent integration | 8 passed | Preparation/admission/uncertain Drop; includes one API-shape test |
| Opaque Mock refusal harness | 1 passed | A Mock outcome cannot provide native retirement authority |
| Unused-cancellation harness | 3 passed | Foreign owner refusal, same-owner retry/retired refusal, prior admission refusal |

These are scoped entries, with overlap with previous checkpoints. They are not
19 booted-MVM tests, a full crate suite, or a capacity/WAN result. Protected
executables were copied and hashed before execution; nine source inputs and
four executables matched after execution. Both release builds exited zero.
Known dead-code warnings identify the unconnected disk/endpoint consumer and
future endpoint lifecycle states; they are preserved in the build records.

The receipt archive retains exact commands' output, exit codes, source/binary
digests and the critical-package metadata. Its SHA-256 is
`97ed18c588045e1d5d31712466796a0ee9a4b68d54809dd38b8969b3a1a0144c`.
It contains no executable, VM disk, payload dataset or credential.

- [Receipt archive](public777-receipts.tar.gz)
- [Code/Rust/security cancellation review](../2026-11-04-resident-parent/reviews/unused-cancellation-consumer-delta.md)
- [Canonical preflight review](../2026-11-04-resident-parent/reviews/unused-preflight-code-rust-security.md)
- [Parent API](../../../api/installed-parent-v1.md)
- [Preflight API](../../../api/unused-host-lifetime-preparation-v1.md)

Earlier intermediate Core/client preflight tests, helper tests and filesystem
tests retain their own pins and scopes. These new runs do not requalify all of
those components. Positive native retirement, resident endpoint/worker ownership,
aggregate service budgets, final-ACK recovery, sustained Calls, capacity and
canonical WAN/platform acceptance remain open. No new transfer-speed, byte,
CPU, memory or energy advantage is claimed by these lifecycle tests.
