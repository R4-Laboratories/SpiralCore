# Networking delivery status

Snapshot: **2026-11-04**, checked against published `origin/main`
`d8e90e98e5049721267d7204004132d34c05cb26` and the live GitHub issue graph.
This page separates accepted components, measured experiments and unmerged work.
The [README](../../README.md) is the architecture/results entry point;
[issue #42](https://github.com/R4-Laboratories/SpiralCore/issues/42) remains the
delivery authority. The complete MVM contribution is **not finished**.

## Published checkpoints

| Component | What the checkpoint establishes | Evidence |
|---|---|---|
| Shared object core and gateway | Versioned exact reconstruction, runtime/cache/schema selection, scoped verified storage and authenticated QUIC object service | [Object-v1](../specs/spiralcore-v1.md), [gateway](runtime-object-gateway.md), [matched results](cpu-conscious-results-2026-11-02.md) |
| Paired Linux packet gateway | Real IPv4/IPv6 TCP/UDP through TUN/QUIC, independent packet encodings, calibrated local impairment | [Packet report](packet-selection-results-2026-11-01.md) |
| Mailbox/session foundation | Canonical encrypted envelopes, independent keyless relay directions, bounded events, retained identity and explicit incomplete-Call recovery | [Events](mailbox-session-events-results-2026-11-03.md), [reconciliation](session-reconciliation-results-2026-11-03.md) |
| Installation owner and supervisor | Durable epochs, original resource-holder exclusion, exact worker custody, native transfers and a Linux death/recovery witness | [Owner](installed-owner-results-2026-11-03.md), [supervisor/native report](installed-supervisor-results-2026-11-03.md) |
| Shared managed-resource component | One bounded ledger, conservative accounting/fencing and component lifecycle tests | [Published component packet](evidence/2026-11-03-managed-resources/README.md) |
| Native Windows private storage | Isolated Win32 backend, protected NTFS profile and exact shared-core component tests | [Storage contract](../api/windows-storage-v1.md), [platform scope](installed-supervisor-results-2026-11-03.md#platform-scope) |
| FlowMux paired stream component | Authenticated TCP/TLS/object mapping, monotonic fatal delivery fencing and original FIN serialization | [PR72 release](evidence/2026-11-03-flowmux/pr72-release.md) |
| QUIC object finish custody | Original-deadline terminal ownership and retained fault evidence | [Finish packet](evidence/2026-11-03-object-finish/README.md) |

The latest FlowMux source is [PR72](https://github.com/R4-Laboratories/SpiralCore/pull/72),
merged at `25a1c786263afcc0a955927f10b441cea401dbd2`. Its
[PR77 evidence publication](https://github.com/R4-Laboratories/SpiralCore/pull/77)
preserves the older baseline and rejected intermediate rather than relabeling
them as the accepted source. The final seven-role run passed **34 tests per
role**, across nine nonzero scopes, with original ZIP digests and inner checksums
verified. These are 34 tests repeated across seven native process roles, not
238 distinct tests, cross-platform network pairs, or booted MVMs.

## Full MVM acceptance remains open

All five outcomes in the [Ari delivery plan](../plans/2026-11-03-ari-delivery.md)
are required. The linked issues were open at this snapshot.

| Gate | Required integrated result | Current boundary |
|---|---|---|
| [#43 mailbox](https://github.com/R4-Laboratories/SpiralCore/issues/43) | Canonical resident service, original admitted-plan/child ownership, aggregate plaintext/cache/workspace budgets, shutdown and orphan recovery | Published development witnesses and prerequisites do not complete the installed service |
| [#43 / #49 sustained sessions](https://github.com/R4-Laboratories/SpiralCore/issues/49) | Final acknowledgment recovery, safe retention/expiry, native qualification beyond the 2,048-Call witness guard and measured control/data coexistence | Explicit incomplete-Call recovery is published; general completed-tail/closed-stream recovery and production retention remain open |
| [#44 FlowMux](https://github.com/R4-Laboratories/SpiralCore/issues/44) | Actual admitted destination/protocol, independent queues/audit/rates, installed lifecycle and shared-core mapping | The paired stream component is accepted; canonical MVM activation is still unmerged work |
| [#45 owner access](https://github.com/R4-Laboratories/SpiralCore/issues/45) | Separately admitted development access and sealed typed operations, bounded lifecycle and shared-core payload mapping | Implementation is active and unmerged; no implicit shell/PTY permission or development fallback in sealed mode |
| [#46 platforms](https://github.com/R4-Laboratories/SpiralCore/issues/46) | Exact OS/version/architecture/role matrix and durable client/gateway qualification | Native component rows are recorded; remaining end-to-end platform and backend gates stay open |
| [#47 WAN/upstream](https://github.com/R4-Laboratories/SpiralCore/issues/47) | Current canonical direct/relay two-host WAN, reproducible supported package and reviewed upstream contribution | Historical WAN is a different source/transport; final canonical WAN and upstream package remain open |
| [#49 capacity](https://github.com/R4-Laboratories/SpiralCore/issues/49) | Aggregate budgets, bounded retained state, concurrency/fairness, overload/recovery and tail latency | Two guests and logical-endpoint loads do not establish a customer fleet envelope |

## Active work that is not an accepted main release

Development checkpoints are published for source inspection:

| Repository / development branch | Immutable checkpoint | Acceptance |
|---|---|---|
| [SpiralCore core checkpoint](https://github.com/R4-Laboratories/SpiralCore/tree/codex/spiralcore-core-checkpoint-20261104) | [`ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1`](https://github.com/R4-Laboratories/SpiralCore/commit/ca5d9ca1266541f71cc5ab7d0b17f4acf14d06b1) | Non-main source checkpoint |
| [SpiralCore integration checkpoint](https://github.com/R4-Laboratories/SpiralCore/tree/codex/spiralcore-integration-checkpoint-20261104) | [`665e23349e8cf3c644af989b2069815f7885b779`](https://github.com/R4-Laboratories/SpiralCore/commit/665e23349e8cf3c644af989b2069815f7885b779) | Locked graph and scoped parent/disk tests; full installed acceptance open |
| [MVM fork installed-service checkpoint](https://github.com/Casey-allard/mvm/tree/codex/spiralcore-installed-service-checkpoint-20261104) | [`8b833aee8be5c1be9ce0d1197b6b98d4d03625f6`](https://github.com/Casey-allard/mvm/commit/8b833aee8be5c1be9ce0d1197b6b98d4d03625f6) | Non-main contribution-preparation fork |

The [immutable graph packet](https://github.com/R4-Laboratories/SpiralCore/tree/4cece3276120606b85de91c554c3d96dc6e5f6d1/docs/network/evidence/2026-11-04-immutable-graph)
records an actual locked release build and seven installed-parent component
tests. It resolves one MVM Core/Contract revision and one shared transfer core,
with no local path overrides. These tests boot no guest. The newer
[disk-retirement packet](https://github.com/R4-Laboratories/SpiralCore/tree/665e23349e8cf3c644af989b2069815f7885b779/docs/network/evidence/2026-11-04-live-disk-retirement)
records ten Linux disk-module entries (including one process helper), one
scope-binding test, and five new macOS filesystem tests that overlap that module.
It preserves the APFS retry failure and independent source review. Its typed
retirement wrapper still needs the resident consumer and a positive sealed-receipt
integration witness; it is not an installed native recovery result.

New helper deadline/admission fixes passed 18 unique selected library tests,
nine resource tests and seven helper integration tests (five start original
helper children), with protected binary and source hashes unchanged. Component
strict Clippy also passed. Three GitHub
fork pushes failed with server errors; the exact successor commit and its
original failure/success records are backed up in a
[verified incremental source bundle](https://github.com/R4-Laboratories/SpiralCore/blob/665e23349e8cf3c644af989b2069815f7885b779/docs/development/mvm-helper-deadline-20261104.md).
The fork baseline above has not advanced. None of these checkpoints qualifies
the full combined service, WAN performance or formal upstream acceptance.
Branches may advance; immutable commits identify each recorded source.

The development tree contains canonical resident-parent and launcher work,
original-process guardian custody, shared parent partitions, disposable endpoint
disk inventory, and owner-access/FlowMux activation. Focused source reviews or
local process proofs for a slice are not final native lifecycle acceptance.
The native entry must carry the original authority, actual children and held
resources through every hop; DTO fields, hashes and persisted PIDs cannot replace
those owners.

Small-resource and Ordered-retention/shared-core candidates, their TLS readiness
optimization and MVM endpoint-pool integration are also separate from the frozen
PR72 component. A macOS sustained candidate missed its original 90-second gate;
a separate shadow diagnostic completion does not turn that normal gate into a
PASS or establish a released speedup. Source qualification, capacity accounting
and matched measurements must follow the final candidate, not an earlier result.

Resource models distinguish declared logical reservations from whole-process
RSS. Disk cleanup must retain both durable inventory and actual credit through
uncertainty; stream/driver metadata must remain bounded over sustained use.
No finite lifetime request counter or periodic all-VM restart is presented as
the completed sustained model. These are active integration gates, not a claim
that every existing component already satisfies the combined envelope.

## Read the native evidence by its actual scope

The [installed-supervisor report](installed-supervisor-results-2026-11-03.md)
contains current published local Linux/Firecracker and macOS/HVF pairs, exact
reopened lengths/bytes, lifecycle costs and the Linux original-process death
witness. Its larger tier has eight **14,851,384-byte** targets. Each guest has
one vCPU/1 GiB, at most two run concurrently, and no guest NIC is admitted by
these mailbox experiments. Configured limits are not measured RSS.

Canonical stream counts include both directions. A two-guest relay has two
vsock hops, so accounted stream bytes on both hops are twice the logical count;
neither is captured physical-network traffic. Sender/receiver guest CPU and
host/VMM CPU are separate, overlapping domains. No energy or send-only time is
derived from CPU. Fixed-order local pairs do not establish a causal OS comparison.

The [sustained broker report](sustained-broker-results-2026-11-02.md) records
13,684 Linux logical-endpoint operations and 20,537 exact readbacks, alongside
separate bounded native pilots. Logical endpoints are not the same number of
booted VMs. The complete installed cross-process resource/fairness envelope
requires its own saturation and recovery qualification.

## Platform interpretation

- Native MVM hosts in this program are Linux x86_64/Firecracker and Apple Silicon
  macOS/HVF, within their recorded profiles. Native HVF death recovery remains open.
- Linux ARM64, macOS Intel and Windows native client/component results establish
  their named process roles, not additional native MVM backends.
- Windows Server 2022/2025 x64 and Windows 11 ARM64 have native private-storage
  and portable stream/client component results. The private-store profile is
  NTFS-specific; full durable gateway/restart/WAN acceptance remains open.
- TUN packet attachment is Linux-native. A portable TLS/object component does
  not by itself supply a Windows or macOS TUN adapter.
- Cross-compilation and WSL do not substitute for native Windows execution.

See the [platform contract](../specs/2026-11-02-mvm-platforms.md) and original
artifact identities for exact versions. Do not pool a newer local host,
historical native backend and seven CI component roles into one platform claim.

## Failures and measurement rules remain visible

The linked reports retain smaller-session CPU/lifecycle losses, unrelated-data
overhead, schema/drift costs, congestion drops, original platform/fixture failures,
and rejected source/coverage attempts. A source correction or same-source rerun
does not erase the first failure or prove a speculative cause.

Object-finish tests distinguish authenticated completion from honest bounded
failure. A dropped encrypted close-phase datagram is not automatically a faithful
first-CLOSE-loss witness. No universal terminal-packet-loss recovery is claimed;
read the [finish evidence](evidence/2026-11-03-object-finish/README.md) for that gap.

Matched results require the same input, setup, policy and accounting boundary,
exact reconstruction/readback, both-direction bytes, endpoint CPU, completion
and lifecycle costs. Historical HTTP bodies, captured IP and encrypted canonical
streams are different counters. Existing gains are workload-specific; failures
and finite-queue losses remain part of the result.

[Public artifact index](public-artifacts.md) · [Optimization history](optimization-ledger.md) ·
[Delivery guide](delivery.md) · [Contribution workflow](../../CONTRIBUTING.md)

No FIPS/CUI, host-blind security, physical-energy, exact cross-node clock,
general-purpose router or production-fleet qualification is claimed.
