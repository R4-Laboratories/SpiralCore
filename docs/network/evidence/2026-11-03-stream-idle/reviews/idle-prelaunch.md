# Idle API strict-transfer confirmation — prelaunch review

Date: 2026-11-03. Verdict: **GO for the registered bounded confirmation dose**,
after the documented source/binary checks succeed. No blocking resource, scope
or custody issue found in these four files. No launch, remote action, build or
measurement was performed by this reviewer.

| Reviewed artifact | SHA-256 |
|---|---|
| `registration.json` | `ed01d94a131254edec428e858ab6636cda2d87b64476125d7cd2496c044a18a7` |
| `execution-pins.json` | `7d1bd782f413b2b83e9659c88afd53384715ea06f268e52d3776df1133245c82` |
| `host-pairs.sh` | `1bfad40efe3f147296a59b51dd109525b2503d7d241f4a5e6a9a06bf00bba01c` |
| `native-linux.sh` | `a0dcc3a18fab0a4308e9d5fe575ec7f3f727eaba2aee2478bd02c01ebd1dc827` |

The dose is 12 host cells/six matched pairs: Whole/Combined, three small trials,
before/after with phase reversal for trial 1. Baseline probe `4109a318...` is the
prior C8 strict-transfer engine. The two native cells are current-source Whole
and Combined only. Historical native results must not be relabeled fresh paired
before/after evidence. The additive Idle method is not used by either engine;
these observations check legacy strict-transfer behavior/cost after the shared
framing refactor, not Idle wake CPU, timer retry or resume.

Pins identify source `960e73c59d7eab62f15a16667f95e594e1cabc8b`, product
`live.rs` `29010dde...`, 285 verified crate entries and protected normal host,
static guest and probe binaries. Both scripts verify staged source/binary
manifests before creating fresh result paths. Native additionally pins the
existing kernel/VMM, rejects ELF INTERP and pre-existing VMMs/shared link, and
cleans only a symlink matching its original target, inode and root owner.
No package, network, service, VM-profile or unrelated filesystem changes occur.

One cell is active at a time; native uses at most two NIC-less guests, each one
vCPU/1 GiB, with the existing 256 MiB disk per cell. JSON gates require unchanged
two links/two slots/524,288-byte limits and peak, zero final usage/refusal/fencing,
exact reopen and two-hop accounting. Host retains the Rust 512 MiB owned-store
and 1 GiB sampled-RSS bounds. On-Wick outer timeout 1800 is explicitly recorded
for each batch; per-cell timeouts are 120 host/300 native seconds. Timeout or
failure stops execution, preserves records and requires owned-process teardown
reconciliation before proceeding; cooperative TERM is not proof of joined tasks.

Host successful stores are removed only by the unchanged engine after exact
verification/public-record emission. Native scripts retain every disk pending
archive/member custody; only successful owned disks may later be removed. No
payload, VM image or new Cargo target is copied to Mac. Retain all regressions
and source/CPU/timer distinctions. Shared-host/fixed-order results establish no
causal/statistical speedup, WAN/fleet, energy or send-only claim.

The root reports release 92, shared adapter 123 plus object-v1 eight (131), Clippy
and static builds successful; this prelaunch review does not recount or pool
those checks. Focused Idle semantics and broader release/goal gates remain
separate from these strict-transfer confirmations.
