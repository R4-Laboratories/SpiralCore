# Original-owner startup and bounded publication

The installed-service candidate now refuses a missing registration journal after
an actual prior resource epoch, while a genuinely fresh installation still starts
and collects its original signer before idle exit. A separate process regression
also fixes direct worker startup with no configured home. These are executed
Linux startup/control results; the complete installed MVM service is unfinished.

The supporting Core/VMM changes are public at
[`081901d0285ccf00d9dd824746354e9f82d8b6ce`](https://github.com/Casey-allard/mvm/commit/081901d0285ccf00d9dd824746354e9f82d8b6ce),
tree `0aa3bdd86ca0e1ebcaf8778c114f91ece00fec51`. The Hostd startup,
nonblocking-publication and strict-home changes executed in a separately pinned
development composition. Their supplied source artifacts are for inspection and
reproduction, not an enabled production release or the final upstream PR.

## Executed checks

| Scope | Observed result | Evidence |
|---|---|---|
| Actual missing-journal regression, before repair | Prior issued/finished epoch 1 incorrectly reached Ready and idle exit 42 | [Original failure](missing-journal-red/runtime-attempt1.txt), [receipt](missing-journal-red/qualification.json) |
| Startup control, after repair | Unprovisioned input refused; canonical fresh input started; prior-epoch missing journal refused before signer readiness | [Actual process output](startup/controlled_startup_distinguishes_missing_from_canonical_provisioning.txt), [receipt](startup/qualification.json) |
| Original idle process | Exit 42 after one original signer collection; zero fixture-adopted reaps; 1.03 s | [Output](startup/idle_42_follows_original_signer_collection.txt) |
| Actual latch contention | Old body failed its 5-second deadline; repaired body refuses Busy before dispatch | [Original failure](publication/old-latch-runtime-red.txt), [successor](publication/green-latch.txt) |
| Publication/custody/replay composition | 21 distinct selected outer cases pass: 3 primitives, 2 real-latch, 11 cell, 4 token, 1 two-record Preparing replay | [Receipt](publication/qualification.json), [two-record replay](publication/green-startup.txt) |
| No-home worker regression | Old binary gave the wrong storage error; strict resolver gives the canonical refusal with no Ready or signer collection and unchanged owned fixture | [Failure](strict-red/worker_without_home_refuses_before_owned_startup_effects.txt), [successor](strict-green/worker_without_home_refuses_before_owned_startup_effects.txt) |
| Strict-home composition controls | New negative plus unchanged three-profile startup and original idle cases all pass | [Receipt](strict-green/qualification.json), [startup](strict-green/controlled_startup_distinguishes_missing_from_canonical_provisioning.txt), [idle](strict-green/idle_42_follows_original_signer_collection.txt) |
| Preparing FileGate turn | 25 selected reader/coordinator/private cases and scoped strict Clippy pass on the original Preparing slice | [Receipt](preparing/qualification.json), [public](preparing/public.txt), [private](preparing/private.txt), [strict](preparing/clippy.txt) |
| Resource epoch accessor successor | 17 public reader/coordinator cases and scoped strict Clippy pass; earlier private run is not relabeled as this successor | [Output](resource-epoch/public.txt), [strict](resource-epoch/clippy.txt), [API failure](resource-epoch/api-red.txt) |
| Original recovery predecessor | 21 selected owner/exclusion/private cases and scoped strict Clippy pass | [Receipt](recovery-predecessor/qualification.json), [public](recovery-predecessor/public.txt), [private](recovery-predecessor/private.txt) |
| Exact immutable public commit | Both required gated compile commands pass; 4,478 hashes match independent Git export and remain unchanged | [Receipt](gated/qualification.json), [Linux all targets](gated/linux-all-targets.txt), [BDD all targets](gated/bdd-all-targets.txt) |

Counts describe their individual source scopes and must not be added into one
test-suite total. Isolated subprocess repeats are not extra distinct cases.
The three-profile startup and idle cases were repeated after the strict-home fix,
not treated as new behaviors. No scenario seeds an empty registration journal to
make a used installation appear fresh.

## What changed

The resource recovery guard captures the original predecessor while the actual
owner and holder exclusions are retained. A copied snapshot supplies information,
not cleanup or bootstrap authority. Fresh journal creation requires the same
actual guard, original Clean epoch zero, absent lifecycle state/lock/intent and a
checked empty owned-session inventory. Creation is exclusive and private, with
file/directory synchronization and exact readback. A missing or malformed journal
on a used installation is not replaced with empty state.

Preparing replay reconstructs both original services behind their original
parked latches. It retains the full registration record set and original accepted
work through preparation, then checks the complete map before Active publication.
The immutable operation descriptor captures original configuration and ownership
facts; actual journal observation occurs outside daemon and FileGate locks.

Assurance publication reserves the original writer with a nonblocking attempt
before signaling the original latch. Busy or poison refuses without dispatch;
receiver loss drops the reservation without publication. Successful commit uses
that same reservation. Dispatch and publication are separate steps, and readiness
remains a later caller gate. The primitive does not establish that every error
path or complete FileGate callback is allocation-free.

Direct worker startup now calls the existing strict home resolver. The product
correction is one line; the appended process test preserves the previous 18,412
test bytes exactly. With both home variables removed only from the child command,
the worker refuses before resource descriptor/owner operations. Its earlier
configured verifying-key read still occurs; this is not a zero-readonly-I/O claim.
The test never probes, creates or removes the global fallback directory.

## Source identity and reproduction

The [projection](source-projection.json) lists every difference between public
commit `081901d02` and the final strict-home runtime: 14 changed tracked files,
14 extra entries and no missing tracked paths. All 4,492 runtime entries are
pinned in the supplied source maps. Some extra entries are historical AppleDouble
metadata, not compiled modules. The older mailbox API/crypto pair in this runtime
is explicit; these process checks do not execute mailbox transfer.

Each changed/extra source is supplied as `composition/<original path>.gz`, with
the MVM Apache 2.0 [license](composition/LICENSE). To reconstruct this candidate,
start with that exact public commit and decompress those paths into the matching
relative locations. Preserve ordinary bytes for AppleDouble metadata instead of
asking macOS tar to interpret them as filesystem attributes. Original agent and
test sources for the earlier process composition remain in `strict-red`; the old
latch body remains in `publication`. The source and executable before/after maps
bind each run separately, rather than claiming the latest candidate executed an
older regression or a whole committed release.

The gate source maps match the independently generated Git archive exactly.
Gated compilation is separate from the runtime composition. Genuine unused-code
warnings remain visible in those logs; passing gated checks is not strict
workspace acceptance. The earlier scoped strict successes retain their own source
identities. Compressed artifacts preserve their original decoded SHA-256 in
[artifact-bindings.json](artifact-bindings.json).

All commands ran on Wick Linux x86_64 with Rust 1.97.1, two jobs, nice 10,
incremental disabled and the existing serialized target. The test builds used
release/locked/offline; immutable gated compilation used its recorded original
commands and dev profile. The final strict-home build took 27.80 s;
actual no-home runtime was approximately
0.0023 s, and startup/idle tests approximately 1.035/1.033 s. These are functional
test durations. Runtime CPU/RSS was not collected, and no transfer-performance,
send-only, energy or capacity comparison follows from build CPU counters.

## Remaining service gates

Interrupted mutation recovery still needs the real `ReconciledStopped` consumer:
retain and join the original owners, finish the same cached I/O, compensate and
restore the original canonical record set, then allow a current authorized
Deregister to perform its own removal and journal commit. Recovery must not turn
old metadata into a fresh grant or report success for an unfinished new request.

Historical socket ownership also remains unresolved. Current inode, UID, path or
a registration DTO is not a durable original-bind receipt. Canonical short-path
and signed custom endpoints both require the original producer/recovery
transaction; this packet does not qualify the candidate socket-retirement helper.
The original four restart checks have not been rerun on this composition. Their
previous two-pass/two-fail result remains retained in the
[earlier report](../2026-11-11-frontend-recovery/README.md).

Actual lifecycle-bound assurance activation, installed native retirement,
sustained/capacity acceptance, final Linux/macOS/Windows-role WAN qualification
and FIPS/security qualification remain open in the GitHub roadmap. No VM boots,
WAN transfers or compression measurements were performed by this packet.
