# Installed-owner component: lifecycle and durable fencing

The Rust owner/epoch component is implemented in merged [PR68](https://github.com/R4-Laboratories/SpiralCore/pull/68).
It is the first prerequisite for installation-wide MVM admission under
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49). Actual installed
worker/VMM handoff and native crash/recovery are still open; existing sessions
continue to report their original orchestrator scope.

## Behavior

One owner holds an exclusive stable lock. Every participating resource holds an
independent shared lease on a second stable object. The owner publishes a
checksummed Active epoch before admission. A successor can retire that epoch
only after obtaining exclusive access to the holders object. No timeout or
recorded PID establishes exit, and Drop never publishes Clean. A delayed holder
must check the original Active epoch while shared-locked.

The fixed176-byte state and bounded publication-intent marker refuse malformed
records, replaced objects and uncertain publication. The actual opened holder
descriptor is checked before locking, closing an A→B→A namespace race. Private
parent synchronization, local-filesystem admission, ownership/mode/link checks
and macOS extended-ACL rejection bound the storage profile. These checks do not
claim protection from a trusted same-UID/root operator deliberately bypassing
the installed launcher or a physical power-loss qualification.

See the [local contract](../api/installed-host-owner-v1.md),
[implementation plan](../plans/2026-11-03-installed-host-owner.md), and
[full compact evidence](evidence/2026-11-03-installed-host-owner/README.md).

## Executed results

Product source `d4f2ed412ae961c5d2bd267caf0ec1a2628157c1`, production file
SHA-256 `b75172d56b284adf9b69417fac3d6835fd9cb2edf311b871cb518222bda1ae57`.

| Scope | Result | Limit |
|---|---|---|
| macOS27 ARM64 exact-production-file harness | 4 fault-boundary unit +10 lifecycle entries pass; strict Clippy passes | One lifecycle entry is a subprocess helper; not10 independent scenarios |
| Linux x86_64 exact-file harness, serial | 4 unit +9 lifecycle entries pass | Same helper and overlapping component tests; original parallel failure retained |
| Linux full live crate, release `--lib --tests` | 115 pass; strict Clippy over all targets passes | Includes the same4 unit tests; no VM boot |
| Linux instrumented component line coverage | 402/428 mapped production lines,93.93% | 26 uncovered; macOS cfg body unmapped; branch coverage NA |
| Hosted Linux/macOS x64/ARM64 owner matrix | All4 rows pass54 entries, strict Clippy | Includes4 helpers and overlapping local scenarios; no guest boot |
| Hosted full Linux contracts | 115 live +138 shared checks, Clippy/default/analyzers pass | Shared130 adapter +8 object-v1; scopes overlap |

The [four native Unix architecture rows](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41697837824)
and [full Linux workflow](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41697837631)
passed on reviewed head64d93fed. Artifacts identify tested merge dd8362d0;
its base/head parents are authenticated, and its full tree3d478a93 equals the
reviewed tree. All4 original ZIP/API digests and20 compiled file pins verify.
The [release decision](evidence/2026-11-03-installed-host-owner/release-decision.json)
retains exact identities and scopes. Windows client
qualification remains in its existing matrix; this Unix owner module supplies
no Windows installed MVM backend.

## Retained failures and fixes

- The initial missing-source/API RED precedes product code. Earlier invalid
  storage/error-class failures remain in the packet.
- Independent review exposed an actual holder-descriptor ABA bug and a durable
  publication gap after rename/directory-sync failure. Both have deterministic
  RED regressions followed by passing exact-source tests.
- A macOS ACL regression demonstrated five cases where private POSIX modes
  still allowed extended grants. The first guard then incorrectly rejected
  ordinary no-ACL files: Apple's descriptor API reports absent ACL properties
  with ENOENT. An owned live-descriptor probe and primary Apple Libc source
  corroborate the corrected interpretation; the five grant cases still refuse.
- The first parallel Linux run had one valid Busy refusal. A controlled trace
  confirmed that another test subprocess can temporarily inherit a locked open
  file description before exec applies CLOEXEC. That controlled mechanism does
  not directly trace the original incident. Exact and serial tests pass with
  unchanged assertions. Process qualification explicitly uses serial fixtures.
- The first new CI workflow was rejected before jobs because runner context
  was placed in job-level env. The corrected step-level env preserves the
  target path. The original failed run is retained, not counted as tests.
- The first metadata verifier incorrectly expected PR head rather than tested
  merge. macOS Bash3.2 continued after false compound conditions despite
  errexit, producing an unaccepted false-PASS receipt. Explicit failure branches
  now authenticate the tested merge/parents/tree, artifacts and source pins;
  wrong-commit and wrong-digest controls exit1. Original script, output and
  diagnosis remain retained. No product tests were rerun or reclassified.

## What follows

The [proposed Firecracker handoff](../api/firecracker-host-lease-v1.md) carries
exact directory/holder descriptors and a bounded current-state expectation
through the existing scoped sudo launcher. The actual VMM must acquire its own
shared lock and retain it across exec. Diskless sender/verifier paths and macOS
in-process HVF lifetime must be covered too. Only executed owner+worker death,
surviving-backend exclusion, backend exit and positive next-epoch admission can
qualify full B6a.

The installed entry/budget lifetime, B6b shared plaintext/cache partitions,
safe production retention beyond2,048 Calls, measured pipelining, fleet envelope
and canonical WAN/platform acceptance remain tracked work. This change modifies
no transfer encoding and establishes no new bytes/CPU/transfer-time result.
Native and WAN measurements remain in the preceding reports. Builds/coverage
ran on Wick; only the small focused target and compact records were added
locally, with no payload store or VM disk copied to the Mac.
