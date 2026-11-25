# Canonical pack health and signed mailbox intent

The reviewed development checkpoints extend the original-instance and committed-policy sources
described in [PR #331](https://github.com/R4-Laboratories/SpiralCore/pull/331) and
[PR #332](https://github.com/R4-Laboratories/SpiralCore/pull/332). The installed consumers
remain MVM `f2ddbcbf` and SpiralCore core `8994951b`.

## Pack health boot

[MVM `ab161fda`](https://github.com/Casey-allard/mvm/commit/ab161fda830aa4de1c06e772ae048a2a6af09db4),
tree `772203af9717186dbdf9c32eedc5e0c476ca54df`, integrates canonical
[#5666](https://github.com/tinylabscom/mvm/pull/5666). The pack health boot carries the verified
evidence for its exact attached release proof into admission. The original pre-dispatch config
verification runs again after the admission substrate is applied. The client retains the verified
set through boot ownership.

Linux x86-64 passed 36 tests across four scopes and strict all-target client Clippy. The canonical
signed-workload test remained ignored because it requires a genuine signed runtime, published Linux
proofs and an existing sealed workload. The original harness recorded an empty selector. The
retained-journal regression then executed on the identical ELF. Test processes totaled 0.20 s wall,
0.14 s user CPU and 0.10 s system CPU, with maximum RSS of 41,456 KiB.

The 4,865-file manifest hashes to
`909b6f0efc2788f4e30da87ff4babb8d7d5744701413a810cb152354754dcda7`.
The verified 30,702,068-byte test ELF archive hashes to
`85a29bc0d6b40212fc7e20700b5c8d85717e41d53ece7d73a815183521779c68`.

## Signed intent and durable receipt recovery

[MVM `c515f382`](https://github.com/Casey-allard/mvm/commit/c515f3823ed4bbdbe0171637e2da21c8f8affe48),
tree `801a3d42ee22c335a4c8a4e0aeca8e2e07cc27e4`, integrates canonical
[#5668](https://github.com/tinylabscom/mvm/pull/5668) through `7fd22815` and its preceding rc.10
source changes. Canonical signed V3 intent is carried into the plan. Ordinary and original installed
launch handlers refuse unsupported V3 before dispatch effects.

- Local acceptance reserves completion and checkpoint capacity and returns success only after the
  exact signed receipt is retained. Recovery validates the original operation and authority.
- Directed messages keep the signed recipient route, authenticated remote exchange and original
  absolute deadline. The local recovery path stays separate.
- Journal replay identity, physical record and byte limits and both-generation checkpoint funding
  remain enforced. Opportunistic maintenance defers when capacity is insufficient.
- A malformed present plan reaches the started-mailbox rollback. Its startup error and any original
  stop failure retain their separate causes.

Independent review covered the routing and admission seams, preservation of original checkpoint
limits, stored receipt compatibility, and the fixture adaptations. The first full journal scope
completed with 35 passed and four failed because its fixtures assumed pre-v4 completion accounting.
An empty selector in that first campaign produced no executed test.
The revised fixtures keep the quota boundaries and require two fresh records, with no extra record
on the recovered retry. The final journal scope passed 39 of 39. The complete final focused library
run passed 149 tests across 13 scopes, and the signed-selector contract suite passed five more.
Strict all-target Clippy passed for the nine compiled packages. The library test processes
totaled 2.49 s wall, 1.17 s user CPU and 0.14 s system CPU, with a maximum RSS of 63,100 KiB.
The source manifest covers 4,871 files and hashes to
`2c4a3a51df8c04ff41c8118fec86ba115782fffc5c9b047d7ee3bea634878df0`.

| Verified ELF archive | Bytes | SHA-256 |
|---|---:|---|
| Final library controls | 103,399,956 | `973251743a9d974834de12f7e31af20fff30bb6a3885e5af847a3af70a1bb2d4` |
| Signed-selector contract controls | 764,568 | `4620498b8553d3f9ca0e68b571c73b1bf9693c7f8cc523158e0c060bdac1cda3` |

## Current integration gates

Canonical main was refreshed to `bc019bab`. [MVM #5945](https://github.com/tinylabscom/mvm/pull/5945)
sets macOS 26+ Apple Silicon/HVF as the supported Apple tier along with Linux x86-64 and ARM64
with KVM/Firecracker. The [SpiralCore role matrix issue](https://github.com/R4-Laboratories/SpiralCore/issues/46)
retains native Windows client and gateway qualification. Native Windows MVM hosting remains future
work. WSL coverage has its own role.

Later canonical egress changes check each call against the workload epoch and close existing
flows when that epoch loses admission. These changes remain outside the tested rc.10 development
candidate. [Per-call gate](https://github.com/tinylabscom/mvm/pull/5847).
[Live-flow revocation](https://github.com/tinylabscom/mvm/pull/5867).

These Linux component checks exercised local fixtures. WAN traffic, end-to-end transfer and
sender/receiver transfer CPU remain unmeasured in this slice. Ari's canonical audit-health and
original-deadline interface reply and compatible signed guest runtime remain outstanding. The rc.11
run failed the documented Firecracker and HVF lanes before publication. [rc.11 run](https://github.com/tinylabscom/mvm/actions/runs/44278080837).
The original installed readiness, joined resource and storage acceptance [#258](https://github.com/R4-Laboratories/SpiralCore/issues/258),
private MVMD, and the full [#42](https://github.com/R4-Laboratories/SpiralCore/issues/42) delivery
requirements remain open. [Audit interface request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842).
Exact artifacts and the retained failed runs stay on the HDD.
