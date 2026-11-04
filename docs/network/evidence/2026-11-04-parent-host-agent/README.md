# Original parent / host-agent owner component

2026-11-04, Wick Linux x86_64, Rust 1.97.1. This development checkpoint adds
actual helper ownership to the original parent resource ledger. It succeeds
`24e6c96df30a84ecb486e788a3f2b70fbfbe5412` on the same locked public dependency
graph: MVM `f708539f`, shared core `ca5d9ca1`, adapter `dee6e52a`.

**The shipping helper profile is inactive.** Its complete requested-resource
model, canonical binding/consumer connection, actual per-VM registration and
native installed transfer/retirement remain open. No guest is booted by these
tests. New local MVM Core/Hostd/client API seams are separate work and are not
qualified or imported by this checkpoint.

## Implemented ownership

The private parent reserves from its SAME managed pool before copying helper
identity, preparing the original holder or spawning the canonical host agent.
One original owner retains the actual daemon/signer lifecycle; registrar clones
refer to that same owner. Weak parent/anchor references avoid a strong ownership
cycle. Uncertain startup/Drop closes admission and retains actual custody and
credits. Cleanup checks retained identity and can proceed after admission closes.

Positive helper shutdown does not release credits while a registrar alias still
holds the original shared owner. The resident consumer must retire VM operations,
stop the original helper, drop its original backend/registrar aliases and only
then attempt global finish. This consumer connection remains to be completed.

The registration-binding implementation records bounded canonical facts and
the original anchor, streams a domain-separated identity commitment without a
JSON Value tree, and pins both the actual VM state directory AND canonical broker
socket-parent directory. A digest identifies facts; it grants no service. The
actual opaque-admission/config hook remains to be connected on the composed
source graph. Private refusal/path tests are not canonical admission or native
exit receipts.

## Executed cases

| Scope | Actual result |
| --- | --- |
| Missing parent helper API | Exact metadata E0599, and separate full-library release build E0599; no tests ran |
| Missing/unqualified profile, wrong supervisor, duplicate profile and foreign tenant | Three focused policy cases pass without child/resource effects |
| Independently replaced canonical shortened broker directory | Genuine behavior RED: state-only pin accepted replacement; corrected two-directory pin passes |
| Existing endpoint/resident/retirement policy regressions | Eleven entries pass on the successor |
| Final scoped parent execution | 15 PASS, one functional case explicitly ignored, 124 unrelated filtered |
| Explicit real-helper case | That same functional case separately executes: 1 PASS, zero ignored, 139 filtered |
| Final resolved lib/tests Clippy | `-D clippy::all`, exit zero; 18 acknowledged Rust warnings for unconnected component code, not a warning-free whole service |

The real-helper case uses two freshly built canonical binaries from the SAME
MVM revision, protected before execution. It proves actual original daemon and
signer startup, shutdown after admission closes, idempotent shutdown, worker
credit remaining charged after shutdown/owner Drop while a registrar survives,
and release only after the last alias drops. The original supervisor then
finishes. It issues no VM registration, mailbox transfer or native guest launch.

The fixture declares candidate 40 MiB operation + 12 MiB retained requested
storage and 32 MiB usable Linux stacks, with two worker-process slots. This is
a functional reservation/ownership witness, not approval of that resource bound,
an OS memory limit or whole-process RSS. The profile constructor remains private
to tests; no production activation constructor is supplied.

The explicit fixture took 0.05 seconds according to `/usr/bin/time -v` and
observed 12,336 KiB maximum RSS. The command's CPU fields display 0.00 seconds
at their reporting precision; this does **not** mean zero CPU cost. These are
whole-fixture observations including setup, checks and cleanup, not transfer
time, aggregate simultaneous process memory or a matched speedup. IPC traffic
was not instrumented and no object bytes were transferred.

## Failures and custody

The first fresh-home helper attempt failed with ENOENT before spawning: it had
no canonical host signing key. Independent debugger/source/namespace inspection
confirmed that prerequisite. The fixture now calls the SAME canonical
`audit::host_keypair::load_or_init_at` used upstream, only in its fresh synthetic
home. Product startup has no raw-key or legacy-daemon fallback. The original
failure and bounded test-only diagnostic remain recorded.

Other prerequisite records remain distinct: an owned debug-profile compile was
intentionally canceled (130) before rerunning release; the first product compile
used an incorrect broker constant namespace, corrected to the actual portable
`mvm_net::channel::GuestService::Broker.port()`; and a test-only unavailable
TempDir constructor was corrected before obtaining the genuine path-behavior
RED. External-package binary selection refusals preceded the successful qualified
workspace-feature command. None is relabeled as a behavioral regression.

[Compact original receipts](receipts.tar.gz), SHA-256
`e7842626fb3d2cdac804313482753d4f91f95783ed5a9306ca48867a9998cc96`,
are 26,672 bytes: 97 hashed members plus the original JSON index. All member
hashes were checked after transfer. The packet preserves failures, commands,
actual compiler target identities, protected binary hashes, six source/manifest/
lock pins, source readbacks and acknowledged warnings. Original compiler JSON
and executables remain on Wick; keys, payloads and VM disks are not included.

[Independent Code/Rust/Security review](reviews/code-rust-security.md) preserves
the first HOLD, actual countercase and successor PASS. Review approves only
this frozen component; it does not approve native activation or the full MVM
submission. The [parent integration contract](../../../api/installed-host-agent-parent-v1.md)
and [live roadmap #42](https://github.com/R4-Laboratories/SpiralCore/issues/42)
retain canonical installed ownership, final-ACK/retention, sustained scale,
FlowMux/access, platform/WAN and security/FIPS acceptance requirements.
