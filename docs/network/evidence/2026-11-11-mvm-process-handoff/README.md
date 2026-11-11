# MVM process-resource handoff qualification

Date: 2026-11-11. These are source-pinned Linux component checks supporting
[#111](https://github.com/R4-Laboratories/SpiralCore/issues/111) and
[#114](https://github.com/R4-Laboratories/SpiralCore/issues/114).
The complete installed MVM contribution remains unfinished.

## Implementations available for review

- Upstream draft [MVM #5340](https://github.com/tinylabscom/mvm/pull/5340),
  commit `7cd704aefe01ed90169037415dd1574f876fdfea`: the isolated MIT
  installation-owner/resource-holder primitive, based on upstream `a707f5e8`.
- Published development commit
  [`b0958e006`](https://github.com/Casey-allard/mvm/commit/b0958e006a9bd6b0dcdb0dcf771442fc60ce762c),
  tree `feefd3ed0463cd98ce835667d1e9888b4648c241`: typed exec handoff,
  real CLI/signer entry adoption and original-child watcher-job ownership.
  Its twelve-file delta excludes the evolving lifecycle gate, classifier,
  client publication hooks, daemon and host-agent coordinator.

Each exact Git export was streamed directly to Wick. All **4,446** regular
source files in the upstream export and **4,466** in the development export
were unchanged before/after their respective runs. No additive Core or daemon
overlay was present. The [qualification record](qualification.json) contains
commands, source/output hashes, environment and declared scope. Retained
outputs are included alongside this report; [evidence.sha256](evidence.sha256)
binds this compact packet.

## Executed results

Linux x86_64, Rust 1.97.1, release/locked, two build jobs, incremental disabled,
nice 10, umask022 and serial subprocess fixtures:

| Source and executed target | Passed | Scope |
|---|---:|---|
| `7cd704ae`: VMM `installed_owner` integration | 11 | Ten behavior scenarios plus one fixture entry |
| `7cd704ae`: private `host::installed_owner` units | 6 | 1,070 unrelated tests filtered |
| `7cd704ae`: strict VMM library/integration Clippy | Pass | Exact declared targets |
| `b0958e00`: VMM `installed_owner_handoff` | 3 | Two behavior entries plus one fixture; ten refusal controls inside one behavior test |
| `b0958e00`: Hostd `watcher_jobs` | 5 | Three behavior entries plus two fixtures |
| `b0958e00`: actual CLI `resource_lease_entry` | 3 | Valid typed handoff, malformed refusal and normal unleased invocation |
| `b0958e00`: strict VMM / Hostd-signer / CLI scopes | Pass | Three separate commands, including the respective tests |

All executed scopes had zero failures and zero ignored tests. Exact source
received independent code/Rust and security reviews. Scoped rustfmt checks also
passed. Build/test command durations include compilation; transfer bytes,
endpoint CPU, transfer latency, memory and energy were not measured here.

The CLI build reported missing `cargo-zigbuild 0.23.0` and consequently omitted
embedded Linux host assets. The actual version/entry tests passed with that
declared build. Installed-package qualification still requires the pinned
toolchain and proper embedded assets.

## What changed

The original installation owner retains its exclusive owner lock. Each
participating resource retains an independent shared holder lease. A replacement
needs a separately opened exclusive holder lock; leases are lifetime evidence,
and canonical MVM authority still governs operations.

The exec adapter prepares two exact descriptors and bounded versioned metadata
before spawn, returns the original Child and parent lease without a fallible
post-spawn gap, and consumes the handoff before CLI or signer dispatch. The child
validates storage/record identity, restores CLOEXEC and acquires a fresh
independent shared lease before closing the inherited aliases. The metadata
contains provisioning labels and a record digest, with no keys or boot grant.

The watcher owner reserves one finite job slot before thread/Child creation,
retains the original Child and lease through timeout/refusal, and produces a
private receipt only after actual collection and waiter join. An explicit retry
addresses the same retained job. Its activity getter is conservative and does
not manufacture a receipt. These components still need their complete admitted
host-agent/service wiring.

## Retained negative evidence

The [same-inode runtime RED](same-inode-ex-red.txt) executed the final test
against helper `9dc0af44f546a6f706fb9bf3a1c0c3e24525c081d44841c147f522e1f403af61`.
It unexpectedly accepted an inherited exclusive holder description and changed
the parent's EX lock to SH. A fresh independent SH probe succeeded when it
should have been blocked. The original Child was collected before that probe.

The [minimal fix](same-inode-ex-fix.diff) removes the inherited `flock` call and
its import. The final helper is
`d168a9418120f80e46c8639941e14933c63d9c6e3b33bc04e9cb25a531e5ffd8`;
test `2ad655ba6d5f2fbe184e762000413b5182949ed583848bc2c71998a218984588`
now requires both refusal and preservation of the original EX lock. Independent
SH acquisition supplies the required exclusion without converting that alias.
The exact published-source handoff scope passes all ten controls.

## Remaining delivery gates

Canonical fresh-install provisioning and explicit legacy reconciliation,
original signer/worker participation, cooperative service/connection drain,
aggregate resource admission and positive native retirement remain open.
The combined four-phase lifecycle/coordinator successor is development work.
Complete workspace/gated-feature/conformance checks, native platform roles,
two-host direct/relay WAN, fleet capacity and the concrete FIPS/security profile
remain required. No guest was booted by these component checks, and the earlier
native helper-zombie failure remains a failed lifecycle result.

Use [MVM #5328](https://github.com/tinylabscom/mvm/pull/5328) for shared mailbox
interface coordination, [#5336](https://github.com/tinylabscom/mvm/pull/5336) for
the SDK grant fix and [#5340](https://github.com/tinylabscom/mvm/pull/5340) for the
resource-exclusion primitive. The
[upstream implementation index](https://github.com/tinylabscom/mvm/issues/4629#issuecomment-6794294611)
links reusable implementations while the complete contribution is qualified.
