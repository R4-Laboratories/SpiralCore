# Canonical workload epoch admission

[MVM development commit `abd5155f`](https://github.com/Casey-allard/mvm/commit/abd5155f33d6e8ab18a5b144b686891165b3fd3c), tree
`3e9f3b125e49b41bbb859848bca6484a8f6ba0db`, extends the signed-intent checkpoint `c515f382`.
It imports the coordinator-owned ledger, the nonzero `ExecutionEpoch`, and the shared
per-workload `CommandGate`. The coordinator publishes ledger changes with a recoverable
commit obligation. Readers refuse unresolved state, and the first migration preserves the
legacy bytes while fencing the old writer path. The direct coordinator observation uses one
bounded socket exchange with kernel peer identity. [Ledger owner](https://github.com/tinylabscom/mvm/pull/5758).
[Epoch type](https://github.com/tinylabscom/mvm/pull/5793).
[Shared gate](https://github.com/tinylabscom/mvm/pull/5803).
[Coordinator observation](https://github.com/tinylabscom/mvm/pull/5846).

The ordinary workload endpoint checks the signed workload and epoch before binding listeners
and on each egress decision. The existing destination, DNS and audit checks retain their order
within the authorized path. Linux confinement grants the endpoint read access to the canonical
ledger directory. The original object mailbox bootstrap uses the same typed ledger decision
and signed workload key, with its existing original directories and reader checks.
[Canonical endpoint admission](https://github.com/tinylabscom/mvm/pull/5847).

## Executed checks on Wick

Rust 1.97.1 on Linux x86-64 passed formatting, strict all-target Clippy for
`mvm-contract`, `mvm-core`, `mvm-vmm`, `mvm-hostd`, `mvm-client` and `mvm-cli`, and
135 focused tests across ten scopes. The scopes cover the shared gate, egress decisions,
host grant admission, endpoint admission, fresh ledger creation, two interrupted commit
positions, compare-and-swap replacement, detached observation and startup behavior.
Independent review covered the durable owner, direct observation, signed-workload key,
shared egress decisions and read-only ledger confinement. Execution of the actual
confined endpoint on a native backend remains pending.

| Linux x86-64 qualification | Wall time | User CPU | System CPU | Peak RSS |
|---|---:|---:|---:|---:|
| Admission strict Clippy | 66.85 s | 123.32 s | 8.28 s | 1,824,224 KiB |
| Admission focused test build | 29.03 s | 55.10 s | 4.22 s | 2,578,660 KiB |
| Admission test processes | 3.14 s | 0.03 s | 0.06 s | 47,400 KiB |
| Revocation candidate test processes | 7.40 s | 0.06 s | 0.16 s | 25,676 KiB |

The initial integrated checks failed at the installed mailbox's raw epoch access and
the detached client's Unix socket-path import. The final revisions route the mailbox
through the typed decision and restore the scoped import. The exact sources were
checked against 4,877-file manifests, with SHA-256
`37c824fc77536a6c26297deb69d5efd2a404f27091e5ef012030127d88a32d5b` for admission and
`81edeb8debfd3daef8cd2da43a2d168d286f82d6ef6120a562c3f08ffaf0f8ce` for the watcher candidate.
All four executable hashes were verified across each test run and from each archived member.

| Verified compact ELF archive | Bytes | SHA-256 |
|---|---:|---|
| Admission | 96,727,883 | `183e32c36e3221e54e3e1e6c14fc8c583b6aae5324de156815f3a54fc649324d` |
| Revocation candidate | 96,796,890 | `4699e08296d487a490ae056cc2ccc363ab008dc4b6face3b2fee8fcd18dc671a` |

The Wick HDD retains the source manifests, failed build logs and exact executable archives.
Network bytes in each direction, setup and sender/receiver CPU, verified end-to-end delivery
time and lifecycle costs remain unmeasured for these sources.

## Live-flow revocation review

Independent review of development candidate `52ec86560daa8d1c1468954922577c6a28c37d1f`,
tree `92f167f2a7b6aee9c66c85656d48a7d706c66874`, found a blocker in the imported
watcher despite 149 focused component passes. After the ledger refuses the epoch, the callback waits for the bound audit write
before closing the endpoint process. A stalled writer can hold that return and delay closure
of existing flows beyond the stated bound. Publication covers the admission checkpoint;
the watcher remains a development candidate. The remaining regression must demonstrate
timely closure of established sessions with audit stalled, alongside signed revocation
evidence with the existing plan identity. [Canonical revocation acceptance](https://github.com/tinylabscom/mvm/issues/5848).
[Watcher](https://github.com/tinylabscom/mvm/pull/5867).
[Unreadable-ledger grace](https://github.com/tinylabscom/mvm/pull/5874).
[Commit-marker regression](https://github.com/tinylabscom/mvm/pull/5925).

## Installed gates

The installed partition has a separate dispatch and owns its descriptor-confined transport,
audit, worker budget and retirement. The selected consumers remain MVM `f2ddbcbf` and
SpiralCore core `8994951b`. The audit request still awaits Ari's reply. The reviewed
recent source releases contained host helpers, and the rc.11 run failed the native
Firecracker and HVF lanes before publication. The original readiness and both #258
criteria remain pending on the installed path. [Audit request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842).
[rc.11 run](https://github.com/tinylabscom/mvm/actions/runs/44278080837).
[Storage and resource gates](https://github.com/R4-Laboratories/SpiralCore/issues/258).

Canonical upstream was checked through `4a32fa440`, which reserves an HVF owner
commissioning intent. The remaining [MVM delivery gates](https://github.com/R4-Laboratories/SpiralCore/issues/42)
cover installed ownership, persistent delivery, FlowMux and separate owner access, the
supported Linux/macOS/Windows role matrix with executed WAN evidence, and measured
capacity and fairness. Private MVMD integration remains external.
