# Canonical first-use storage and startup recovery

The canonical local storage path is now wired into the Rust CLI and SDK. The
actual CLI regression failed before this hook and passes after it. The original
four host-agent restart scenarios now reach their intended setup and finish
**two passing, two failing** after the fixture uses canonical fresh provisioning.
The complete installed mailbox service remains unfinished.

The reviewed development implementation is public at
[`98ec9d270f5f88606b49677c266fd08f26e97369`](https://github.com/Casey-allard/mvm/commit/98ec9d270f5f88606b49677c266fd08f26e97369),
tree `324297c3c0b4311cdd06c25b90d447a8fe6a5ba7`. Its separately reviewable slices are:

- [Core publication callbacks](https://github.com/Casey-allard/mvm/commit/de9a956dbc4d7241044d15a9d7041276860dc9a0):
  retain one original FileGate through Preparing checks, durable Active write,
  exact-record/deadline revalidation and the publication callback. Partial
  publication failure or panic attempts same-generation Draining/unresolved
  fencing; a fencing error is preserved with the original error.
- [SDK preparation](https://github.com/Casey-allard/mvm/commit/5dbae1c366e658ab958f608d39bf022047c87229):
  prepare before four local writable entrypoints while keeping construction pure.
- [CLI preparation](https://github.com/Casey-allard/mvm/commit/98ec9d270f5f88606b49677c266fd08f26e97369):
  prepare after argument/environment selection and before ordinary global writers.

The CLI and SDK use the existing canonical `local` tenant. An audit tenant label
cannot select resource provisioning. Fresh creation is exclusive and private;
existing homes remain unverified without implicit owner initialization or repair.
The original preparation handle stays owned across the command or operation.
This filesystem action grants neither service readiness nor guest access.

## Executed results

| Scope | Result | Evidence |
|---|---|---|
| Core reader/coordinator/private lifecycle | 5 + 9 + 9 passed; scoped strict checks passed | [Receipt](core/qualification.json), [public output](core/public-green.txt), [private output](core/private-green.txt) |
| SDK launch module | 56 passed; focused case is included in those 56 | [Receipt](sdk/qualification.json), [output](sdk/launch.txt) |
| Actual CLI first-use command | RED: command succeeded but canonical owner open failed; GREEN: Clean epoch 0 and no foreign audit-label namespace | [Receipt](cli/qualification.json), [RED](cli/red.test.txt), [GREEN](cli/green.normal_cli_writers_prepare_only_the_canonical_local_home.txt) |
| Existing caller-identity show/help | Both passed on the same CLI executable | [Show](cli/green.caller_identity_show_does_not_create_state.txt), [help](cli/green.caller_identity_help_exposes_explicit_enrollment_and_read_only_show.txt) |
| Controlled startup | Unprovisioned input refused; canonically provisioned absent home reached Ready and collected its original signer before exit 42 | [Receipt](restart/qualification.json), [output](restart/controlled-startup-attempt1.txt) |
| Original idle-collection assertion | Passed in 1.03 s; zero fixture-adopted reaping | [Output](restart/original-idle-attempt1.txt) |
| Original four restart scenarios | 2 passed, 2 failed; no ignored or filtered cases | [Raw output](restart/provisioned-restart4-attempt1.txt) |
| Exact immutable published commit | Both required gated compile commands passed; all 4,478 source hashes unchanged | [Receipt](gated/qualification.json), [independent Git comparison](gated/source-verification.json) |

The controlled startup negative reached the actual refusal for missing canonical
provisioning: worker exit 1, zero readiness and zero signer collections. The fresh
positive returned 42 with one readiness, one original collection and zero adopted
reaps. Neither profile seeded a physical empty journal. This does not establish
safe interpretation of missing inventory for an already used installation.

The passing restart scenarios are idle after last deregistration and wrapper
restart with journal/chain recovery. Worker restart and crash mid-flight still
fail after the daemon exits during recovery. The four together ran in 2.46 s.
Their original scenario bodies, assertions, process interactions and 180/120-second
guards are unchanged. Only the common fixture first creates an absent child home
through canonical provisioning before key/config writers. The exact qualified
[restart fixture](restart/host_agent_restart.rs) and
[startup/idle probe](restart/host_agent_idle_collection.rs) are included; these
fixture revisions are not part of published commit `98ec9d27`.

The next source-visible obstruction is replay through normal Register while the
new generation is Preparing: normal Register requires Active. Stale-socket and
resource-exclusion recovery remain separate required boundaries. The original
null-stderr launcher does not establish which obstruction was the earliest runtime
failure, so the two failures are not attributed to a single proven runtime cause.

## Failed attempts remain visible

- Actual Core missing-API failures preceded implementation: [public](core/public-api-red.txt)
  and [private](core/private-api-red.txt).
- The SDK [baseline regression](sdk/baseline-red.txt) failed. An intermediate
  full launch run had [30 passing and 26 failing cases](sdk/intermediate-launch-failed.txt)
  because the new private-home guard refused the existing 0755 test fixture.
  Making the fixture private before TestEnv, while retaining its assertions,
  produced the 56-case pass. The guard was not relaxed.
- SDK strict checking was [attempted and blocked by nine Hostd dependency errors](sdk/strict-blocked.txt).
  Core strict success is not a workspace or SDK strict pass.
- The first exact-gate attempt stopped before compilation because cargo-zigbuild
  was absent from that command's PATH. Its [exit-127 output](gated/attempt1-linux-all-targets.txt)
  and [receipt](gated/attempt1-qualification.json) remain. The existing installed
  tool directories were located and the same immutable source passed both commands.
  No source correction or new tool installation was used.

## Source and measurement boundaries

Each runtime used a declared development composition. The
[complete source projection](source-projection.json) compares every tracked path
against the published commit, rather than equating a matching component with
whole-commit runtime acceptance:

| Runtime composition | Regular manifest entries | Tracked differences | Extra metadata entries |
|---|---:|---:|---:|
| Core callbacks | 4,489 | 11 | 11 |
| SDK preparation | 4,489 | 11 | 11 |
| CLI preparation | 4,489 | 8 | 11 |
| Provisioned restart fixtures | 4,489 | 13 | 11 |

The selected two Core, three SDK and three CLI blobs match the published source.
The runtime maps include the older mailbox API/crypto-test pair and historical
metadata/documentation differences. No mailbox transfer was executed by these
scopes. AppleDouble metadata entries are declared, not compiled Rust modules.
Source manifests are losslessly compressed; checksums refer to original raw data.
The CLI's RED source map is reconstructible from its GREEN map and
[three-path delta](cli/red-source-delta.json).

The exact immutable commit was separately exported. Every one of its 4,478
regular file hashes matched an independent Git-archive comparison and remained
unchanged through both gated commands. That is type-check qualification, with
genuine Hostd unused-code warnings retained, rather than whole-commit runtime or
strict acceptance. The [raw gate receipt](gated/qualification.raw.json) omitted
the tree field; the published receipt supplements it with the independently
verified Git tree and retains the raw receipt's hash.

Runtime checks used Linux x86_64, Rust 1.97.1, release/locked/offline, two jobs,
nice 10, incremental disabled, umask 022 and the existing serialized target.
The actual CLI was unembedded with default features disabled; its ordinary
completion command and caller-identity paths require no VM. Gated checks used
the retained required commands with the documented shared-target override and
existing Zig/cargo-zigbuild tools. No Mac build or VM boot was used.

These are functional control/storage tests. Their test durations and rounded
command/build CPU counters are not sender/receiver transfer CPU, data-transfer
speed, energy, fidelity/compression benchmarks or a supported capacity envelope.
No gate for native MVM retirement, WAN/platform pairs, complete sustained recovery,
FIPS/CUI or production deployment is closed by this packet. Recovery work remains
tracked in [#114](https://github.com/R4-Laboratories/SpiralCore/issues/114) and
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).

[The manifest](evidence.sha256) covers this packet. Included MVM fixtures and
source excerpts retain the [MVM Apache-2.0 license](../2026-11-11-cooperative-idle/MVM-LICENSE).
Original SpiralCore documentation retains its repository license.
