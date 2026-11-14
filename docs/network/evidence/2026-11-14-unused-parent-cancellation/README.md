# Original-parent cancellation after unused journal startup

The bounded Linux host test passes the **unused-preparation recovery** contract.
The actual SDK refused a missing destination definition, returned its original
unused preparation, and the original parent cancelled exactly its guest
reservation. No workload was dispatched. The paired command correctly exited 1
with its original application refusal; this is not a successful transfer.

## Source and executable identity

The executed SpiralCore source is `2f26e691`, with contributor MVM
[`0909b7ec11942abbc48c47229e27533ba170bbcb`](https://github.com/Casey-allard/mvm/commit/0909b7ec11942abbc48c47229e27533ba170bbcb).
[Changed Rust file hashes](source-files.json) match the files staged on Wick for
that execution. Later feature-combination assertion corrections affect tests
only. [Frozen executable identities](executables.json) identify the actual
controller, installer and original helper binaries. Each was copied to a fresh
inode with mode 0755; older qualification binaries were preserved.

The explicit Cargo configuration uses one path-patched MVM source family and the
SpiralCore transfer/FlowMux family from `4ea98b9e`. This is a contributor
qualification graph, not qualification of a new default product lock or the held
runtime-producer PR. The existing signed rc.3 assets and two immutable role
packages passed canonical installer verification. They were not modified, and
no guest executed them. This campaign establishes no newer signed-runtime
compatibility.

## Observed lifecycle

| Stage | Guest reservations | Guest memory charge | Workers | Active reservations |
|---|---:|---:|---:|---:|
| Original service initialized | 0 | 0 | 10 | 4 |
| Actual SDK refusal, original parent retained | 1 | 256 MiB | 11 | 6 |
| Same parent's unused cancellation completed | 0 | 0 | 10 | 5 |
| Original service shutdown completed | 0 | 0 | 0 | 4 |

Premature service close refused at its ownership guard without changing usage
or reservation count. Repeated cancellation refused at its state guard without
changing credit. Normally retired empty slots cannot satisfy the separate
never-dispatched shutdown condition.

The service finished fenced with no guest or worker charges. Portable
preparation debt remains charged: 1,075,318,784 metadata bytes, 1,073,745,920
staging bytes and 8,192 cache bytes. These are accounting reservations, not
measured resident memory or disk consumption. Cancelling unused guest ownership
does not authorize erasing portable debt, protected definitions or retained
state. The final inventory still contains A's original definition; B's definition
was never created. No generic reaper or forced cleanup supplied positive proof.

## Measurements and limits

| Observation | Recorded value |
|---|---:|
| Controller internal wall time | 1.234658506 s |
| Controller process CPU | 0.646449014 s |
| GNU time command wall time | 1.24 s |
| GNU time user / system CPU | 0.59 s / 0.06 s |
| GNU time maximum RSS | 42,676 KiB |
| Installation ledger peak resident reservation | 1,480,237,056 bytes |
| Finite filesystem configured size | 1,073,741,824 bytes |
| Filesystem used after test | 223,588,352 bytes |

The controller CPU counter excludes other processes. GNU time describes its
command accounting boundary; neither is a complete host energy measurement.
Setup kernel acquisition and package installation precede the controller timer.
No application transfer ran: wire bytes, fidelity comparison and WAN latency are
unmeasured, not zero-byte or speedup results. No helper exit status is invented
for a workload that never started.

The test ran inside a private finite ext4 mount namespace, with controller
address space capped at 1 GiB, tenant process address space at 192 MiB and host
service scope capped at 3 GiB and two CPU equivalents. There were **zero booted
VMs**. The Firecracker request names the selected host backend; it is not an
observed live guest backend witness.

[Original structured result](host-result.json) · [Qualification and raw-log
identities](qualification.json). Raw logs and the finite image remain on Wick;
only compact reports and source are copied to the Mac.

## Verification and retained failures

The clean collector feature passed 27 tests and strict Clippy. Separate ordinal,
object-pair and guest-library runs passed 25, 26 and 3 tests. The first CLI
regression failed before implementation. Intermediate runs caught a missing
ordinal module import, absent sparse-checkout fixture links and a missing
historical Git revision in the offline Cargo cache. The fixture links were
restored and the public revision fetched; the successful guest tests still use
the explicit MVM source patch family.

An additional all-feature run caught two assertions expecting the clean-profile
error even when receipt-loss qualification is enabled and refuses earlier.
Those tests now assert the correct earlier refusal under that combination;
no production guard was relaxed. The final all-feature driver run passes **31 tests** and strict Clippy; guest
library strict Clippy also passes. Cargo emitted unused patch-configuration
notices; there were no Rust Clippy warnings. These follow-up changes affect only
test assertions; the host execution retains its original source identity.

The attempted default `--locked` check refused the staged path-patched fixture
lock. It does not qualify the default product graph, and no dependency manifest
or lock file changes in this PR. The existing held producer and platform gates
remain held. The successful runs above all use the explicitly recorded
contributor graph; they must not be reported as default-release acceptance.

This closes the bounded original-caller unused-credit recovery witness, not
uncertain creation, detached restart recovery, retained-state expiry, fleet
capacity or canonical WAN acceptance. A compatible signed runtime remains a
release dependency: the operator confirmed that Ari must publish it because no
contributor signing process exists. The unsigned archive is a build witness
only. Fresh admitted two-guest delivery/recovery must follow that publication.
