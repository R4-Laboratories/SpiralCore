# Networking delivery status

Snapshot: **2026-11-05**, checked against published `origin/main`
`72415e7c9b92e6ff95342d1bfd23daef6d479e32` and the live GitHub issue graph.
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

[PR #95](https://github.com/R4-Laboratories/SpiralCore/pull/95) adds canonical
transport-binding revocation to the mailbox adapter. Live Call and Reply keep
both checked policy generations through transport admission. Prepared receipts
recheck the source and return bindings before sending. The installed MVM owner
still needs to distribute current revocations and restore verified policy before
restart admission.

On the merged tree, each of seven native process roles passed 113 mailbox
tests, 24 storage/publication tests and 34 FlowMux component tests. The locked
Linux host job also passed its host and shared conformance suites, strict Clippy
and the guest-entry build check. The matrix covers native process roles.
Installed guest, direct/relay WAN, capacity and FIPS gates have separate
acceptance criteria. [Verified artifacts](https://github.com/R4-Laboratories/SpiralCore/issues/46#issuecomment-6670157754)

## Full MVM acceptance remains open

All five outcomes in the [Ari delivery plan](../plans/2026-11-03-ari-delivery.md)
are required. The linked issues were open at this snapshot.

| Gate | Required integrated result | Current boundary |
|---|---|---|
| [#43 mailbox](https://github.com/R4-Laboratories/SpiralCore/issues/43) | Canonical resident service, original admitted-plan/child ownership, aggregate plaintext/cache/workspace budgets, shutdown and orphan recovery | Published development witnesses and prerequisites do not complete the installed service |
| [#43 / #49 sustained sessions](https://github.com/R4-Laboratories/SpiralCore/issues/49) | Final acknowledgment recovery, safe retention/expiry, native qualification beyond the 2,048-Call witness guard and measured control/data coexistence | An installed Firecracker session completed 2,311 Calls and recovered a withheld Finish receipt after replay tracker expiry. Measured concurrency and production retention remain open. |
| [#44 FlowMux](https://github.com/R4-Laboratories/SpiralCore/issues/44) | Actual admitted destination/protocol, independent queues/audit/rates, installed lifecycle and shared-core mapping | The paired stream component is accepted; canonical MVM activation is still unmerged work |
| [#45 owner access](https://github.com/R4-Laboratories/SpiralCore/issues/45) | Separately admitted development access and sealed typed operations, bounded lifecycle and shared-core payload mapping | Implementation is active and unmerged; no implicit shell/PTY permission or development fallback in sealed mode |
| [#46 platforms](https://github.com/R4-Laboratories/SpiralCore/issues/46) | Exact OS/version/architecture/role matrix and durable client/gateway qualification | Native component rows are recorded; remaining end-to-end platform and backend gates stay open |
| [#47 WAN/upstream](https://github.com/R4-Laboratories/SpiralCore/issues/47) | Current canonical direct/relay two-host WAN, reproducible supported package and reviewed upstream contribution | Historical WAN is a different source/transport; final canonical WAN and upstream package remain open |
| [#49 capacity](https://github.com/R4-Laboratories/SpiralCore/issues/49) | Aggregate budgets, bounded retained state, concurrency/fairness, overload/recovery and tail latency | Two guests and logical-endpoint loads do not establish a customer fleet envelope |
| [#83 security/FIPS profile](https://github.com/R4-Laboratories/SpiralCore/issues/83) | Concrete validated-module/build/environment mapping, approved-mode enforcement, canonical crypto integration and customer security-control evidence | Explicit unclassified/CUI submission target; current builds have no established validated deployment profile |

## Installed Firecracker mailbox qualification

The [2,311-Call sustained run](https://github.com/R4-Laboratories/SpiralCore/blob/d0ede350f3055d8cd9bf6eabaaf814296ee4ef4f/docs/network/evidence/2026-11-05-installed-sustained-campaign/README.md)
transferred three 3 MiB objects on one authenticated carrier and observed
tracker expiry on both sides. The original MVM owner retired the guest before
the host independently read back all 9,437,184 bytes.

The [Finish receipt recovery run](https://github.com/R4-Laboratories/SpiralCore/blob/19cd77e39e4fc10d99431736082234a3fa22394c/docs/network/evidence/2026-11-05-installed-object-finish-redial/README.md)
withheld the receipt before send, reauthenticated the replacement and replayed
one exact encrypted Reply. The same guest finished all 20 Calls and the host
verified all seven publications after retirement. A separate focused test
refused replacement after a partial old frame.

The [combined sustained recovery run](https://github.com/R4-Laboratories/SpiralCore/blob/efcec38e265e883f8380f80e5e6cd43b622dbb5a/docs/network/evidence/2026-11-05-installed-sustained-finish-redial/README.md)
replaced the carrier after 1,540 completed Calls and observed tracker expiry
on both endpoints before the cut. The host replayed one cached Reply and
the guest resent zero Calls. The session completed all 2,311 Calls and the
host independently verified the three 3 MiB objects after original retirement.

These were signed synthetic Linux/Firecracker fixtures. Larger concurrent
data/control workloads, ambiguous partial receipt writes, the installed
release and cross-platform WAN remain open. The reports retain byte counts,
CPU, elapsed time, source pins and failed runs.

## Current installed FlowMux qualification

The [MVM contribution fork checkpoint](https://github.com/Casey-allard/mvm/commit/9c2361423c86082a48d23e3f5651cb3ff0881e3f)
includes the original host-agent registration and signer check. The
[hosted helper run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42135824037)
passed all eight tests, including foreign signer, changed registration facts,
parent revocation and same-name replacement controls.

Earlier pins on the same branch passed [x86 store cleanup and flag controls](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42132613337)
and [pre-TLS carrier and resource checks](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42134384311).
Both resource refusals emitted zero TLS bytes on an already connected loopback
socket. The funded failed handshake returned the original credit. The
[hosted scope check](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42135240936)
verified kernel CPU, memory, swap and task limits and scope retirement.

The [Runtime retention run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42140574877)
passed both focused tests. An inert endpoint and mock backend showed that the
original handle survives a boot-preparation failure, a competing operation is
fenced during cleanup, and retirement needs the original receipt. The fixture
did not boot a guest.

The [canonical executable baseline](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42141356436)
built and reproduced early EOF before the first framed Challenge. The
[reviewed fork candidate](https://github.com/Casey-allard/mvm/commit/9df8ceec88b4a49c1065719f147f323fbda75e86)
routes the actual binary into the original installed protocol. All 10 entries
passed in the [exact-source hosted run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42142321535).
The actual executable issued Challenge, adopted a valid Prepare and withheld
Ready until the audited controller is available. It also rejected an
incomplete Prepare and mixed installed/probe arguments. The next gate carries
the original audit and records descriptors through Prepare and attaches the
owned collector and FlowMux controller before Ready.

The [Hostd records-adoption run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42143892252)
passed all 13 integration tests, including inherited records identity, holder
alias refusal, and the one-time claim. Its VMM unit build exposed an existing
fixture missing the new optional field. The fixture-only successor preserved
the previous wire encoding and passed all nine selected installed VMM tests
in the [hosted follow-up](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42144793237).
The parent still needs to send both original audit and records descriptors,
and the controller still needs to adopt them before readiness.

The [original signer handoff run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42146422478)
passed 10 real-helper integration tests, three Hostd descriptor sideband tests,
and four VMM parser and readiness tests. The original signed registration can
claim the signer capability once and cannot deregister while it is held. The
parent and child still need to pass that capability through the full installed
endpoint startup and use it in the audited controller.

These checkpoints cover components and host prerequisites. The current
combined installed guest transfer, original-owner retirement and real-WAN
acceptance remain open. Follow
[#44](https://github.com/R4-Laboratories/SpiralCore/issues/44) for exact pins,
failures and successor runs.

## Active work that is not an accepted main release

Development checkpoints are published for source inspection:

| Repository / development branch | Immutable checkpoint | Acceptance |
|---|---|---|
| [SpiralCore core checkpoint](https://github.com/R4-Laboratories/SpiralCore/tree/codex/spiralcore-tls-startup-20261104) | [`1025d87d048b15cefd483b4c11c70a6e029cb29c`](https://github.com/R4-Laboratories/SpiralCore/commit/1025d87d048b15cefd483b4c11c70a6e029cb29c) | Original reader/deadline, terminal completion and startup EOF component; 43 Linux checks, installed/native/platform acceptance open |
| [SpiralCore integration checkpoint](https://github.com/R4-Laboratories/SpiralCore/tree/codex/spiralcore-retained-catalog-20261104) | [`8476d7583e083e02d8b6bc571fc505815b85de90`](https://github.com/R4-Laboratories/SpiralCore/commit/8476d7583e083e02d8b6bc571fc505815b85de90) | Original retained-file ownership, checked disk split and catalog-loss denial; 152 library entries pass, two ignored; scoped Clippy passes with three warnings; full installed acceptance open |
| [MVM fork installed-service checkpoint](https://github.com/Casey-allard/mvm/tree/codex/spiralcore-retained-telemetry-20261104) | [`9a3a964bd540d5e74c71f9b7df93cc5593295546`](https://github.com/Casey-allard/mvm/commit/9a3a964bd540d5e74c71f9b7df93cc5593295546) | Original retained-file handoff and canonical runtime cleanup protection; non-main contribution-preparation fork |

The latest [resident/helper witness](https://github.com/R4-Laboratories/SpiralCore/blob/7a080af1049455f6cb0e07f511cb467d05a902e8/docs/network/evidence/2026-11-04-resident-helper/README.md)
executes the original canonical daemon/signer through the resident's same backend,
then verifies shutdown and final-alias resource drain. It registers no VM and
boots no guest. The [capacity prerequisite](https://github.com/R4-Laboratories/SpiralCore/blob/f7d06da6b4aa148cd348dd91df3342f1bc53e399/docs/network/evidence/2026-11-04-retained-telemetry/README.md)
records a reproduced undersized-total defect and pre-effect refusal: 23 ordinary
parent checks pass, two actual-helper cases remain ignored in that invocation,
and strict Clippy emits no warnings. Both packets bind their exact Git sources;
iterations and overlapping test scopes are not summed.

The [retained-catalog successor](https://github.com/R4-Laboratories/SpiralCore/blob/8476d7583e083e02d8b6bc571fc505815b85de90/docs/network/evidence/2026-11-04-retained-catalog/README.md)
implements the original retained-file owner, disposable/retained disk split,
bounded catalog and durable denial after catalog loss. Actual fault tests cover
new append and restart after process exit, unreadable records and changed
directory privacy. Final focused five entries overlap the 152 passing library
entries; the broad run ignores two helper cases. Scoped Clippy passes with three
unconnected-constructor warnings. All 134 supplied compiled inputs match Git;
independent source/privacy/publication reviews and original failures are retained.
Incomplete-I/O recovery, closed-debt adoption, actual producer/controller coupling
and positive installed-native retirement remain open. The installed profile
remains inactive; these results add no native transfer, new WAN measurement,
accepted capacity envelope or FIPS claim.
Upstream MVM has also changed endpoint wait-failure finality, transient
volume/secret-session ownership and HVF stop handling; the final integration must
preserve those semantics before its own qualification. Earlier witnesses retain
their original revisions.

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
The fork publication subsequently recovered: the exact helper files are now
included starting at checkpoint `77752e19` and retained in the successor above.
The failed pushes and bundle remain
historical evidence.

The [earlier parent/cancellation packet](https://github.com/R4-Laboratories/SpiralCore/blob/7ff84ca7f808e1bc6d9c6d4c2ba5599207c7be47/docs/network/evidence/2026-11-04-unused-parent/README.md)
records actual locked Linux release builds against MVM777, shared coreCA and
adapter13d, without local dependency overrides. Four retirement-policy, three
resource-accounting and eight parent integration entries passed; the eight
include one API-shape test. A separate opaque Mock-refusal harness and three
unused-cancellation tests passed. Protected source/executable identities match.
These scopes overlap older tests; no guest is booted. Safe cancellation requires
the same sealed unused preparation and actual private owner, and prior admission
or uncertain effects refuse. Positive native retirement and resident endpoint
ownership remain open. Known unconnected endpoint/disk dead-code warnings remain
in the build receipt.

The [new resident-service packet](https://github.com/R4-Laboratories/SpiralCore/blob/ef4f5530cb4df41ef534dd63b149f2de5dd107b3/docs/network/evidence/2026-11-04-resident-service/README.md)
records an implemented owner of one original backend and two stored canonical
launch/stop operations. Canceling a driver wait preserves custody; caught
operation panic permanently quarantines it. Weak install-once parent registration
fixes the reproduced permanent 4 KiB charge while preserving scoped ownership
and refusal after expiry. Three private driver-policy tests and one actual
canonical two-slot unused-preflight/global-drain case pass on the final source.
Both release builds and Clippy `-D clippy::all` exit zero; six acknowledged
unconnected endpoint/disk warnings remain. Nine selected source inputs and two
protected binaries match. These tests boot no guest and supply no synthetic
native receipt. The earlier HOLD, panic RED and lint failure remain recorded.
The public graph's serde_json version/features remain separate from the active
RawValue/bootstrap model; its full composed qualification is still required.

The successor [endpoint-owner packet](https://github.com/R4-Laboratories/SpiralCore/blob/24e6c96df30a84ecb486e788a3f2b70fbfbe5412/docs/network/evidence/2026-11-04-endpoint-owner/README.md)
implements one-shot canonical partition issuance from the actual original guest,
endpoint memory/worker reservation and disposable store. The same guard remains
owned across handoff; uncertain Drop retains credit, and resident cleanup keeps
the sealed endpoint receipt outside that guard. It preserves the default 4 KiB
registry footprint. Three endpoint, four resident and four retirement-policy
entries pass on the final source; strict Clippy passes and seven selected source
inputs/protected binary match before and after. A real foreign-supervisor
regression and a caught synchronous cleanup-unwind regression were reproduced
before their fixes. Both original failures and the independent HOLD/successor
review remain recorded in 5,567 bytes of compressed official receipts.
The profile has no production activation constructor yet. Actual positive
sealed endpoint retirement, failed-disk-barrier retry, complete managed helper
model and native installed execution remain open. These tests boot no guest,
transfer no object and supply no new CPU, RSS, latency or WAN result.

The [parent/helper successor](https://github.com/R4-Laboratories/SpiralCore/blob/0bd0f4d0fc8200467a7164c7d6564adeadf27595/docs/network/evidence/2026-11-04-parent-host-agent/README.md)
retains one actual helper and original installation holder under the SAME parent
reservation. It separately pins MVM's canonical shortened broker directory;
the prior state-only check accepted a replaced broker parent, reproduced in a
real filesystem countercase before the fix. Fifteen parent checks pass, with
one real-helper case normally ignored and then explicitly executed separately.
That case proves daemon/signer startup, shutdown after admission closes,
idempotence, retained credits through owner Drop while a registrar alias exists,
and global finish only after the final alias drops. No VM or registration is
issued. Six source/manifest/lock blobs match the published tree, all 97 receipt
members verify, and scoped Clippy passes with 18 acknowledged unwired-code
warnings. Independent source review preserves the original HOLD and successor
PASS. Shipping profile, actual opaque-plan/config/anchor hook, per-VM registration
and resident/native integration remain open.

The [config-admission successor](https://github.com/R4-Laboratories/SpiralCore/blob/56db5291bf53433340eaf7102041370ee48aa7cc/docs/network/evidence/2026-11-04-parent-config/README.md)
binds actual opaque admission to the SAME original config/VM anchor and reserves
registration facts before one atomic lifecycle commit. An unsupported preflight
first consumed admission in an actual regression; the fix leaves it available
for a corrected config. Four new cases and 15 existing parent checks pass on the
final public Git dependency graph; one real-helper case is ignored by this run.
Strict Clippy exits zero with 13 acknowledged unconnected Rust warnings, all 131
supplied inputs match the code candidate, and the SDK tree matches its commit.
The earlier mixed Core/Contract graph and extraction/compiler failures remain
recorded. The 854,025-byte packet retains 83 hashed members and its index. These
checks boot no VM, register no VM and transfer no object; source/model/native
activation gates remain separate.

A native macOS probe of the newer installed audit capability found that its
process-exit event descriptor cannot cross the private descriptor-transfer path;
a separate truncated-ancillary case also exposed a receiver failure. Both are
recorded in [the platform checkpoint](https://github.com/R4-Laboratories/SpiralCore/issues/46#issuecomment-6639662172).
An early refusal and native lifetime-pipe/checked-receiver implementation are
active work. Linux component success does not qualify that new macOS mapping;
the macOS host role remains a required final delivery gate.

The current provisional two-endpoint collector allowance plus helper already
exceeds the existing 256 MiB tenant ceiling before carrier/core and other terms.
This is an admission/model gate, not a measured capacity result. The
[capacity issue](https://github.com/R4-Laboratories/SpiralCore/issues/49)
records the arithmetic; constructor measurements alone do not qualify arbitrary
PII search storage. No private per-process allocation or hidden accounting bucket
may bypass the shared host/tenant budgets.

None of these checkpoints qualifies the full combined service, WAN performance
or formal upstream acceptance. Branches may advance; immutable commits identify
each recorded source.

The [reader-cleanup successor](https://github.com/R4-Laboratories/SpiralCore/blob/6c885fe5a1a03d65a6f9093edc407c98feff31be/docs/network/evidence/2026-11-03-flowmux/validation/reader-completion/README.md)
retains the original reader handle across deadline expiry, preserves its semantic
result and shares completion custody across mapped-stream aliases. It also fixes
a terminal-close race reproduced in 9/10 original and 9/10 reader-only trials.
Only a next-frame EOF after the authenticated application-completion barrier can
complete; missing FIN/publication and fatal cancellation remain failures. Final
protected execution passes 39 entries across six objects, plus 10/10 repetitions
of the original positive fixture; scoped strict Clippy passes. All 384 executed
source inputs match committed blobs. The 21,206-byte archive preserves 182 hashed
receipts and its index. Independent Code/Rust/Security review passes this exact
component. A separate installed setup test originally failed when TLS stalled
after ClientHello; its original deadline/child-reap failure is retained.

The [TLS startup successor](https://github.com/R4-Laboratories/SpiralCore/blob/1025d87d048b15cefd483b4c11c70a6e029cb29c/docs/network/evidence/2026-11-03-flowmux/validation/handshake-startup/README.md)
fixes physical EOF during authentication without changing TLS parsing or
authentication. Both socket roles first failed with timeout; final qualification
passes 43 entries, ten terminal-close repetitions and eight scoped strict Clippy
targets. All 385 executed source inputs match published Git blobs. Its 9,008-byte
archive retains 73 hashed receipts and the original index, with independent
Code/Rust/Security review. The newer installed consumer separately passes 16
focused entries for setup cancellation, reader/relay custody and close/poison
races; full resource integration and native qualification remain separate gates.
These results do not boot a VM or establish a WAN speedup.

The [requested-allocation witness](evidence/2026-11-04-owned-pii-requested/README.md)
executes all 325 nonempty ordered selections of the five canonical redaction
rules, comparing exact legacy outputs and returning every selection to its
requested-allocation baseline after drop. Its maximum observed constructor peak
is 2,254,054 bytes, above the earlier default-policy sample. Search input,
whole-process RSS and full installed budget acceptance remain explicitly open.
The compact source/results archive is 163,638 bytes; heavy binaries stay remote.

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

The user added [security/FIPS qualification, #83](https://github.com/R4-Laboratories/SpiralCore/issues/83),
to the MVM submission goal on 2026-11-04. FIPS 140-3 validates cryptographic
modules; exact module/version/build, operating environment, approved services,
mode and security-policy conditions must match the deployment. Transport and
canonical mailbox/access cryptography both require inventory and qualification;
changing only outer TLS cannot establish the inner cryptographic boundary.
The current source uses ring-backed transport and independent Rust crypto paths
without an established certificate mapping. No current FIPS-compliance claim is
made. Applicable CUI/contract requirements and customer authorization extend
beyond cryptographic validation. See [NIST FIPS 140-3](https://csrc.nist.gov/pubs/fips/140-3/final)
and the [CMVP register](https://csrc.nist.gov/Projects/cryptographic-module-validation-program/validated-modules).

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
