# Installed workers, native transfers and recovery

This is an intermediate implementation for the [Ari delivery program](../plans/2026-11-03-ari-delivery.md),
tracked in [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43) and
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).
The installed entry now retains its original installation owner, worker
children and native lifetime holders. Actual Linux supervisor/worker death
and positive next-epoch recovery have executed. Native macOS/HVF transfers
also execute through that entry. The canonical resident launcher, aggregate
plaintext/cache/workspace ownership, native HVF death recovery and canonical
two-host WAN acceptance remain unfinished.

The [complete evidence packet](evidence/2026-11-03-installed-supervisor/README.md)
retains original JSON, executed drivers, source/binary identities, failures,
review receipts and storage custody. These results do not qualify the whole
networking contribution or establish customer fleet capacity.

## Native transfer results

Each row transfers the same eight target objects through two actual guests,
then checks exact reopened bytes and lengths using independent verifier boots.
At most two guests run concurrently, each with one vCPU and 1 GiB of RAM and
no guest NIC. Combined also transfers its checkpoint and schema; those setup
objects, requests, acknowledgments and readiness framing are counted.

The nominal 1 MiB tier has eight **1,047,202-byte** targets. The nominal 16 MiB
tier has eight **14,851,384-byte** targets. Requested tiers are corpus-generation
parameters, not a claim that every target has exactly that many bytes.

| Native backend / tier | Encoding | Both-direction canonical bytes | Two-hop vsock stream bytes | Sender guest CPU | Receiver guest CPU | Lifecycle wall |
|---|---|---:|---:|---:|---:|---:|
| Linux/Firecracker, 16 MiB | Whole-object zstd | 107,301,660 | 214,603,320 | 4.488958 s | 3.707541 s | 8.814947 s |
| Linux/Firecracker, 16 MiB | Combined | 62,491,548 | 124,983,096 | 2.963787 s | 2.519790 s | 6.392388 s |
| macOS/HVF, 1 MiB | Whole-object zstd | 7,345,570 | 14,691,140 | 1.250831 s | 0.978973 s | 2.971770 s |
| macOS/HVF, 1 MiB | Combined | 4,933,266 | 9,866,532 | 0.815613 s | 0.761566 s | 2.113634 s |
| macOS/HVF, 16 MiB | Whole-object zstd | 107,276,257 | 214,552,514 | 15.335437 s | 11.293480 s | 30.038789 s |
| macOS/HVF, 16 MiB | Combined | 62,546,739 | 125,093,478 | 9.181874 s | 5.281349 s | 16.368091 s |

All six rows report exact reopened results. The larger Linux pair uses 41.76%
fewer canonical bytes, 33.10% less combined guest endpoint CPU and 27.48% less
lifecycle time with Combined. The larger macOS pair uses about 41.70% fewer
bytes, with 14.463223 s of combined guest endpoint CPU versus 26.628917 s and
16.368091 s of lifecycle versus 30.038789 s.

These are single fixed-order local native pairs on shared hosts, not randomized
causal performance campaigns or WAN results. Linux and macOS use different
native implementations and enforcement paths; their timings must not be used
as a matched operating-system comparison. The canonical byte counter includes
encrypted envelopes and four-byte frame prefixes/readiness in both directions.
The two-hop counter observes the duplicated guest-to-host and host-to-guest
streams; it excludes virtio control/credit headers and is not a NIC capture.
Sender/receiver CPU includes codec work and endpoint handling. Host worker/VMM
CPU is retained separately and overlaps nested guest activity; adding it to
guest CPU would double count. No send-only time or energy is inferred.

The earlier Linux 1 MiB pair remains in the packet: Combined used 4,922,739
versus 7,353,966 canonical bytes and 567,360 versus 645,614 microseconds of
combined guest CPU, but lifecycle was **0.685% slower** (1,245,464 versus
1,236,986 microseconds). Its sender worker CPU was also higher. In the larger
Linux pair sender worker CPU is 186,281 versus 184,255 microseconds, another
retained loss. Configured RAM limits and serialized-buffer reservations are
not measurements of whole-process RSS.

Original records:

- [Final Linux source-pinned pair and recovery](evidence/2026-11-03-installed-supervisor/native/final-linux/summary.json).
- [Four macOS pairs and their source delta](evidence/2026-11-03-installed-supervisor/native/hvf-transfers/summary.json).
- [Earlier Linux pairs and original failures](evidence/2026-11-03-installed-supervisor/README.md).

## Lifecycle behavior

The controller issues fresh, non-serializable local grants to its own bounded
worker slots. A private inherited control channel binds the actual child to
the current installation and checks that binding before activation. Diskless
sender/verifier roles retain native lifetime holders too. Transport allocation
and VM launch permission remain distinct.

The Linux death witness captures the original processes before signaling them.
With an actual guest ready, it stops Firecracker and kills the owned supervisor
and worker. A successor sees `Busy` / `RecoveryRequired`, and the original Active
epoch remains unchanged while Firecracker survives. Retirement succeeds only
after positive Firecracker exit and the exact sudo launcher's exit; a subsequent
4096-tier transfer reopens exactly and ends Clean in generation 2. A surviving
sudo monitor genuinely retained a holder in an earlier failed attempt, which
is preserved. Signals, EOF and a disappeared persisted PID are not exit proofs.

Native macOS testing exposed a clean terminal half-close case: Darwin may return
`NotConnected` when shutting down an already closed Unix direction. The narrow
correction applies after clean EOF. Actual truncated frames, failed writes and
bounded cleanup remain fatal and have separate Linux/macOS regressions.

This development orchestrator is not yet the complete canonical installed
mailbox service. Original `AdmittedPlan` continuity, resident native owners,
typed parent partitions, independent service queues and shared plaintext/cache/
workspace reservations must reach every final launcher and endpoint. The next
implementation retains those authorities rather than reconstructing handles
from VM names or creating independent per-process quota pools.

## Platform scope

The Windows implementation uses the same protocol and object formats, with a
small isolated native storage crate that owns Windows FFI. The shared transfer
crate retains `forbid(unsafe_code)`. Its supported private-store profile checks
native identity/ACLs, original directory handles and path/reparse/hardlink rules.
The native Windows Server 2022/2025 x64 and Windows 11 ARM64 storage/component
rows pass on the reviewed candidate. Their original compile, junction-fixture
and missing-Clippy failures remain recorded separately.

Those Windows rows are component/client evidence. They do not establish native
Windows MVM hosting, Windows end-to-end gateway WAN qualification, power-loss
durability or FIPS validation. Linux/Firecracker and Apple Silicon macOS/HVF
remain the native MVM host roles in this program.

## Remaining acceptance

The live issues remain open for canonical installed ownership and aggregate
resource integration, final-ACK recovery and long-lived retention, admitted
FlowMux and owner-access mappings, native HVF death recovery, two-host direct/
relay WAN, the declared client/storage platform matrix, sustained fairness and
saturation, and the final upstream MVM package. Separate successful component
tests are recorded under their own source and are not substituted for these
integrated acceptance gates.
