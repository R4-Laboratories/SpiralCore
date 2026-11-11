# MVM resource-owner prerequisite

The existing MIT SpiralCore installation-owner component is now imported into
MVM's public development branch at
[`c009524ea1da35db52ea909084220b10c36aafd2`](https://github.com/Casey-allard/mvm/commit/c009524ea1da35db52ea909084220b10c36aafd2).
This is a tested resource-lifetime prerequisite, not the installed mailbox
service or a completed upstream submission.

The original owner holds an exclusive owner lock. Participating resources hold
shared leases for an exact installation, profile and epoch. Recovery requires a
fresh, separately opened exclusive holder lock. A live holder prevents retirement;
owner death, PID absence and a timeout cannot replace that check. Drop does not
publish a clean epoch. Stable descriptor identities and the existing private,
bounded durable state publication remain intact.

Resource retirement is separate from collecting an original Child exit status.
These locks grant no permission to start a VM, access a mailbox or sign a receipt.
They establish exclusion only for resources actually retaining a lease. Legacy
resources cannot be declared recovered from an empty new lock namespace.

## Executed component qualification

Linux x86_64 on the authorized Wick host; Rust 1.97.1; release/locked; two jobs;
incremental disabled; explicit umask022; serial subprocess fixtures. No VM was
booted and no payload was transferred.

| Scope | Executed result |
|---|---|
| Initial integration target before the import | Intended E0432 missing-module RED; zero runtime tests |
| Imported integration suite | 11 passed: ten behavior scenarios and one subprocess fixture; zero failed/ignored |
| Imported private units | Six passed; zero failed/ignored; 1,066 unrelated tests filtered |
| Strict VMM library/integration Clippy | Passed |
| Format check of the imported module and tests | Passed |
| Independent code/Rust and security source review | Passed on the imported component |
| Seven component/configuration files, before/after | Identical hashes |

The behavior scenarios include an owner dying while another process retains a
lease, retained Active state until the last holder exits, exact-descriptor
handoff lifetime, stale epochs, malformed state, storage replacement and failed
durability barriers. One scenario can exercise multiple faults; counts are test
entries, not a fleet or platform matrix.

The imported production file and both private test files are byte-identical to
SpiralCore main `179fb1eba84e1056b86ac7027de5dc777138c512`. The public integration
suite changes only its library import path. MVM uses its already locked
`sha2 0.11.0` dependency; the original used `sha2 0.10`. The fixed 176-byte record
and checksum domain are unchanged. The complete MIT notice and file provenance
are preserved in MVM's `third_party/spiralcore-installed-owner` directory.

The qualification checkout also contained the additive, uncommitted Core
lifecycle-reader prerequisite listed in [qualification.json](qualification.json).
The seven imported component/configuration files match the published commit;
this is not a claim that the entire unmodified commit or workspace was tested.
Original outputs and source manifests are retained on Wick; this packet records
their digests without duplicating build trees or VM images locally.

## Integration still required

The host-agent must retain its coordinator, signer and all three watchers, and
each actual signer/helper/cleanup process must hold its resource lease across
parent failure. Canonical admission, original-child collection and strict
pending/session reconciliation remain distinct checks. New Active state requires
actual signer/listener readiness. Missing legacy provisioning must refuse rather
than silently initialize a clean resource namespace.

The lifecycle storage gate, typed subprocess handoff and finite watcher-job owner
are development successors. Their initial scoped tests do not establish an
installed cooperative shutdown. The reproduced idle path still needs to be
replaced: process exit42 bypasses the retained signer owner. See
[#114](https://github.com/R4-Laboratories/SpiralCore/issues/114),
[#111](https://github.com/R4-Laboratories/SpiralCore/issues/111), and the
[earlier canonical native result](../2026-11-11-canonical-native-sdk/README.md),
which retains the failed helper-collection gate.

The shared upstream interface is [MVM #5328](https://github.com/tinylabscom/mvm/pull/5328);
the independent SDK fix is [MVM #5336](https://github.com/tinylabscom/mvm/pull/5336).
The [upstream implementation/ownership index](https://github.com/tinylabscom/mvm/issues/4629#issuecomment-6794294611)
links the existing code and requests the exact private journal/guest-reader
source before changes to overlapping service/launch/process files.

Native macOS, installed Linux/Firecracker retirement, complete workspace/features,
supported platform roles, WAN, sustained capacity and the concrete FIPS/security
profile remain separate open acceptance gates. This component records no
wire-byte savings, transfer CPU/time or energy result.
